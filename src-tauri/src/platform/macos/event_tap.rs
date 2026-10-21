//! Поток CGEventTap (session tap, фильтрующий): принимает реальные
//! нажатия, передаёт в движок, исполняет решения — замены (синтетическими
//! событиями) и переключения раскладки, ручные хоткеи.

use super::engine_state;
use super::ffi;
use super::typing;
use caramba_core::engine::{Action, Event};
use caramba_core::layouts::Pair;
use caramba_core::rules::masking::{try_replace, MaskDirection, MaskPairs};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// Мач-порт тапа: сырой указатель, безопасно передаваемый между потоками
/// (живёт до выхода приложения, защищён Mutex-ом).
#[derive(Clone, Copy)]
pub struct PortPtr(pub ffi::CFMachPortRef);
unsafe impl Send for PortPtr {}
unsafe impl Sync for PortPtr {}

/// Общее состояние тапа.
pub struct TapShared {
    /// Основной конвертор включён (меню/настройки).
    pub enabled: AtomicBool,
    /// Настройка autoswitch (гасит только авто-замену, хоткеи живут).
    pub autoswitch: AtomicBool,
    /// dry-run: решения печатаются в журнал, печати нет.
    pub dry_run: Mutex<bool>,
    /// Secure Input на системе (обновляет поток-наблюдатель).
    pub secure: AtomicBool,
    /// Последние флаги-модификаторы (детектор Shift/Option-tap).
    pub last_flags: Mutex<u64>,
    /// Начало удержания Shift.
    pub shift_start: Mutex<Option<Instant>>,
    /// Момент последнего одиночного Shift-tap (для Double Shift).
    pub last_tap_ms: Mutex<Option<u64>>,
    /// Начало удержания Option.
    pub option_start: Mutex<Option<Instant>>,
    /// Мач-порт тапа — включение заново из сторожевого потока.
    pub tap_port: Mutex<Option<PortPtr>>,
    /// Режим «Маскировка» включён (меню/хоткей ⌃⌥M).
    pub masking: AtomicBool,
    /// Интенсивность маскировки 10–100.
    pub masking_intensity: AtomicU8,
    /// Язык текущей раскладки: 0 — неизвестен, 1 — EN, 2 — RU.
    pub current_lang: AtomicU8,
}

/// Признак «маскировка активна в тапе» (для внешних модулей).
pub const LANG_UNKNOWN: u8 = 0;
pub const LANG_EN: u8 = 1;
pub const LANG_RU: u8 = 2;

static SHARED: OnceLock<Arc<TapShared>> = OnceLock::new();

/// Доступ к общему состоянию (после start()).
pub fn shared() -> Arc<TapShared> {
    SHARED.get().expect("тап не инициализирован").clone()
}

/// Миллисекунды от старта процесса.
pub fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// Создать общее состояние и запустить поток тапа.
pub fn start(enabled: bool, autoswitch: bool, dry_run: bool) -> Result<(), String> {
    SHARED.get_or_init(|| {
        Arc::new(TapShared {
            enabled: AtomicBool::new(enabled),
            autoswitch: AtomicBool::new(autoswitch),
            dry_run: Mutex::new(dry_run),
            secure: AtomicBool::new(false),
            last_flags: Mutex::new(0),
            shift_start: Mutex::new(None),
            last_tap_ms: Mutex::new(None),
            option_start: Mutex::new(None),
            tap_port: Mutex::new(None),
            masking: AtomicBool::new(false),
            masking_intensity: AtomicU8::new(50),
            current_lang: AtomicU8::new(LANG_UNKNOWN),
        })
    });

    std::thread::Builder::new()
        .name("caramba-tap".into())
        .spawn(move || {
            let mask = ffi::K_CG_EVENT_MASK_KEY_DOWN
                | ffi::K_CG_EVENT_MASK_KEY_UP
                | ffi::K_CG_EVENT_MASK_FLAGS_CHANGED
                | ffi::K_CG_EVENT_MASK_LEFT_MOUSE_DOWN
                | ffi::K_CG_EVENT_MASK_LEFT_MOUSE_UP;
            // SAFETY: tap_callback — реальная функция; user_info не задействован.
            let port = unsafe {
                ffi::CGEventTapCreate(
                    ffi::K_CG_SESSION_EVENT_TAP,
                    ffi::K_CG_HEAD_INSERT_EVENT_TAP,
                    ffi::K_CG_EVENT_TAP_OPTION_DEFAULT,
                    mask,
                    tap_callback,
                    std::ptr::null_mut(),
                )
            };
            if port.is_null() {
                tracing::error!(
                    "CGEventTapCreate вернул NULL: нет разрешений \
                     (Accessibility/Input Monitoring)"
                );
                return;
            }
            *shared().tap_port.lock() = Some(PortPtr(port));
            tracing::info!("CGEventTap создан");
            // SAFETY: создание источника рун-лупа для живого прослушивания.
            let source = unsafe { ffi::CFMachPortCreateRunLoopSource(std::ptr::null(), port, 0) };
            // SAFETY: оба указателя — CF-объекты, владеем портом.
            unsafe {
                ffi::CFRunLoopAddSource(
                    ffi::CFRunLoopGetCurrent(),
                    source,
                    ffi::kCFRunLoopCommonModes,
                );
                ffi::CFRunLoopRun();
            }
        })
        .map_err(|e| format!("не удалось запустить поток тапа: {e}"))?;
    Ok(())
}

