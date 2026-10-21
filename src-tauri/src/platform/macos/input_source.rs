//! Работа с источниками ввода (TIS): чтение текущей раскладки и выбор.
//!
//! Все вызовы TIS должны выполняться на главном потоке (требование macOS).

use core_foundation::base::{CFRelease, TCFType};
use core_foundation::string::{CFString, CFStringRef};

use crate::platform::Platform;

use super::ffi;

/// Реализация `Platform` для macOS.
pub struct MacosPlatform;

impl MacosPlatform {
    pub fn new() -> Self {
        Self
    }

    /// Property `kTISPropertyInputSourceID` текущего источника ввода.
    fn current_source_id() -> String {
        unsafe {
            let src = ffi::TISCopyCurrentKeyboardInputSource();
            if src.is_null() {
                return String::new();
            }
            let prop = ffi::TISGetInputSourceProperty(src, ffi::kTISPropertyInputSourceID);
            let result = if prop.is_null() {
                String::new()
            } else {
                CFString::wrap_under_get_rule(prop).to_string()
            };
            CFRelease(src as *const std::ffi::c_void);
            result
        }
    }
}

impl Platform for MacosPlatform {
    fn current_layout(&self) -> String {
        Self::current_source_id()
    }

    fn select_layout(&self, id: &str) -> Result<(), String> {
        unsafe {
            // Ищем источник по InputSourceID среди включённых в системе.
            let list = ffi::TISCreateInputSourceList(std::ptr::null(), false.into());
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
                let src_id = ffi::TISGetInputSourceProperty(src, ffi::kTISPropertyInputSourceID);
                if !src_id.is_null() {
                    let s = CFString::wrap_under_get_rule(src_id as CFStringRef);
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

    fn post_keys(&self, _keys: Vec<(u16, bool)>) -> Result<(), String> {
        // Реализуется в M2 (event tap / CGEventPost).
        Err("не реализовано до M2".into())
    }

    fn post_text(&self, _text: &str) -> Result<(), String> {
        // Реализуется в M2 через CGEventKeyboardSetUnicodeString.
        Err("не реализовано до M2".into())
    }

    fn frontmost_app(&self) -> String {
        // Реализуется в M2 через NSWorkspace.
        String::new()
    }

    fn is_secure_input(&self) -> bool {
        // Реализуется в M2 через IsSecureEventInputEnabled.
        false
    }

    fn spell_check(&self, _word: &str, _lang: &str) -> bool {
        // Реализуется в M3 через NSSpellChecker (главный поток).
        false
    }

    fn clipboard_get(&self) -> String {
        // Реализуется в M2 через NSPasteboard.
        String::new()
    }

    fn clipboard_set(&self, _text: &str) {
        // Реализуется в M2 через NSPasteboard.
    }

    fn play_sound(&self, _name: &str) {
        // Реализуется в M5 через NSSound.
    }
}

impl Default for MacosPlatform {
    fn default() -> Self {
        Self::new()
    }
}
