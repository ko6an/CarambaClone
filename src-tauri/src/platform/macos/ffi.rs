//! Тонкие FFI-обвязки над macOS API, не покрытые крейтами:
//! CoreGraphics-события и тапы, HIToolbox/TIS, Accessibility, CoreFoundation.
//!
//! Константы и сигнатуры повторяют заголовки Carbon/CoreGraphics/Accessibility.
//! Все unsafe-вызовы обёрнуты в безопасные помощники на стороне платформенных
//! модулей с комментариями SAFETY на месте вызова.

#![allow(dead_code)] // Часть констант/функций API включается этапами M2-M5.

use core_foundation::{array::CFArrayRef, base::CFAllocatorRef, string::CFStringRef};
use std::os::raw::c_void;

// MARK: — Типы

pub type CGEventRef = *mut c_void;
pub type CGEventSourceRef = *mut c_void;
pub type CFMachPortRef = *mut c_void;
pub type CFRunLoopRef = *mut c_void;
pub type CFRunLoopSourceRef = *mut c_void;
pub type CFNotificationCenterRef = *mut c_void;
pub type CFDictionaryRef = *const c_void;
pub type TISInputSourceRef = *mut c_void;
pub type CGEventField = u32;
pub type CGEventType = u32;
pub type CGEventTapLocation = u32;

/// Прокси-функция обратного вызова тапа. Возвращает событие (пропустить)
/// или NULL (поглотить).
pub type CGEventTapCallback = extern "C" fn(
    proxy: *mut c_void,
    event_type: CGEventType,
    event: CGEventRef,
    user: *mut c_void,
) -> CGEventRef;

/// Обратный вызов подписки на Darwin-уведомления.
pub type CFNotificationCallback = extern "C" fn(
    center: CFNotificationCenterRef,
    observer: *mut c_void,
    name: CFStringRef,
    object: *const c_void,
    user_info: CFDictionaryRef,
);

// MARK: — Константы CoreGraphics

pub const K_CG_HID_EVENT_TAP: CGEventTapLocation = 0;
pub const K_CG_SESSION_EVENT_TAP: CGEventTapLocation = 1;
pub const K_CG_ANNOTATED_SESSION_EVENT_TAP: CGEventTapLocation = 2;

/// kCGHeadInsertEventTap — тап раньше обработчиков приложений.
pub const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
/// kCGEventTapOptionDefault (0) — фильтрующий тап.
pub const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;
/// kCGEventTapOptionListenOnly (1) — только наблюдение.
pub const K_CG_EVENT_TAP_OPTION_LISTEN_ONLY: u32 = 1;

// Маски типов событий (CGEventMask, бит = значение CGEventType).
pub const K_CG_EVENT_MASK_LEFT_MOUSE_DOWN: u64 = 1 << 1;
pub const K_CG_EVENT_MASK_LEFT_MOUSE_UP: u64 = 1 << 2;
pub const K_CG_EVENT_MASK_KEY_DOWN: u64 = 1 << 10;
pub const K_CG_EVENT_MASK_KEY_UP: u64 = 1 << 11;
pub const K_CG_EVENT_MASK_FLAGS_CHANGED: u64 = 1 << 12;

// CGEventType
pub const K_CG_EVENT_LEFT_MOUSE_DOWN: CGEventType = 1;
pub const K_CG_EVENT_KEY_DOWN: CGEventType = 10;
pub const K_CG_EVENT_KEY_UP: CGEventType = 11;
pub const K_CG_EVENT_FLAGS_CHANGED: CGEventType = 12;
/// Тап выключен по таймауту — включить заново.
pub const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: CGEventType = 0xFFFFFFFF;
/// Тап выключен пользовательским вводом.
pub const K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT: CGEventType = 0xFFFFFFFE;

// CGEventField:
/// kCGEventSourceUserData — метка синтетических событий.
pub const K_CG_EVENT_SOURCE_USER_DATA: CGEventField = 42;
/// kCGKeyboardEventKeycode.
pub const K_CG_KEYBOARD_EVENT_KEYCODE: CGEventField = 9;
/// kCGKeyboardEventAutorepeat.
pub const K_CG_KEYBOARD_EVENT_AUTOREPEAT: CGEventField = 83;

// CGEventFlag — флаги-модификаторы.
pub const K_CG_EVENT_FLAG_MASK_SHIFT: u64 = 0x0000_0002;
pub const K_CG_EVENT_FLAG_MASK_CONTROL: u64 = 0x0000_0004;
pub const K_CG_EVENT_FLAG_MASK_OPTION: u64 = 0x0000_0008;
pub const K_CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x0000_0010;
pub const K_CG_EVENT_FLAG_MASK_ALPHA_SHIFT: u64 = 0x0001_0000;

