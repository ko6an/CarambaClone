//! Наблюдатели: Secure Input (≤ 4 Гц), сторож тапа, контекст
//! приложения (профили FR-EXC-3/5) и папка Games.

use super::{ax, engine_state, event_tap, ffi, profiles, workspace};
use caramba_core::engine::Event;
use std::time::Duration;

/// Применить профиль/перекрытия для приложения (смена активного окна).
fn apply_app_context(bundle_id: &str) {
    let cfg = engine_state::config();
    let mode = profiles::mode_for(&cfg, bundle_id);
    let engine_cfg = profiles::merged_engine_config(&cfg, bundle_id);
    engine_state::apply_engine_config_values(engine_cfg);
    event_tap::set_app_mode(match mode {
        caramba_core::config::AppMode::Off => event_tap::APP_MODE_OFF,
        caramba_core::config::AppMode::ManualOnly => event_tap::APP_MODE_MANUAL_ONLY,
        caramba_core::config::AppMode::Auto => event_tap::APP_MODE_AUTO,
    });
}

/// Запустить фоновые наблюдатели: Secure Input и сторож тапа.
pub fn start() {
    // Secure Input: опрос раз в 250 мс (≤ 4 Гц).
    let secure_sh = event_tap::shared();
    std::thread::Builder::new()
        .name("caramba-secure".into())
        .spawn(move || {
            let sh = secure_sh;
            let mut was = sh.secure.load(std::sync::atomic::Ordering::SeqCst);
            loop {
                std::thread::sleep(Duration::from_millis(250));
                let now = ax::is_secure_event_input();
                if now != was {
                    tracing::warn!(secure = now, "Secure Input изменился");
                    was = now;
                    sh.secure.store(now, std::sync::atomic::Ordering::SeqCst);
                    if now {
                        // Вход в Secure Input: буфер сбросить, не конвертировать.
                        let mut engine = engine_state::lock().lock();
                        engine.on_event(
                            caramba_core::engine::Event::AppChanged {
                                bundle_id: String::new(),
                            },
                            event_tap::now_ms(),
                        );
                    }
                }
            }
        })
        .expect("поток secure");

    // Контекст приложения: профили/игры, смена приложения.
    std::thread::Builder::new()
        .name("caramba-appctx".into())
        .spawn(move || {
            let mut current = String::new();
            let mut tick = 0u64;
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let bundle = workspace::frontmost_bundle_id();
                if bundle != current {
                    current = bundle.clone();
                    apply_app_context(&current);
                    // Смена приложения: буфер сбросить (раздел 3.1).
                    let mut engine = engine_state::lock().lock();
                    engine.on_event(
                        Event::AppChanged {
                            bundle_id: current.clone(),
                        },
                        event_tap::now_ms(),
                    );
                }
                tick += 1;
                if tick.is_multiple_of(60) {
                    // Пункты меню: счётчик и оставшееся время паузы.
                    crate::tray::update_menu_texts_threaded();
                }
                if tick.is_multiple_of(10) {
                    // Папка Games: поллинг ~1 раз в 5 с.
                    profiles::poll_games();
                    let games_hit = profiles::games_hit(&current);
                    let mode = profiles::effective_mode_now(&current, games_hit);
                    event_tap::set_app_mode(match mode {
                        caramba_core::config::AppMode::Off => event_tap::APP_MODE_OFF,
                        caramba_core::config::AppMode::ManualOnly => {
                            event_tap::APP_MODE_MANUAL_ONLY
                        }
                        caramba_core::config::AppMode::Auto => event_tap::APP_MODE_AUTO,
                    });
                }
            }
        })
        .expect("поток контекста приложений");

    // Сторож тапа: раз в секунду проверяем и включаем заново.
    let wd_sh = event_tap::shared();
    std::thread::Builder::new()
        .name("caramba-watchdog".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_millis(1000));
            if let Some(port) = *wd_sh.tap_port.lock() {
                let enabled = unsafe { ffi::CGEventTapIsEnabled(port.0) };
                if !enabled {
                    unsafe { ffi::CGEventTapEnable(port.0, true) };
                    tracing::warn!("сторож: тап включён заново");
                }
            }
        })
        .expect("поток сторожа");
}
