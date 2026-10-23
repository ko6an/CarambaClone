//! macOS-реализации поверх CGEventTap/TIS/AX.

#[cfg(target_os = "macos")]
pub mod ax;
#[cfg(target_os = "macos")]
pub mod engine_state;
#[cfg(target_os = "macos")]
pub mod event_tap;
#[cfg(target_os = "macos")]
pub mod ffi;
#[cfg(target_os = "macos")]
pub mod input_source;
#[cfg(target_os = "macos")]
pub mod layout_notifier;
#[cfg(target_os = "macos")]
pub mod main_thread;
#[cfg(target_os = "macos")]
pub mod pasteboard;
#[cfg(target_os = "macos")]
pub mod profiles;
#[cfg(target_os = "macos")]
pub mod sound;
pub mod spell;
#[cfg(target_os = "macos")]
pub mod typing;
#[cfg(target_os = "macos")]
pub mod workers;
#[cfg(target_os = "macos")]
pub mod workspace;

#[cfg(target_os = "macos")]
pub use input_source::MacosPlatform;

#[cfg(not(target_os = "macos"))]
/// Заглушки для не-macOS: `cargo check` должен проходить везде.
pub mod stubs {
    use super::super::Platform;

    pub struct StubPlatform;

    impl Platform for StubPlatform {
        fn current_layout(&self) -> String {
            "com.apple.keylayout.US".into()
        }
        fn select_layout(&self, _id: &str) -> Result<(), String> {
            Err("нет macOS".into())
        }
        fn post_keys(&self, _keys: Vec<(u16, bool)>) -> Result<(), String> {
            Err("нет macOS".into())
        }
        fn post_text(&self, _text: &str) -> Result<(), String> {
            Err("нет macOS".into())
        }
        fn frontmost_app(&self) -> String {
            "unknown".into()
        }
        fn is_secure_input(&self) -> bool {
            false
        }
        fn spell_check(&self, _word: &str, _lang: &str) -> bool {
            false
        }
        fn clipboard_get(&self) -> String {
            String::new()
        }
        fn clipboard_set(&self, _text: &str) {}
        fn play_sound(&self, _name: &str) {}
    }
}

#[cfg(not(target_os = "macos"))]
pub use stubs::StubPlatform;
