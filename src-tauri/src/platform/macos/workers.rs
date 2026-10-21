//! Наблюдатель Secure Input (опрос ≤ 4 Гц) и сторож тапа.

use super::{ax, engine_state, event_tap, ffi};
use std::time::Duration;

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
