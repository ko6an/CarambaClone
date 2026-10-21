//! Иконка и меню в строке меню (трей).
//!
//! В M0 — минимальное меню: Включить/выключить, Пауза-заглушка, Настройки, Выход.

use caramba_core::layouts::builtin_pair;
use tauri::{
    image::Image,
    menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

/// Пункты меню, по ним находим их в обработчике.
pub const MENU_TOGGLE: &str = "toggle";
pub const MENU_MASKING: &str = "masking";
pub const MENU_SETTINGS: &str = "settings";
pub const MENU_SWITCH_LAYOUT: &str = "switch_layout";
pub const MENU_QUIT: &str = "quit";
#[allow(dead_code)] // Используется при реализации паузы (M5)
pub const MENU_PAUSE: &str = "pause";

/// Создать трей с иконкой-шаблоном и меню.
pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle =
        MenuItemBuilder::with_id(MENU_TOGGLE, "Включить приложение / Enable").build(app)?;
    let masking = CheckMenuItemBuilder::with_id(MENU_MASKING, "Маскировка (⌃⌥M) / Masking")
        .checked(false)
        .build(app)?;
    let switch =
        MenuItemBuilder::with_id(MENU_SWITCH_LAYOUT, "Сменить раскладку… / Switch layout…")
            .build(app)?;
    let settings = MenuItemBuilder::with_id(MENU_SETTINGS, "Настройки… / Settings…").build(app)?;
    let quit = MenuItemBuilder::with_id(MENU_QUIT, "Завершить / Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&toggle)
        .separator()
        .item(&masking)
        .separator()
        .item(&switch)
        .separator()
        .item(&settings)
        .separator()
        .item(&quit)
        .build()?;

    let icon: Image<'_> = tauri::include_image!("icons/tray.png");

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_TOGGLE => {
                // Глобальный on/off: в M0 просто логируем через меню-галочку позже.
                tracing::info!("menu: toggle");
            }
            MENU_MASKING => {
                // Приход «переключена»: применяем противоположное состояние.
                let current = app
                    .try_state::<crate::commands::AppState>()
                    .map(|st| st.config.lock().masking_on)
                    .unwrap_or(false);
                if let Err(e) = crate::commands::apply_masking(app, !current) {
                    tracing::error!("меню: маскировка: {e}");
                }
            }
            MENU_SWITCH_LAYOUT => {
                tracing::info!("menu: switch_layout");
            }
            MENU_SETTINGS => {
                show_settings(app);
            }
            MENU_QUIT => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

/// Индикатор «Маскировки»: силуэт-маска вместо обычной иконки,
/// пока режим включён (раздел 5 ТЗ).
pub fn set_masking_indicator(app: &AppHandle, on: bool) {
    let Some(tray) = app.tray_by_id("main-tray") else {
        return;
    };
    if on {
        let icon: tauri::image::Image<'_> = tauri::include_image!("icons/tray-anon.png");
        let _ = tray.set_icon(Some(icon));
        let _ = tray.set_icon_as_template(true);
    } else {
        let icon: tauri::image::Image<'_> = tauri::include_image!("icons/tray.png");
        let _ = tray.set_icon(Some(icon));
        let _ = tray.set_icon_as_template(true);
    }
    if let Some(menu) = app.menu() {
        if let Some(tauri::menu::MenuItemKind::Check(c)) = menu.get(MENU_MASKING) {
            let _ = c.set_checked(on);
        }
    }
}

/// Обновить флаг раскладки рядом с иконкой трея.
pub fn update_layout_flag(app: &AppHandle, layout_id: &str) {
    let pair = builtin_pair();
    let short = if layout_id.contains("Russian") {
        &pair.ru
    } else {
        &pair.en
    }
    .short_name
    .to_string();
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_title(Some(short));
    }
}

/// Показать окно настроек (создаётся по требованию и прячется при закрытии).
pub fn show_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let webview = tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("CarambaClone — Настройки / Settings")
    .inner_size(720.0, 560.0)
    .minimizable(false)
    .maximizable(false)
    .build();
    match webview {
        Ok(win) => {
            let _ = win.show();
            let _ = win.set_focus();
        }
        Err(e) => tracing::error!("окно настроек: {e}"),
    }
}
