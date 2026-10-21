//! NSWorkspace: первое приложение (bundle id) — для профилей исключений.

use objc2_app_kit::{NSRunningApplication, NSWorkspace};

/// Bundle id первого приложения ("" — не удалось получить).
pub fn frontmost_bundle_id() -> String {
    let ws = NSWorkspace::sharedWorkspace();
    let app: Option<objc2::rc::Retained<NSRunningApplication>> = ws.frontmostApplication();
    match app {
        Some(app) => {
            let bundle = app.bundleIdentifier();
            bundle.map(|b| b.to_string()).unwrap_or_default()
        }
        None => String::new(),
    }
}