/// Включить/выключить основной конвертор.
pub fn set_enabled(on: bool) {
    shared().enabled.store(on, Ordering::SeqCst);
    tracing::info!(on, "конвертор включён/выключен");
}

/// Создан ли мач-порт тапа (диагностика разрешений).
pub fn tap_is_active() -> bool {
    shared().tap_port.lock().is_some()
}

/// Обновить настройку autoswitch (после сохранения настроек).
pub fn set_autoswitch(on: bool) {
    shared().autoswitch.store(on, Ordering::SeqCst);
}

/// Обновить режим dry-run.
pub fn set_dry_run(on: bool) {
    *shared().dry_run.lock() = on;
}

/// Включить/выключить «Маскировку» в тапе.
pub fn set_masking(on: bool) {
    shared().masking.store(on, Ordering::SeqCst);
    tracing::info!(on, "маскировка включена/выключена");
}

/// Обновить интенсивность маскировки.
pub fn set_masking_intensity(v: u8) {
    shared()
        .masking_intensity
        .store(v.clamp(10, 100), Ordering::SeqCst);
}

/// Синхронизировать язык раскладки в тапе (из layout_notifier).
pub fn set_current_lang(code: u8) {
    shared().current_lang.store(code, Ordering::SeqCst);
}

/// Колбэк переключения маскировки из хоткея (ставит главный поток).
type MaskingToggleFn = Box<dyn Fn(bool) + Send>;
static MASKING_TOGGLE: Mutex<Option<MaskingToggleFn>> = Mutex::new(None);

/// Установить обработчик переключения маскировки (меню/иконка/конфиг).
pub fn set_masking_toggle(f: Box<dyn Fn(bool) + Send>) {
    *MASKING_TOGGLE.lock() = Some(f);
}

// MARK: — Обратный вызов тапа

extern "C" fn tap_callback(
    _proxy: *mut std::os::raw::c_void,
    event_type: ffi::CGEventType,
    event: ffi::CGEventRef,
    _user: *mut std::os::raw::c_void,
) -> ffi::CGEventRef {
    let Some(sh) = SHARED.get() else {
        return event;
    };
    let sh = Arc::clone(sh);

    if event_type == ffi::K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT
        || event_type == ffi::K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT
    {
        tracing::warn!(event_type, "тап выключен системой, включаю заново");
        if let Some(port) = *sh.tap_port.lock() {
            // SAFETY: порт жив, живёт до выхода приложения.
            unsafe { ffi::CGEventTapEnable(port.0, true) };
        }
        return event;
    }

    match event_type {
        ffi::K_CG_EVENT_FLAGS_CHANGED => {
            handle_flags_changed(&sh, event);
            // Модификаторы всегда пропускаются.
            event
        }
        ffi::K_CG_EVENT_LEFT_MOUSE_DOWN => {
            // Клик мыши сбрасывает буфер (раздел 3.1 ТЗ).
            let mut engine = engine_state::lock().lock();
            engine.on_event(
                Event::AppChanged {
                    bundle_id: String::new(),
                },
                now_ms(),
            );
            event
        }
        ffi::K_CG_EVENT_KEY_DOWN => handle_key_down(&sh, event),
        // Прочие события пропускаются без разбора.
        _ => event,
    }
}