// CGEventSourceStateID
pub const K_CG_EVENT_SOURCE_STATE_COMBINED: i64 = 0;
pub const K_CG_EVENT_SOURCE_STATE_HID_SYSTEM: i64 = 1;

/// Метка синтетических событий CarambaClone.
pub const CARAMBA_MAGIC: i64 = 0xCA_4BA_4BA;

// MARK: — Константы HIToolbox (TextInputSources)

/// kTISPropertyInputSourceID.
pub const K_TIS_PROPERTY_INPUT_SOURCE_ID: &str = "InputSourceID";
/// kTISPropertyInputSourceCategory.
pub const K_TIS_PROPERTY_INPUT_SOURCE_CATEGORY: &str = "InputSourceCategory";
/// kTISCategoryKeyboardInputSource.
pub const K_TIS_CATEGORY_KEYBOARD_INPUT_SOURCE: &str = "KeyboardInputSource";

// MARK: — Константы Accessibility

/// kAXTrustedCheckOptionPrompt.
pub const K_AX_TRUSTED_CHECK_OPTION_PROMPT: &str = "AXTrustedCheckOptionPrompt";

// MARK: — CoreGraphics

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn CGEventTapCreate(
        tap: CGEventTapLocation,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: CGEventTapCallback,
        user_info: *mut c_void,
    ) -> CFMachPortRef;
    pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    pub fn CGEventTapIsEnabled(tap: CFMachPortRef) -> bool;
    pub fn CGEventTapPostEvent(tap: CFMachPortRef, event: CGEventRef);
    pub fn CGEventPost(tap: CGEventTapLocation, event: CGEventRef);
    pub fn CGEventCreateKeyboardEvent(source: CGEventSourceRef, key: u16, down: bool)
        -> CGEventRef;
    pub fn CGEventSourceCreate(state_id: i64) -> CGEventSourceRef;
    pub fn CGEventGetIntegerValueField(event: CGEventRef, field: CGEventField) -> i64;
    pub fn CGEventSetIntegerValueField(event: CGEventRef, field: CGEventField, value: i64);
    pub fn CGEventGetFlags(event: CGEventRef) -> u64;
    pub fn CGEventSetFlags(event: CGEventRef, flags: u64);
    pub fn CGEventKeyboardSetUnicodeString(event: CGEventRef, length: usize, string: *const u16);
    pub fn CGPreflightListenEventAccess() -> bool;
    pub fn CGPreflightPostEventAccess() -> bool;
}

// MARK: — CoreFoundation

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub static kCFAllocatorDefault: CFAllocatorRef;
    pub static kCFRunLoopCommonModes: CFStringRef;
    pub fn CFMachPortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;
    pub fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    pub fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    pub fn CFRunLoopRun();
    pub fn CFRunLoopStop(rl: CFRunLoopRef);
    pub fn CFRelease(cf: *const c_void);
    pub fn CFNotificationCenterGetDistributedCenter() -> CFNotificationCenterRef;
    pub fn CFNotificationCenterAddObserver(
        center: CFNotificationCenterRef,
        observer: *const c_void,
        callback: CFNotificationCallback,
        name: CFStringRef,
        object: *const c_void,
        suspension_behavior: u32,
    );
}

// MARK: — HIToolbox / Carbon (TIS, secure input)

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    pub static kTISPropertyInputSourceID: CFStringRef;
    pub static kTISPropertyInputSourceCategory: CFStringRef;
    pub static kTISPropertyInputSourceLanguages: CFStringRef;
    pub static kTISCategoryKeyboardInputSource: CFStringRef;
    /// Настоящее имя Darwin-уведомления о смене выбранного источника
    /// ввода (строкой его задавать нельзя — центр сопоставляет по
    /// самому CFString-символу).
    pub static kTISNotifySelectedKeyboardInputSourceChanged: CFStringRef;
    pub fn TISCreateInputSourceList(
        properties: CFDictionaryRef,
        include_all_installed: bool,
    ) -> CFArrayRef;
    pub fn TISSelectInputSource(source: TISInputSourceRef) -> i32;
    pub fn TISCopyCurrentKeyboardInputSource() -> TISInputSourceRef;
    pub fn TISGetInputSourceProperty(source: TISInputSourceRef, key: CFStringRef) -> *mut c_void;
    pub fn IsSecureEventInputEnabled() -> bool;
}

// MARK: — Accessibility

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub fn AXIsProcessTrusted() -> bool;
    pub fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
}

// MARK: — AppKit (линовка фреймворка; вызовы идут через objc2-app-kit)

#[link(name = "AppKit", kind = "framework")]
extern "C" {
    /// Фиктивная точка линковки фреймворка AppKit.
    #[allow(dead_code)]
    pub fn NSBeep();
}
