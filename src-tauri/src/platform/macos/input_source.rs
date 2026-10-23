//! Работа с источниками ввода (TIS): чтение текущей раскладки и выбор;
//! платформенные операции (печать, буфер, Secure Input, NSWorkspace).

use core_foundation::base::{CFRelease, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use std::time::Duration;

use crate::platform::Platform;

use super::ffi;

/// Идентификаторы встроенной пары — значения по умолчанию, если в системе
/// включены другие раскладки (например, «ABC» вместо «U.S.»).
const FALLBACK_EN: &str = "com.apple.keylayout.US";
const FALLBACK_RU: &str = "com.apple.keylayout.Russian";

/// Включённые в системе раскладки пары EN/RU (кэш; заполняется в главном
/// потоке при старте и по уведомлению о смене источника ввода).
static ENABLED_PAIR: parking_lot::Mutex<Option<(String, String)>> = parking_lot::Mutex::new(None);

/// Идентификатор включённой английской раскладки (US, ABC, British…).
pub fn enabled_en_id() -> String {
    ENABLED_PAIR
        .lock()
        .as_ref()
        .map_or_else(|| FALLBACK_EN.into(), |p| p.0.clone())
}

/// Идентификатор включённой русской раскладки (Russian или Russian – PC).
pub fn enabled_ru_id() -> String {
    ENABLED_PAIR
        .lock()
        .as_ref()
        .map_or_else(|| FALLBACK_RU.into(), |p| p.1.clone())
}

/// Пересчитать кэш по включённым источникам ввода.
/// Только главный поток: TISCreateInputSourceList требует main-очередь.
pub fn refresh_enabled_pair() {
    unsafe {
        // SAFETY: TISCreateInputSourceList(false) — включённые источники.
        let list = ffi::TISCreateInputSourceList(std::ptr::null(), false);
        if list.is_null() {
            return;
        }
        let arr =
            core_foundation::array::CFArray::<*const std::os::raw::c_void>::wrap_under_create_rule(
                list as *const _,
            );
        let mut en: Option<String> = None;
        let mut ru: Option<String> = None;
        for item in arr.iter() {
            let src = *item as ffi::TISInputSourceRef;
            let prop = ffi::TISGetInputSourceProperty(src, ffi::kTISPropertyInputSourceID);
            if prop.is_null() {
                continue;
            }
            let id = CFString::wrap_under_get_rule(prop as CFStringRef).to_string();
            // Клавиатурные раскладки; IME и палитры символов пропускаем.
            if !id.starts_with("com.apple.keylayout.") {
                continue;
            }
            let lang = source_primary_language(src);
            if en.is_none() && lang.starts_with("en") {
                en = Some(id.clone());
            } else if ru.is_none() && lang.starts_with("ru") {
                ru = Some(id);
            }
        }
        if en.is_some() || ru.is_some() {
            *ENABLED_PAIR.lock() = Some((
                en.unwrap_or_else(|| FALLBACK_EN.into()),
                ru.unwrap_or_else(|| FALLBACK_RU.into()),
            ));
        }
        let (en, ru) = (enabled_en_id(), enabled_ru_id());
        tracing::info!(en = %en, ru = %ru, "включённые раскладки пары");
    }
}

/// Логический id движка («US»/«Russian» из таблиц) → реальный включённый
/// источник системы. Без этого на машинах с «ABC» вместо «U.S.» выбор
/// английской раскладки всегда падает («источник не найден»).
pub fn resolve_layout_id(logical: &str) -> String {
    if logical.contains("Russian") {
        enabled_ru_id()
    } else {
        enabled_en_id()
    }
}

/// Потокобезопасная смена раскладки: TIS выполняется только в главном
/// потоке (из колбэка тапа — краш, см. main_thread.rs). Логический id
/// движка автоматически разрешается в реальный включённый источник.
pub fn select_layout_threadsafe(logical_id: &str) -> Result<(), String> {
    let id = resolve_layout_id(logical_id);
    let result = super::main_thread::run_with_timeout(
        move || {
            let exact = select_by_id(&id);
            // Запасной путь: точный ID мог пропасть из включённых
            // между обновлением кэша и выбором.
            if exact.is_err() {
                let want_lang = if id.contains("Russian") { "ru" } else { "en" };
                select_by_language(want_lang).map(|_| ())
            } else {
                exact
            }
        },
        Duration::from_millis(250),
    );
    match result {
        Some(Ok(())) => Ok(()),
        Some(Err(e)) => Err(e),
        None => Err("главный поток не ответил (тайм-аут)".into()),
    }
}

/// Реализация `Platform` для macOS.
pub struct MacosPlatform;

impl MacosPlatform {
    pub fn new() -> Self {
        Self
    }
}

/// Идентификатор текущего источника ввода (свободная функция — вызывается
/// и из остальных модулей платформы).
pub fn current_input_source_id() -> String {
    unsafe {
        // SAFETY: TIS* — тредобезопасны; объект освобождаем CFRelease-ем.
        let src = ffi::TISCopyCurrentKeyboardInputSource();
        if src.is_null() {
            return String::new();
        }
        let prop = ffi::TISGetInputSourceProperty(src, ffi::kTISPropertyInputSourceID);
        let result = if prop.is_null() {
            String::new()
        } else {
            CFString::wrap_under_get_rule(prop as CFStringRef).to_string()
        };
        CFRelease(src as *const std::ffi::c_void);
        result
    }
}

/// Языки источника (первый элемент массива kTISPropertyInputSourceLanguages).
fn source_primary_language(src: ffi::TISInputSourceRef) -> String {
    unsafe {
        // SAFETY: TISGetInputSourceProperty возвращает CFArrayRef языков.
        let prop = ffi::TISGetInputSourceProperty(src, ffi::kTISPropertyInputSourceLanguages);
        if prop.is_null() {
            return String::new();
        }
        // SAFETY: возвращённый указатель — CFArrayRef (Get-правило).
        let arr =
            core_foundation::array::CFArray::<*const std::os::raw::c_void>::wrap_under_get_rule(
                prop as *const _,
            );
        arr.iter()
            .next()
            .map(|p| {
                // SAFETY: первый элемент массива — CFStringRef языка ("en", "ru").
                CFString::wrap_under_get_rule(*p as CFStringRef).to_string()
            })
            .unwrap_or_default()
    }
}

/// Выбрать первый источник с основным языком, начинающимся на prefix
/// ("en", "ru"). Запасной путь, когда конкретный ID (US и т.п.) не найден.
pub fn select_by_language(prefix: &str) -> Result<String, String> {
    unsafe {
        // SAFETY: TISCreateInputSourceList(false) — включённые источники.
        let list = ffi::TISCreateInputSourceList(std::ptr::null(), false);
        if list.is_null() {
            return Err("TIS: список источников пуст".into());
        }
        let arr =
            core_foundation::array::CFArray::<*const std::os::raw::c_void>::wrap_under_create_rule(
                list as *const _,
            );
        for item in arr.iter() {
            let src = *item as ffi::TISInputSourceRef;
            let lang = source_primary_language(src);
            if lang.starts_with(prefix) {
                let prop = ffi::TISGetInputSourceProperty(src, ffi::kTISPropertyInputSourceID);
                let id = if prop.is_null() {
                    String::new()
                } else {
                    CFString::wrap_under_get_rule(prop as CFStringRef).to_string()
                };
                if ffi::TISSelectInputSource(src) == 0 {
                    return Ok(id);
                }
            }
        }
        Err(format!("нет источника с языком {prefix}"))
    }
}

/// Выбрать источник ввода по InputSourceID (только главный поток:
/// HIToolbox требует main-очередь; из фоновых потоков — через
/// `select_layout_threadsafe`).
pub fn select_by_id(id: &str) -> Result<(), String> {
    unsafe {
        // SAFETY: TISCreateInputSourceList — чистая функция; массив CF.
        let list = ffi::TISCreateInputSourceList(std::ptr::null(), false);
        if list.is_null() {
            return Err("TIS: список источников пуст".into());
        }
        let arr =
            core_foundation::array::CFArray::<*const std::ffi::c_void>::wrap_under_create_rule(
                list,
            );
        let target = CFString::new(id);
        let mut found: Option<ffi::TISInputSourceRef> = None;
        for item in arr.iter() {
            let src = *item as ffi::TISInputSourceRef;
            let prop = ffi::TISGetInputSourceProperty(src, ffi::kTISPropertyInputSourceID);
            if !prop.is_null() {
                let s = CFString::wrap_under_get_rule(prop as CFStringRef);
                if s == target {
                    found = Some(src);
                    break;
                }
            }
        }
        match found {
            Some(src) => {
                let status = ffi::TISSelectInputSource(src);
                if status == 0 {
                    Ok(())
                } else {
                    Err(format!("TISSelectInputSource: код {status}"))
                }
            }
            None => Err(format!("источник {id} не найден")),
        }
    }
}

#[allow(dead_code)] // Трейт задействуется полностью этапами M3-M5.
impl Platform for MacosPlatform {
    fn current_layout(&self) -> String {
        current_input_source_id()
    }

    fn select_layout(&self, id: &str) -> Result<(), String> {
        select_by_id(id)
    }

    fn post_keys(&self, keys: Vec<(u16, bool)>) -> Result<(), String> {
        super::typing::post_keys(&keys);
        Ok(())
    }

    fn post_text(&self, text: &str) -> Result<(), String> {
        super::typing::post_text(text);
        Ok(())
    }

    fn frontmost_app(&self) -> String {
        super::workspace::frontmost_bundle_id()
    }

    fn is_secure_input(&self) -> bool {
        super::ax::is_secure_event_input()
    }

    fn spell_check(&self, _word: &str, _lang: &str) -> bool {
        // Подключается в M3 через NSSpellChecker (главный поток).
        false
    }

    fn clipboard_get(&self) -> String {
        super::pasteboard::text().unwrap_or_default()
    }

    fn clipboard_set(&self, text: &str) {
        super::pasteboard::set_text(text)
    }

    fn play_sound(&self, _name: &str) {
        // Подключается в M5 через NSSound.
    }
}

impl Default for MacosPlatform {
    fn default() -> Self {
        Self::new()
    }
}
