//! Работа с источниками ввода (TIS): чтение текущей раскладки и выбор;
//! платформенные операции (печать, буфер, Secure Input, NSWorkspace).

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

/// Выбрать источник ввода по InputSourceID (вне трейта, вызывается и
/// из потока тапа для действий SwitchLayout).
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
