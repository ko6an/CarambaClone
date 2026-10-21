//! Accessibility: статус доверия, Secure Input, доступ к событиям.

use super::ffi;
use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;

/// Доступность Accessibility для нашего процесса (без показа предупреждения).
pub fn is_accessibility_trusted() -> bool {
    is_accessibility_trusted_prompt(false)
}

/// Доступность Accessibility с показом системного предупреждения.
pub fn is_accessibility_trusted_prompt(prompt: bool) -> bool {
    unsafe {
        // SAFETY: строим словарь {AXTrustedCheckOptionPrompt: CFBoolean} и
        // вызываем системную функцию.
        let key = CFString::new(ffi::K_AX_TRUSTED_CHECK_OPTION_PROMPT);
        let dict = CFDictionary::<CFString, CFBoolean>::from_CFType_pairs(&[(
            key,
            CFBoolean::from(prompt),
        )]);
        ffi::AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef() as *const std::os::raw::c_void)
    }
}

/// Разрешение «Input Monitoring» (слушать события).
pub fn listen_event_access() -> bool {
    // SAFETY: чистый запрос статуса.
    unsafe { ffi::CGPreflightListenEventAccess() }
}

/// Разрешение на генерацию событий.
pub fn post_event_access() -> bool {
    // SAFETY: чистый запрос статуса.
    unsafe { ffi::CGPreflightPostEventAccess() }
}

/// Включён ли Secure Input (пароли, парольные поля Keychain).
pub fn is_secure_event_input() -> bool {
    // SAFETY: чистый запрос статуса.
    unsafe { ffi::IsSecureEventInputEnabled() }
}