/// Разбор keyDown: синтетика, off/secure, хоткеи, основной путь движка.
fn handle_key_down(sh: &Arc<TapShared>, event: ffi::CGEventRef) -> ffi::CGEventRef {
    // Наши синтетические события — пропускаем.
    let synthetic = unsafe {
        // SAFETY: чтение поля события.
        ffi::CGEventGetIntegerValueField(event, ffi::K_CG_EVENT_SOURCE_USER_DATA)
    } == ffi::CARAMBA_MAGIC;
    if synthetic {
        return event;
    }

    let keycode = unsafe {
        // SAFETY: чтение keycode.
        ffi::CGEventGetIntegerValueField(event, ffi::K_CG_KEYBOARD_EVENT_KEYCODE)
    } as u16;
    let repeat = unsafe {
        // SAFETY: чтение признака автоповтора.
        ffi::CGEventGetIntegerValueField(event, ffi::K_CG_KEYBOARD_EVENT_AUTOREPEAT) != 0
    };
    let flags = unsafe { ffi::CGEventGetFlags(event) };
    let shift = flags & ffi::K_CG_EVENT_FLAG_MASK_SHIFT != 0;
    let caps = flags & ffi::K_CG_EVENT_FLAG_MASK_ALPHA_SHIFT != 0;

    // Приложение выключено или Secure Input активен — пропускаем всё.
    if !sh.enabled.load(Ordering::SeqCst) || sh.secure.load(Ordering::SeqCst) {
        return event;
    }

    // Умная отмена по ⌫ (раздел 6 ТЗ): если последняя конвертация свежа
    // и после неё ничего не набрано — откат вместо удаления символа.
    if keycode == 0x33 && !repeat {
        if let Some(actions) = smart_backspace() {
            execute(&actions);
            return std::ptr::null_mut(); // сам ⌫ поглощается
        }
    }

    // Хоткей конвертации выделенного: ⌥⇧R (раздел 6 ТЗ).
    if keycode == 0x0F && flags & ffi::K_CG_EVENT_FLAG_MASK_OPTION != 0 && shift {
        tracing::info!("хоткей ⌥⇧R: конвертация выделенного");
        spawn_selection_convert();
        return std::ptr::null_mut(); // поглотить — не печатать 'r'
    }

    // Pause/F15: откат последней конвертации.
    if keycode == 0x71 {
        return undo_last();
    }

    // Хоткей маскировки: ⌃⌥M (0x2E), без Shift и Command.
    if keycode == 0x2E
        && flags & (ffi::K_CG_EVENT_FLAG_MASK_CONTROL | ffi::K_CG_EVENT_FLAG_MASK_OPTION) != 0
        && !shift
        && flags & ffi::K_CG_EVENT_FLAG_MASK_COMMAND == 0
        && !repeat
    {
        let on = !sh.masking.load(Ordering::SeqCst);
        if let Some(f) = MASKING_TOGGLE.lock().as_ref() {
            f(on);
        }
        return std::ptr::null_mut();
    }

    // Маскировка: подмена Unicode-строки прямо в событии (раздел 5 ТЗ).
    if sh.masking.load(Ordering::SeqCst)
        && !repeat
        && sh.current_lang.load(Ordering::SeqCst) == LANG_RU
        && flags & (ffi::K_CG_EVENT_FLAG_MASK_CONTROL | ffi::K_CG_EVENT_FLAG_MASK_COMMAND) == 0
    {
        if let Some(replacement) = mask_substitute(sh, keycode, shift, caps) {
            let mut units = [0u16; 2];
            replacement.encode_utf16(&mut units);
            // SAFETY: строка живёт до конца вызова; поле события — Unicode.
            unsafe {
                let len = replacement.len_utf16();
                ffi::CGEventKeyboardSetUnicodeString(event, len, units.as_ptr());
            }
            tracing::trace!(keycode, replacement = %replacement, "маскировка: буква-двойник");
        }
    }

    // Основной путь: движок принимает реальное нажатие.
    let mut engine = engine_state::lock().lock();
    let mut actions = engine.on_event(
        Event::KeyDown {
            keycode,
            shift,
            caps,
            repeat,
        },
        now_ms(),
    );

    if !sh.autoswitch.load(Ordering::SeqCst) {
        actions.retain(|a| !matches!(a, Action::Replace { .. } | Action::SwitchLayout { .. }));
    }

    if *sh.dry_run.lock() {
        for action in &actions {
            tracing::info!("dry-run: {action:?}");
        }
        return event;
    }

    execute(&actions);
    if actions.iter().any(|a| matches!(a, Action::Replace { .. })) {
        std::ptr::null_mut() // поглотить: замена уже напечатана
    } else {
        event
    }
}

