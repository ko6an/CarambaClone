//! Иконка и меню в строке меню (трей).
//!
//! В M0 — минимальное меню: Включить/выключить, Пауза-заглушка, Настройки, Выход.

use caramba_core::layouts::builtin_pair;
use tauri::{
    image::Image,
    menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

/// Пункты меню, по ним находим их в обработчике.
pub const MENU_TOGGLE: &str = "toggle";
pub const MENU_MASKING: &str = "masking";
/// Подпункты паузы.
pub const MENU_PAUSE_30M: &str = "pause_30m";
pub const MENU_PAUSE_1H: &str = "pause_1h";
pub const MENU_PAUSE_FOREVER: &str = "pause_forever";
pub const MENU_PAUSE_CLEAR: &str = "pause_clear";
/// Счётчик (неактивный информационный пункт).
pub const MENU_STATS: &str = "stats";
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
    let stats = MenuItemBuilder::with_id(MENU_STATS, "Исправлено: 0 / 0")
        .enabled(false)
        .build(app)?;
    let pause = SubmenuBuilder::new(app, "Приостановить")
        .item(&MenuItemBuilder::with_id(MENU_PAUSE_30M, "На 30 минут").build(app)?)
        .item(&MenuItemBuilder::with_id(MENU_PAUSE_1H, "На 1 час").build(app)?)
        .item(&MenuItemBuilder::with_id(MENU_PAUSE_FOREVER, "До перезапуска").build(app)?)
        .item(&MenuItemBuilder::with_id(MENU_PAUSE_CLEAR, "Продолжить").build(app)?)
        .build()?;
    let quit = MenuItemBuilder::with_id(MENU_QUIT, "Завершить / Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&toggle)
        .separator()
        .item(&masking)
        .item(&pause)
        .item(&stats)
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
                crate::commands::toggle_enabled_from_menu(app);
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
                crate::commands::switch_layout_from_menu(app);
            }
            MENU_STATS => {}
            MENU_PAUSE_30M => {
                pause_items(app, crate::commands::pause_cmd(app.clone(), Some(30)));
            }
            MENU_PAUSE_1H => {
                pause_items(app, crate::commands::pause_cmd(app.clone(), Some(60)));
            }
            MENU_PAUSE_FOREVER => {
                pause_items(app, crate::commands::pause_cmd(app.clone(), None));
            }
            MENU_PAUSE_CLEAR => {
                let _ = crate::commands::pause_clear_cmd(app.clone());
                pause_items(app, Ok(()));
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

static APP_HANDLE: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();

/// Запомнить AppHandle (для потоков workers).
pub fn remember_app_handle(app: &AppHandle) {
    let _ = APP_HANDLE.set(app.clone());
}

/// Обновить пункты меню из любого потока.
pub fn update_menu_texts_threaded() {
    if let Some(app) = APP_HANDLE.get() {
        let _ = app.run_on_main_thread(move || {
            update_menu_texts(app);
        });
    }
}

/// Меню-обработчики паузы. Обновление строки с оставшимся временем
pub fn pause_items(_app: &AppHandle, _result: Result<(), String>) {
    // Текст обновляет поток workers (update_menu_texts).
}

/// Периодическое обновление пунктов меню (вызывает workers).
pub fn update_menu_texts(app: &AppHandle) {
    let Some(menu) = app.menu() else { return };
    // Счётчик исправлений (FR-SYS-3).
    let (today, total) = crate::commands::stats_numeric();
    if let Some(tauri::menu::MenuItemKind::MenuItem(m)) = menu.get(MENU_STATS) {
        let _ = m.set_text(format!("Исправлено: {today} / {total}"));
    }
    // Пауза: оставшееся время в пункте «Продолжить».
    if let Some(tauri::menu::MenuItemKind::MenuItem(m)) = menu.get(MENU_PAUSE_CLEAR) {
        #[cfg(target_os = "macos")]
        {
            let text = match crate::platform::macos::event_tap::pause_left_ms() {
                Some(ms) => {
                    let mins = ms / 60_000;
                    if mins > 0 {
                        format!("Продолжить (осталось {mins} мин)")
                    } else {
                        format!("Продолжить ({} с)", ms / 1000)
                    }
                }
                None => "Продолжить".into(),
            };
            let _ = m.set_text(text);
        }
    }
    // Иконка паузы (⏸) у трея на время паузы.
    #[cfg(target_os = "macos")]
    {
        let paused = crate::platform::macos::event_tap::is_paused();
        if let Some(tray_on) = app.tray_by_id("main-tray") {
            if paused {
                let _ = tray_on.set_title(Some(String::from("⏸")));
            }
        }
    }
}

/// Отразить состояние «вкл/выкл» в пункте меню.
pub fn refresh_enable_item(app: &AppHandle, on: bool) {
    if let Some(menu) = app.menu() {
        if let Some(tauri::menu::MenuItemKind::MenuItem(m)) = menu.get(MENU_TOGGLE) {
            let text = if on {
                "Выключить приложение / Disable"
            } else {
                "Включить приложение / Enable"
            };
            let _ = m.set_text(text);
        }
    }
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
