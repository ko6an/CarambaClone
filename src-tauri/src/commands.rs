//! Команды Tauri для интерфейса.

use caramba_core::config::Config;
use parking_lot::Mutex;
use tauri::State;

/// Состояние приложения в памяти.
pub struct AppState {
    pub config: Mutex<Config>,
    pub config_path: Mutex<Option<std::path::PathBuf>>,
}

/// Прочитать конфигурацию.
#[tauri::command]
pub fn get_config(state: State<AppState>) -> Config {
    state.config.lock().clone()
}

/// Сохранить конфигурацию.
#[tauri::command]
pub fn set_config(state: State<AppState>, config: Config) -> Result<(), String> {
    let path = {
        let mut guard = state.config_path.lock();
        if guard.is_none() {
            let base = dirs_app_support()?;
            let p = base.join("config.toml");
            *guard = Some(p);
        }
        guard.clone().unwrap()
    };
    caramba_core::config::save_atomic(&path, &config).map_err(|e| e.to_string())?;
    *state.config.lock() = config.clone();
    // Применяем на живой движок и тап (M2).
    #[cfg(target_os = "macos")]
    {
        crate::platform::macos::engine_state::apply_config(&config);
        crate::platform::macos::event_tap::set_enabled(config.enabled);
        crate::platform::macos::event_tap::set_autoswitch(config.autoswitch);
        crate::platform::macos::event_tap::set_dry_run(config.dry_run);
    }
    Ok(())
}

/// Базовый каталог данных приложения.
fn dirs_app_support() -> Result<std::path::PathBuf, String> {
    // ~/Library/Application Support/CarambaClone
    let home = std::env::var("HOME").map_err(|_| "нет HOME".to_string())?;
    let mac_dir = if cfg!(target_os = "macos") {
        std::path::PathBuf::from(home).join("Library/Application Support/CarambaClone")
    } else {
        std::path::PathBuf::from(home).join(".carambaclone")
    };
    Ok(mac_dir)
}

/// Статус системных разрешений (M2).
#[derive(Clone, serde::Serialize)]
#[allow(dead_code)]
pub struct PermissionsStatus {
    /// Accessibility доверен.
    pub accessibility: bool,
    /// Input Monitoring (слушать события) разрешён.
    pub input_monitoring: bool,
    /// Право на генерацию событий (posting).
    pub posting: bool,
    /// Secure Input активен на системе.
    pub secure_input: bool,
    /// Тап реально работает (создан).
    pub tap_active: bool,
}

/// Статус разрешений для мастера (M2).
#[tauri::command]
pub fn permissions_status() -> PermissionsStatus {
    #[cfg(target_os = "macos")]
    {
        PermissionsStatus {
            accessibility: crate::platform::macos::ax::is_accessibility_trusted(),
            input_monitoring: crate::platform::macos::ax::listen_event_access(),
            posting: crate::platform::macos::ax::post_event_access(),
            secure_input: crate::platform::macos::ax::is_secure_event_input(),
            tap_active: crate::platform::macos::event_tap::tap_is_active(),
        }
    }
    #[cfg(not(target_os = "macos"))]
    PermissionsStatus {
        accessibility: true,
        input_monitoring: true,
        posting: true,
        secure_input: false,
        tap_active: true,
    }
}

/// Открыть нужную панель системных настроек.
#[tauri::command]
pub fn open_permissions_pane(pane: String) -> Result<(), String> {
    let url = match pane.as_str() {
        "accessibility" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        }
        "input_monitoring" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
        }
        "autostart" | "login_items" => {
            "x-apple.systempreferences:com.apple.LoginItems-Settings.extension"
        }
        other => return Err(format!("неизвестная панель: {other}")),
    };
    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Ручная конвертация текущего слова (кнопка в мастере/тесте).
#[tauri::command]
pub fn convert_current_word() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use caramba_core::engine::Event;
        let now = crate::platform::macos::event_tap::now_ms();
        let mut engine = crate::platform::macos::engine_state::lock().lock();
        let actions = engine.evaluate_current_word(now);
        drop(engine);
        crate::platform::macos::event_tap::execute(&actions);
        let _ = Event::LayoutChanged {
            new_layout: String::new(),
        };
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    Err("нет macOS".into())
}

/// Тумблер основного режима (меню/мастер).
#[tauri::command]
pub fn set_enabled_cmd(state: State<AppState>, on: bool) -> Result<(), String> {
    state.config.lock().enabled = on;
    #[cfg(target_os = "macos")]
    crate::platform::macos::event_tap::set_enabled(on);
    Ok(())
}

/// Выдать путь каталога данных интерфейсу (для диагностики).
#[tauri::command]
pub fn app_data_dir(state: State<AppState>) -> String {
    let path = {
        let mut guard = state.config_path.lock();
        if guard.is_none() {
            match dirs_app_support() {
                Ok(base) => {
                    let p = base.join("config.toml");
                    *guard = Some(p.clone());
                    p
                }
                Err(_) => return String::new(),
            }
        } else {
            guard.clone().unwrap()
        }
    };
    path.parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default()
}