/// Пары двойников (вшитая таблица `resources/masking/pairs.toml`).
fn mask_pairs() -> &'static MaskPairs {
    static ONCE: OnceLock<MaskPairs> = OnceLock::new();
    ONCE.get_or_init(MaskPairs::builtin)
}

/// Простое ГПСЧ для «какие буквы заменить случайно на лету».
struct MaskRng(u64);

impl MaskRng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Подменяемый символ для текущего нажатия, если он есть в таблице пар.
fn mask_substitute(sh: &Arc<TapShared>, keycode: u16, shift: bool, caps: bool) -> Option<char> {
    // Caps Lock не маскируем — регистр нестандартный.
    if caps {
        return None;
    }
    let engine = engine_state::lock().lock();
    let sym = engine
        .pair()
        .by_lang(caramba_core::layouts::Lang::Ru)
        .symbol(keycode, shift)?;
    let mut chars = sym.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None; // многозначные строки не подменяем
    }
    drop(engine);
    let intensity = sh.masking_intensity.load(Ordering::SeqCst) as u64;
    let mut rng = MaskRng(0x000C_6ABB_ABA1 ^ (now_ms() << 3) ^ (keycode as u64));
    let pairs = mask_pairs();
    // Направление всегда ru_to_lat: маскируем только в русской раскладке.
    try_replace(pairs, MaskDirection::RuToLat, c).filter(|_| rng.next() % 100 < intensity)
}

/// Исполнение решений движка.
pub fn execute(actions: &[Action]) {
    for action in actions {
        match action {
            Action::Pass | Action::Swallow => {}
            Action::Replace { backspaces, text } => {
                if *backspaces > 0 {
                    typing::post_backspaces(*backspaces);
                }
                typing::post_text(text);
            }
            Action::SwitchLayout { layout } => {
                tracing::info!(%layout, "переключение раскладки");
                let _ = super::input_source::select_by_id(layout);
            }
            Action::Sound => {} // звук — в M5
            Action::Hint { text } => tracing::info!("подсказка: {text}"),
        }
    }
}

/// Откат последней конвертации (⌥-tap без набора, Pause/F15).
fn undo_last() -> ffi::CGEventRef {
    let mut engine = engine_state::lock().lock();
    let window = engine.config().undo_window_ms;
    let buffer_empty = engine.buffer_is_empty();
    let actions = engine
        .undo_mut()
        .revert_last(now_ms(), window, buffer_empty);
    drop(engine);
    if actions.is_empty() {
        tracing::info!("undo: нечего откатывать");
        return std::ptr::null_mut();
    }
    execute(&actions);
    std::ptr::null_mut()
}

/// Умный ⌫: Some(действия) — откат (⌫ поглощается), None — обычное поведение.
fn smart_backspace() -> Option<Vec<Action>> {
    let mut engine = engine_state::lock().lock();
    if !engine.buffer_is_empty() {
        return None; // набирается слово — ⌫ работает как обычно
    }
    let window = engine.config().undo_window_ms;
    let actions = engine.undo_mut().revert_last(now_ms(), window, true);
    if actions.is_empty() {
        None
    } else {
        tracing::info!("умный ⌫: откат конвертации");
        Some(actions)
    }
}

/// flagsChanged: детектор одиночных Shift/Option-taps (решение D5).
fn handle_flags_changed(sh: &Arc<TapShared>, event: ffi::CGEventRef) {
    let keycode = unsafe {
        // SAFETY: чтение keycode.
        ffi::CGEventGetIntegerValueField(event, ffi::K_CG_KEYBOARD_EVENT_KEYCODE)
    } as u16;
    let flags = unsafe { ffi::CGEventGetFlags(event) };

    // 0x38/0x3C — левый/правый Shift; 0x3A/0x3D — левый/правый Option.
    match keycode {
        0x38 | 0x3C => {
            {
                let mut last = sh.last_flags.lock();
                *last = flags;
            }
            let now_down = flags & ffi::K_CG_EVENT_FLAG_MASK_SHIFT != 0;
            if now_down {
                *sh.shift_start.lock() = Some(Instant::now());
            } else if let Some(start) = sh.shift_start.lock().take() {
                if start.elapsed() < Duration::from_millis(300) {
                    on_single_shift_tap();
                }
            }
        }
        0x3A | 0x3D => {
            let now_down = flags & ffi::K_CG_EVENT_FLAG_MASK_OPTION != 0;
            if now_down {
                *sh.option_start.lock() = Some(Instant::now());
            } else if let Some(start) = sh.option_start.lock().take() {
                if start.elapsed() < Duration::from_millis(300) {
                    on_single_option_tap();
                }
            }
        }
        _ => {}
    }
}

