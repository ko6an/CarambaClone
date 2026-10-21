//! Команды Tauri для интерфейса.

use caramba_core::config::Config;
use parking_lot::Mutex;
use tauri::{Manager, State};

/// Снимок настроек звука для тапа (файлы и громкость).
pub fn sound_config_snapshot() -> (Option<String>, Option<String>, f32) {
    #[cfg(target_os = "macos")]
    {
        let cfg = crate::platform::macos::engine_state::config_snapshot();
        (cfg.0, cfg.1, cfg.2)
    }
    #[cfg(not(target_os = "macos"))]
    (None, None, 0.7)
}

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

/// Применить состояние «Маскировки» ко всем подсистемам
/// (меню, хоткей ⌃⌥M, настройки). Сохраняем конфиг, если включено
/// «Запоминать» (FR: состояние не сохраняется между запусками,
/// пока не включено «Запоминать»).
pub fn apply_masking(app: &tauri::AppHandle, on: bool) -> Result<(), String> {
    let (saved_remember, intensity) = {
        let st: tauri::State<AppState> = app.state();
        let mut cfg = st.config.lock();
        cfg.masking_on = on;
        (cfg.masking_remember, cfg.masking_intensity)
    };
    #[cfg(target_os = "macos")]
    {
        crate::platform::macos::event_tap::set_masking(on);
        crate::platform::macos::event_tap::set_masking_intensity(intensity);
        let cfg = app.state::<AppState>().config.lock().clone();
        crate::platform::macos::engine_state::apply_config(&cfg);
    }
    crate::tray::set_masking_indicator(app, on);
    if saved_remember {
        let path = {
            let st: tauri::State<AppState> = app.state();
            let mut guard = st.config_path.lock();
            if guard.is_none() {
                let base = dirs_app_support()?;
                *guard = Some(base.join("config.toml"));
            }
            guard.clone().unwrap()
        };
        let cfg = app.state::<AppState>().config.lock().clone();
        caramba_core::config::save_atomic(&path, &cfg).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// MARK: — Автозапуск (FR-SYS-5), пауза, счётчик

/// Статус автозапуска: not_registered / requires_approval / enabled / not_found.
#[derive(Clone, serde::Serialize)]
pub struct AutostartStatus {
    pub status: String,
    pub registered: bool,
}

#[cfg(target_os = "macos")]
fn sm_service() -> objc2::rc::Retained<objc2_service_management::SMAppService> {
    unsafe { objc2_service_management::SMAppService::mainAppService() }
}

#[cfg(target_os = "macos")]
fn sm_status_name(s: objc2_service_management::SMAppServiceStatus) -> String {
    use objc2_service_management::SMAppServiceStatus;
    if s == SMAppServiceStatus::Enabled {
        "enabled".into()
    } else if s == SMAppServiceStatus::RequiresApproval {
        "requires_approval".into()
    } else if s == SMAppServiceStatus::NotRegistered {
        "not_registered".into()
    } else if s == SMAppServiceStatus::NotFound {
        "not_found".into()
    } else {
        format!("other({})", s.0)
    }
}

/// Текущий статус автозапуска.
#[tauri::command]
pub fn autostart_status() -> AutostartStatus {
    #[cfg(target_os = "macos")]
    {
        let svc = sm_service();
        let st = unsafe { svc.status() };
        AutostartStatus {
            status: sm_status_name(st),
            registered: st == objc2_service_management::SMAppServiceStatus::Enabled,
        }
    }
    #[cfg(not(target_os = "macos"))]
    AutostartStatus {
        status: "unsupported".into(),
        registered: false,
    }
}

/// Включить/выключить автозапуск.
#[tauri::command]
pub fn autostart_set(state: State<AppState>, on: bool) -> Result<(), String> {
    state.config.lock().autostart = on;
    #[cfg(target_os = "macos")]
    {
        let svc = sm_service();
        if on {
            // SAFETY: константа статуса системы.
            let err = unsafe { svc.registerAndReturnError() };
            if let Err(e) = err {
                return Err(format!("SMAppService.register: {e:?}"));
            }
        } else {
            let err = unsafe { svc.unregisterAndReturnError() };
            if let Err(e) = err {
                return Err(format!("SMAppService.unregister: {e:?}"));
            }
        }
    }
    save_config(state)?;
    Ok(())
}

/// Сохранить конфиг на диск по текущему пути.
fn save_config(state: State<AppState>) -> Result<(), String> {
    let path = {
        let mut guard = state.config_path.lock();
        if guard.is_none() {
            let base = dirs_app_support()?;
            *guard = Some(base.join("config.toml"));
        }
        guard.clone().unwrap()
    };
    let cfg = state.config.lock().clone();
    caramba_core::config::save_atomic(&path, &cfg).map_err(|e| e.to_string())
}

/// Пауза (меню/команда): minutes=None — до перезапуска.
#[tauri::command]
pub fn pause_cmd(app: tauri::AppHandle, minutes: Option<u64>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = crate::platform::macos::event_tap::set_pause(minutes);
    }
    let _ = app;
    Ok(())
}

/// Снять паузу.
#[tauri::command]
pub fn pause_clear_cmd(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    crate::platform::macos::event_tap::clear_pause();
    let _ = app;
    Ok(())
}

/// Числовой счётчик для обновления меню.
pub fn stats_numeric() -> (u32, u32) {
    #[cfg(target_os = "macos")]
    return crate::platform::macos::event_tap::stats_get();
    #[cfg(not(target_os = "macos"))]
    (0, 0)
}

/// Счётчик исправлений (сегодня / всего).
#[tauri::command]
pub fn stats_cmd() -> (u32, u32) {
    #[cfg(target_os = "macos")]
    return crate::platform::macos::event_tap::stats_get();
    #[cfg(not(target_os = "macos"))]
    (0, 0)
}

/// Установить список слов-исключений (вкладка «Словарь»).
#[tauri::command]
pub fn exceptions_set_cmd(appstate: State<AppState>, words: Vec<String>) -> Result<(), String> {
    appstate.config.lock().exceptions = words.clone();
    #[cfg(target_os = "macos")]
    crate::platform::macos::engine_state::set_exceptions(words);
    save_config(appstate)?;
    Ok(())
}

/// Установить профили приложений (вкладка «Приложения»).
#[tauri::command]
pub fn app_profiles_set_cmd(
    state: State<AppState>,
    profiles: Vec<caramba_core::config::AppProfile>,
) -> Result<(), String> {
    let mut cfg_g = state.config.lock();
    cfg_g.app_profiles = profiles;
    drop(cfg_g);
    save_config(state)?;
    Ok(())
}

/// Вернуть стартовый список исключений (FR-EXC-4 «Вернуть стандартный»).
#[tauri::command]
pub fn exceptions_default_cmd() -> Vec<String> {
    caramba_core::config::start_list_exceptions()
}

/// Открыть папку Games (FR-EXC-5).
#[tauri::command]
pub fn open_games_folder() -> Result<(), String> {
    let base = dirs_app_support()?;
    let dir = base.join("Games");
    let _ = std::fs::create_dir_all(&dir);
    std::process::Command::new("open")
        .arg(&dir)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Текущий язык интерфейса (для перерисовки вкладок).
#[tauri::command]
pub fn get_ui_lang(state: State<AppState>) -> String {
    state.config.lock().ui_lang.clone()
}

/// Открыть журнал (Настройки → Диагностика, FR-SYS-7).
#[tauri::command]
pub fn open_log_cmd() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let path = {
        let home = std::env::var("HOME").map_err(|_| "нет HOME".to_string())?;
        std::path::PathBuf::from(home).join("Library/Logs/CarambaClone/debug.log")
    };
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    Err("нет macOS".into())
}

/// Диагностика: подробный журнал на время (M5: toggle в конфиге).
#[tauri::command]
pub fn set_verbose_logging(state: State<AppState>, on: bool) -> Result<(), String> {
    state.config.lock().dry_run = on;
    let _ = on;
    save_config(state)
}

/// Тумблер «Маскировки» (команда интерфейса).
#[tauri::command]
pub fn set_masking_cmd(app: tauri::AppHandle, on: bool) -> Result<(), String> {
    apply_masking(&app, on)
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
