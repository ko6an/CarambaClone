//! CarambaClone — точка входа.

#![cfg_attr(target_os = "macos", allow(unused_unsafe))]

mod commands;
mod platform;
mod tray;

use commands::AppState;
use parking_lot::Mutex;
use tauri::Manager;

/// Писатель в файл журнала; если файл недоступен — в никуда.
struct DebugLog(std::sync::Arc<std::sync::Mutex<Option<std::fs::File>>>);

impl DebugLog {
    fn from_file(f: std::fs::File) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(Some(f))))
    }

    fn null() -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(None)))
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for DebugLog {
    type Writer = DebugLog;

    fn make_writer(&'a self) -> Self::Writer {
        DebugLog(std::sync::Arc::clone(&self.0))
    }
}

impl std::io::Write for DebugLog {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_mut() {
            Some(f) => std::io::Write::write(f, buf),
            None => Ok(buf.len()),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let mut guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_mut() {
            Some(f) => std::io::Write::flush(f),
            None => Ok(()),
        }
    }
}

/// Каталог данных приложения.
fn app_support_dir() -> std::path::PathBuf {
    home_dir()
        .map(|h| {
            h.join(if cfg!(target_os = "macos") {
                "Library/Application Support/CarambaClone"
            } else {
                ".carambaclone"
            })
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

/// Каталог журналов.
fn logs_dir() -> std::path::PathBuf {
    home_dir()
        .map(|h| {
            h.join(if cfg!(target_os = "macos") {
                "Library/Logs/CarambaClone"
            } else {
                ".carambaclone"
            })
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn main() {
    // Лог в ~/Library/Logs/CarambaClone/debug.log (базовый уровень info).
    let log_dir = logs_dir();
    let _ = std::fs::create_dir_all(&log_dir);
    // Дописываем в конец журнала, а не перезаписываем.
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("debug.log"))
        .ok();
    let writer = match file {
        Some(f) => DebugLog::from_file(f),
        None => DebugLog::null(),
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(writer)
        .with_ansi(false)
        .init();

    let cfg = caramba_core::config::load(&app_support_dir().join("config.toml"));
    // Копия для setup (owner-клон в замыкание).
    let cfg_setup = cfg.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Второй запуск: показываем окно настроек существующей копии.
            if let Some(win) = app.get_webview_window("settings") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        .manage(AppState {
            config: Mutex::new(cfg),
            config_path: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::set_config,
            commands::app_data_dir,
            commands::permissions_status,
            commands::open_permissions_pane,
            commands::convert_current_word,
            commands::set_enabled_cmd,
            commands::set_masking_cmd
        ])
        .setup(move |app| {
            // Нет значка в Dock (только строка меню).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Движок и тап (M2): автомат, поток CGEventTap, наблюдатели.
            #[cfg(target_os = "macos")]
            {
                use crate::platform::macos;
                macos::engine_state::init(&cfg_setup);
                // Системная орфография (M3) — замыкание на главный поток.
                macos::engine_state::set_spell_checker();
                macos::event_tap::start(cfg_setup.enabled, cfg_setup.autoswitch, cfg_setup.dry_run)
                    .map_err(Box::<dyn std::error::Error>::from)?;
                macos::workers::start();
                macos::layout_notifier::start(app.handle());
                // Маскировка (M4): стартовое состояние, интенсивность,
                // язык раскладки и хоткей ⌃⌥M.
                macos::event_tap::set_masking(cfg_setup.masking_on);
                macos::event_tap::set_masking_intensity(cfg_setup.masking_intensity);
                let id0 = macos::input_source::current_input_source_id();
                macos::event_tap::set_current_lang(if id0.contains("Russian") {
                    macos::event_tap::LANG_RU
                } else {
                    macos::event_tap::LANG_EN
                });
                let toggle_handle = app.handle().clone();
                macos::event_tap::set_masking_toggle(Box::new(move |on| {
                    let h = toggle_handle.clone();
                    let h2 = h.clone();
                    let _ = h.run_on_main_thread(move || {
                        if let Err(e) = crate::commands::apply_masking(&h2, on) {
                            tracing::error!("маскировка: {e}");
                        }
                    });
                }));
                crate::tray::set_masking_indicator(app.handle(), cfg_setup.masking_on);
            }

            tray::create_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("ошибка запуска Tauri");
}

/// Каталог дома пользователя.
fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var("HOME").ok().map(std::path::PathBuf::from)
}