/// Одиночный Shift-tap: смена раскладки без прерывания набора;
/// если до этого был ещё один tap (<400 мс) — Double Shift, откат.
fn on_single_shift_tap() {
    let sh = shared();
    if !sh.enabled.load(Ordering::SeqCst) {
        return;
    }
    // Double Shift: два tap подряд — Smart Undo.
    if let Some(at) = sh.last_tap_ms.lock().to_owned() {
        if now_ms().saturating_sub(at) <= 400 {
            *sh.last_tap_ms.lock() = None;
            tracing::info!("double shift: откат последней конвертации");
            let mut engine = engine_state::lock().lock();
            let window = engine.config().undo_window_ms;
            let actions = engine.undo_mut().revert_last(now_ms(), window, true);
            drop(engine);
            execute(&actions);
            return;
        }
    }
    let cur = {
        let engine = engine_state::lock().lock();
        engine.current_layout_id().to_string()
    };
    let next = if cur.contains("Russian") {
        "com.apple.keylayout.US"
    } else {
        "com.apple.keylayout.Russian"
    };
    *sh.last_tap_ms.lock() = Some(now_ms());
    tracing::info!("shift-tap: {cur} -> {next}");
    let _ = super::input_source::select_by_id(next);
    // Синхронизация движка произойдёт уведомлением системы (см. workers.rs),
    // но на всякий случай ускоряем.
    sync_engine_layout(next);
}

fn sync_engine_layout(new_layout: &str) {
    let mut engine = engine_state::lock().lock();
    engine.on_event(
        Event::LayoutChanged {
            new_layout: new_layout.into(),
        },
        now_ms(),
    );
}

/// Одиночный Option-tap: конвертация текущего слова или откат последней.
fn on_single_option_tap() {
    let sh = shared();
    if !sh.enabled.load(Ordering::SeqCst) {
        return;
    }
    let mut engine = engine_state::lock().lock();
    if engine.buffer_is_empty() {
        let window = engine.config().undo_window_ms;
        let actions = engine.undo_mut().revert_last(now_ms(), window, true);
        drop(engine);
        execute(&actions);
    } else {
        let actions = engine.evaluate_current_word(now_ms());
        drop(engine);
        execute(&actions);
    }
}

/// Конвертация выделенного фрагмента (⌥⇧R): ⌘C -> флип -> ⌘V,
/// буфер обмена восстанавливается (заглушка восстановления — M3).
fn spawn_selection_convert() {
    std::thread::Builder::new()
        .name("caramba-convert-sel".into())
        .spawn(move || {
            let saved = super::pasteboard::change_count();
            // SAFETY: порождение синтетических событий — стандартный путь.
            typing::post_hotkey_half(ffi::K_CG_EVENT_FLAG_MASK_COMMAND, 0x08, true); // ⌘C
            typing::post_hotkey_half(ffi::K_CG_EVENT_FLAG_MASK_COMMAND, 0x08, false);
            std::thread::sleep(Duration::from_millis(140));

            let Some(text) = super::pasteboard::text() else {
                return; // в буфере не текст — ничего не делаем
            };
            let pair: Pair = {
                let engine = engine_state::lock().lock();
                engine.pair().clone()
            };
            let flipped = pair.flip_auto(&text);
            if flipped == text {
                return;
            }
            super::pasteboard::set_text(&flipped);
            tracing::info!(before = %text, after = %flipped, "выделенное конвертировано");

            typing::post_hotkey_half(ffi::K_CG_EVENT_FLAG_MASK_COMMAND, 0x09, true); // ⌘V
            typing::post_hotkey_half(ffi::K_CG_EVENT_FLAG_MASK_COMMAND, 0x09, false);
            std::thread::sleep(Duration::from_millis(120));

            // Восстановление исходного буфера: пока сохраняем только текст.
            // Full FAT — буфер обмена может содержать иную структуру (фото),
            // восстановление полностью — M3 (chseleng).
            if saved != super::pasteboard::change_count() {
                // Никто другой не трогал буфер. Восстановим исходный текст.
                if let Some(original) = super::pasteboard::text() {
                    if original != flipped {
                        let _ = original; // не совпадает — ок, юзер копировал новое
                    }
                }
            }
        })
        .expect("поток конвертации выделенного");
}
