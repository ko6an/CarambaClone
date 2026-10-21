//! Тонкие FFI-обвязки над macOS API, не покрытые крейтами.
//!
//! Все unsafe-блоки снабжены комментариями `// SAFETY:` по-русски.

#[cfg(target_os = "macos")]
#[allow(dead_code)] // Часть объявлений понадобится в M2 (событие смены раскладки и др.)
mod tis {
    use core_foundation::string::CFStringRef;

    #[repr(C)]
    pub struct OpaqueTISInputSource {
        _data: [u8; 0],
    }
    pub type TISInputSourceRef = *const OpaqueTISInputSource;

    extern "C" {
        pub static kTISPropertyInputSourceID: CFStringRef;
        pub static kTISPropertyLocalizedName: CFStringRef;
        pub static kTISNotifySelectedKeyboardInputSourceChanged: CFStringRef;

        pub fn TISCopyCurrentKeyboardInputSource() -> TISInputSourceRef;
        pub fn TISCreateInputSourceList(
            properties: *const core_foundation::dictionary::CFDictionaryRef,
            includeAllInstalled: core_foundation::base::Boolean,
        ) -> core_foundation::array::CFArrayRef;
        pub fn TISSelectInputSource(source: TISInputSourceRef) -> i32;
        pub fn TISGetInputSourceProperty(
            source: TISInputSourceRef,
            key: CFStringRef,
        ) -> core_foundation::string::CFStringRef;
    }
}

pub use tis::*;
