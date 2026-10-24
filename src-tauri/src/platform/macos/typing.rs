//! Печать синтетических событий (CGEventPost): замены, backspaces,
//! текст юникод-строками, хоткеи. Все события помечаются MAGIC —
//! тап пропускает их.

use caramba_core::layouts::{classify, CharClass};
use std::sync::OnceLock;

use super::ffi;

/// Встроенная пара раскладок (однократная инициализация).
#[allow(dead_code)]
fn pair() -> &'static caramba_core::layouts::Pair {
    static PAIR: OnceLock<caramba_core::layouts::Pair> = OnceLock::new();
    PAIR.get_or_init(caramba_core::layouts::builtin_pair)
}

/// Печать текста «настоящими клавишами» (запасной экспериментальный путь).
#[allow(dead_code)]
pub fn post_text_smart(text: &str) {
    let pair = pair();
    let cyr = text.chars().any(|c| classify(c) == CharClass::Cyrillic);
    let table = if cyr { &pair.ru } else { &pair.en };
    let mut missing = String::new();
    let mut keys: Vec<(u16, bool)> = Vec::new();
    for c in text.chars() {
        let key = c.to_string();
        if let Some(e) = table.keys.iter().find(|e| e.lower == key || e.upper == key) {
            // Сначала сбрасываем копящийся юникод-хвост, потом клавиша.
            if !missing.is_empty() {
                post_text(&missing);
                missing.clear();
            }
            keys.push((e.kc, e.upper == key));
        } else {
            missing.push(c);
        }
    }
    if !missing.is_empty() {
        post_text(&missing);
    }
    if !keys.is_empty() {
        post_keys(&keys);
    }
}

/// Указатель CF-источника: безопасен для хранения между потоками
/// (CGEventSource иммутабелен после создания).
struct SourcePtr(ffi::CGEventSourceRef);
unsafe impl Send for SourcePtr {}
unsafe impl Sync for SourcePtr {}

/// Единый источник синтетических событий.
fn source() -> ffi::CGEventSourceRef {
    use std::sync::OnceLock;
    static SOURCE: OnceLock<SourcePtr> = OnceLock::new();
    SOURCE
        .get_or_init(|| {
            // SAFETY: создание CGEventSource — безобидный вызов.
            SourcePtr(unsafe { ffi::CGEventSourceCreate(ffi::K_CG_EVENT_SOURCE_STATE_COMBINED) })
        })
        .0
}

/// Пометить событие нашими данными.
unsafe fn mark(event: ffi::CGEventRef) {
    // SAFETY: event — живой CGEventRef, поле чтения/записи корректно.
    ffi::CGEventSetIntegerValueField(event, ffi::K_CG_EVENT_SOURCE_USER_DATA, ffi::CARAMBA_MAGIC);
}

fn post_key(kc: u16, down: bool, flags: u64) {
    unsafe {
        // SAFETY: создаём наше событие и после поста освобождаем.
        let ev = ffi::CGEventCreateKeyboardEvent(source(), kc, down);
        if ev.is_null() {
            return;
        }
        mark(ev);
        if flags != 0 {
            ffi::CGEventSetFlags(ev, flags);
        }
        ffi::CGEventPost(ffi::K_CG_SESSION_EVENT_TAP, ev);
        ffi::CFRelease(ev as *const std::os::raw::c_void);
    }
}

/// Публичная обёртка: отправить синтетическое нажатие с флагами.
/// Используется для перепоста граничного события после замены текста.
pub fn post_key_event(kc: u16, down: bool, flags: u64) {
    post_key(kc, down, flags);
}

/// Нажатия клавиш: (keycode, shift).
pub fn post_keys(keys: &[(u16, bool)]) {
    for &(kc, shift) in keys {
        let flags = if shift {
            ffi::K_CG_EVENT_FLAG_MASK_SHIFT
        } else {
            0
        };
        post_key(kc, true, flags);
        post_key(kc, false, flags);
    }
}

/// Печать текста: для каждого символа находим соответствующую клавишу раскладки
/// и одновременно прикрепляем Unicode-символ.
/// Это гарантирует работу во всех приложениях (браузеры, Electron, AppKit, терминалы).
pub fn post_text(text: &str) {
    if text.is_empty() {
        return;
    }
    let pair = pair();
    let cyr = text.chars().any(|c| classify(c) == CharClass::Cyrillic);
    let table = if cyr { &pair.ru } else { &pair.en };

    for c in text.chars() {
        let key = c.to_string();
        let (kc, shift) = if let Some(e) = table.keys.iter().find(|e| e.lower == key || e.upper == key) {
            (e.kc, e.upper == key)
        } else {
            (0, false)
        };

        let flags = if shift {
            ffi::K_CG_EVENT_FLAG_MASK_SHIFT
        } else {
            0
        };

        let mut utf16 = [0u16; 2];
        let utf16_slice = c.encode_utf16(&mut utf16);
        let len = utf16_slice.len();

        unsafe {
            let ev = ffi::CGEventCreateKeyboardEvent(source(), kc, true);
            if !ev.is_null() {
                mark(ev);
                if flags != 0 {
                    ffi::CGEventSetFlags(ev, flags);
                }
                ffi::CGEventKeyboardSetUnicodeString(ev, len, utf16.as_ptr());
                ffi::CGEventPost(ffi::K_CG_SESSION_EVENT_TAP, ev);
                ffi::CFRelease(ev as *const std::os::raw::c_void);
            }

            let up_ev = ffi::CGEventCreateKeyboardEvent(source(), kc, false);
            if !up_ev.is_null() {
                mark(up_ev);
                if flags != 0 {
                    ffi::CGEventSetFlags(up_ev, flags);
                }
                ffi::CGEventKeyboardSetUnicodeString(up_ev, len, utf16.as_ptr());
                ffi::CGEventPost(ffi::K_CG_SESSION_EVENT_TAP, up_ev);
                ffi::CFRelease(up_ev as *const std::os::raw::c_void);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

/// Стереть `n` символов слева от каретки backspace-ами.
pub fn post_backspaces(n: usize) {
    for _ in 0..n {
        post_key(0x33, true, 0); // kVK_Delete
        post_key(0x33, false, 0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

/// Хоткей с модификаторами (например, ⌘C: flags=cmd, kc=0x08).
/// `down`/`up` разделены для точной хореографии.
pub fn post_hotkey_half(flags: u64, kc: u16, down: bool) {
    post_key(kc, down, flags);
}

#[cfg(test)]
mod tests {
    use caramba_core::layouts::{builtin_pair, classify, CharClass};

    /// Полностью ли текст представим клавишами таблицы (без юникод-fallback).
    fn is_key_typable(text: &str) -> bool {
        let pair = builtin_pair();
        let cyr = text.chars().any(|c| classify(c) == CharClass::Cyrillic);
        let table = if cyr { &pair.ru } else { &pair.en };
        text.chars().all(|c| {
            let key = c.to_string();
            table.keys.iter().any(|e| e.lower == key || e.upper == key)
        })
    }

    #[test]
    fn замены_представимы_клавишами() {
        assert!(is_key_typable("привет"));
        assert!(is_key_typable("Здравствуйте"));
        assert!(is_key_typable("hello"));
        assert!(is_key_typable("ghbdtn"));
        assert!(is_key_typable("Спасибо"));
    }
}
