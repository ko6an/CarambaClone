//! Иконка и меню в строке меню (трей).
//!
//! В M0 — минимальное меню: Включить/выключить, Пауза-заглушка, Настройки, Выход.

use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

/// Пункты меню, по ним находим их в обработчике.
pub const MENU_TOGGLE: &str = "toggle";
pub const MENU_SETTINGS: &str = "settings";
pub const MENU_SWITCH_LAYOUT: &str = "switch_layout";
pub const MENU_QUIT: &str = "quit";
#[allow(dead_code)] // Используется при реализации паузы (M5)
pub const MENU_PAUSE: &str = "pause";

/// Создать трей с иконкой-шаблоном и меню.
pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle =
        MenuItemBuilder::with_id(MENU_TOGGLE, "Включить приложение / Enable").build(app)?;
    let switch =
        MenuItemBuilder::with_id(MENU_SWITCH_LAYOUT, "Сменить раскладку… / Switch layout…")
            .build(app)?;
    let settings = MenuItemBuilder::with_id(MENU_SETTINGS, "Настройки… / Settings…").build(app)?;
    let quit = MenuItemBuilder::with_id(MENU_QUIT, "Завершить / Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&toggle)
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
