//! Подписка на смену выбранного источника ввода
//! (Darwin-уведомление kTISNotifySelectedKeyboardInputSourceChanged).
//! Вызывается на главном рун-лупе; синхронизирует движок и трей.

use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use tauri::AppHandle;

use super::input_source::current_input_source_id;
use crate::tray;

static HANDLE: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();

extern "C" fn on_selected_input_changed() {
    let Some(handle) = HANDLE.get() else {
        return;
    };
    let id = current_input_source_id();
    tracing::info!(layout = %id, "система сменила раскладку");
    // Синхронизируем язык движка с системой.
    {
        let mut engine = super::engine_state::lock().lock();
        engine.on_event(
            caramba_core::engine::Event::LayoutChanged {
                new_layout: id.clone(),
            },
            super::event_tap::now_ms(),
        );
    }
    // Обновляем флаг в трее.
    tray::update_layout_flag(handle, &id);
    // Маскировка живёт только в русской раскладке: синхронизируем тап.
    let code = if id.contains("Russian") {
        super::event_tap::LANG_RU
    } else {
        super::event_tap::LANG_EN
    };
    super::event_tap::set_current_lang(code);
}

/// Подписаться (на главном потоке, до запуска рун-лупа приложения).
pub fn start(app: &AppHandle) {
    let _ = HANDLE.set(app.clone());
    unsafe {
        // SAFETY: колбэк — глобальная функция; подписка на Darwin-уведомление.
        let name = CFString::new(super::ffi::K_TIS_NOTIFY_SELECTED_INPUT_SOURCE_CHANGED);
        let center = super::ffi::CFNotificationCenterGetDistributedCenter();
        super::ffi::CFNotificationCenterAddObserver(
            center,
            std::ptr::null(),
            on_selected_input_changed_notify,
            name.as_concrete_TypeRef(),
            std::ptr::null(),
            0,
        );
    }
    // Первичная установка флага.
    let id = current_input_source_id();
    tray::update_layout_flag(app, &id);
}

/// Обёртка с назначенной сигнатурой для CFNotificationCenter.
extern "C" fn on_selected_input_changed_notify(
    _center: super::ffi::CFNotificationCenterRef,
    _observer: *mut std::os::raw::c_void,
    _name: core_foundation::string::CFStringRef,
    _object: *const std::os::raw::c_void,
    _user_info: super::ffi::CFDictionaryRef,
) {
    on_selected_input_changed();
}
