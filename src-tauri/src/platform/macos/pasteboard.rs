//! NSPasteboard: чтение/запись текста в буфер обмена (для ⌥⇧R).

use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;

/// Обычная системная доска.
fn general() -> objc2::rc::Retained<NSPasteboard> {
    NSPasteboard::generalPasteboard()
}

/// Текст в буфере обмена; None — если текста нет.
pub fn text() -> Option<String> {
    let pb = general();
    let s = pb.stringForType(unsafe { NSPasteboardTypeString })?;
    Some(s.to_string())
}

/// Записать текст (общим типом public.utf8-plain-text).
pub fn set_text(s: &str) {
    let pb = general();
    pb.clearContents();
    let ns = NSString::from_str(s);
    pb.setString_forType(&ns, unsafe { NSPasteboardTypeString });
}

/// Счетчик изменений буфера — для проверки «всё ещё наш текст?»,
/// затираем ли мы чужое.
pub fn change_count() -> isize {
    general().changeCount()
}
