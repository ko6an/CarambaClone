//! NSWorkspace: первое приложение (bundle id), категория (игры)
//! — для профилей исключений (FR-EXC-3/5).

use objc2::rc::Retained;
use objc2_app_kit::NSRunningApplication;
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSBundle, NSString};

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

/// Категория приложения (LSApplicationCategoryType из Info.plist).
pub fn app_category(bundle_id: &str) -> String {
    if bundle_id.is_empty() {
        return String::new();
    }
    let bundle = unsafe {
        // SAFETY: конструктор Cocoa по строке-идентификатору.
        NSBundle::bundleWithIdentifier(&NSString::from_str(bundle_id))
    };
    let Some(bundle) = bundle else {
        return String::new();
    };
    let dict = bundle.infoDictionary();
    let Some(dict) = dict else {
        return String::new();
    };
    let key = NSString::from_str("LSApplicationCategoryType");
    let cat = dict.objectForKey(&key);
    let Some(cat) = cat else {
        return String::new();
    };
    // SAFETY: значение Info.plist по ключу LSApplicationCategoryType — NSString.
    let cat: Retained<NSString> = match Retained::downcast::<NSString>(cat) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    cat.to_string()
}

/// Bundle id приложения по пути (.app/алиас/симлинк) — для папки Games.
pub fn bundle_id_for_path_dir(path: &std::path::Path) -> Option<String> {
    let dir = path.to_str().map(str::to_owned)?;
    let ns = NSString::from_str(&dir);
    let bundle = unsafe {
        // SAFETY: конструктор Cocoa по строке пути к .app.
        NSBundle::bundleWithPath(&ns)
    };
    bundle
        .and_then(|b| b.bundleIdentifier())
        .map(|s| s.to_string())
}
