//! Печать синтетических событий (CGEventPost): замены, backspaces,
//! текст юникод-строками, хоткеи. Все события помечаются MAGIC —
//! тап пропускает их.

use super::ffi;

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

/// Сколько UTF-16 единиц печатается за один keyDown (ограничение API).
const UNICODE_CHUNK: usize = 20;

/// Печать текста юникод-строками (чанками по 20 UTF-16 единиц).
pub fn post_text(text: &str) {
    let units: Vec<u16> = text.encode_utf16().collect();
    if units.is_empty() {
        return;
    }
    for chunk in units.chunks(UNICODE_CHUNK) {
        unsafe {
            // SAFETY: keycode 255 («неопределённая» клавиша) — стандартный
            // способ напечатать юникод; событие освобождаем сразу.
            let ev = ffi::CGEventCreateKeyboardEvent(source(), 255, true);
            if ev.is_null() {
                continue;
            }
            mark(ev);
            ffi::CGEventKeyboardSetUnicodeString(ev, chunk.len(), chunk.as_ptr());
            ffi::CGEventPost(ffi::K_CG_SESSION_EVENT_TAP, ev);
            let up_ev = ffi::CGEventCreateKeyboardEvent(source(), 255, false);
            if !up_ev.is_null() {
                mark(up_ev);
                ffi::CGEventKeyboardSetUnicodeString(up_ev, chunk.len(), chunk.as_ptr());
                ffi::CGEventPost(ffi::K_CG_SESSION_EVENT_TAP, up_ev);
                ffi::CFRelease(up_ev as *const std::os::raw::c_void);
            }
            ffi::CFRelease(ev as *const std::os::raw::c_void);
        }
    }
}

/// Стереть `n` символов слева от каретки backspace-ами.
pub fn post_backspaces(n: usize) {
    for _ in 0..n {
        post_key(0x33, true, 0); // kVK_Delete
        post_key(0x33, false, 0);
    }
}

/// Хоткей с модификаторами (например, ⌘C: flags=cmd, kc=0x08).
/// `down`/`up` разделены для точной хореографии.
pub fn post_hotkey_half(flags: u64, kc: u16, down: bool) {
    post_key(kc, down, flags);
}
