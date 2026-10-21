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
    *state.config.lock() = config;
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
