//! Общий экземпляр движка (создаётся один раз при запуске приложения;
//! тап, команды UI и наблюдатели работают с одним и тем же автоматом
//! под Mutex).

use caramba_core::config::{Aggressiveness, Config};
use caramba_core::engine::{Engine, EngineConfig, Exceptions};
use parking_lot::Mutex;
use std::sync::OnceLock;

static ENGINE: OnceLock<Mutex<Engine>> = OnceLock::new();

/// Инициализация (при старте приложения).
pub fn init(cfg: &Config) {
    ENGINE.get_or_init(|| {
        let mut e = Engine::new();
        e.set_config(engine_config(cfg));
        Mutex::new(e)
    });
}

/// Доступ к общему движку.
pub fn lock() -> &'static Mutex<Engine> {
    ENGINE.get().expect("движок не инициализирован")
}

/// Применить новые настройки из config.toml.
pub fn apply_config(cfg: &Config) {
    let mut engine = lock().lock();
    engine.set_config(engine_config(cfg));
    tracing::info!("конфигурация применена к движку");
}

/// Преобразование настроек приложения в настройки движка.
pub fn engine_config(cfg: &Config) -> EngineConfig {
    EngineConfig {
        min_len: match cfg.aggressiveness {
            Aggressiveness::Low => 5,
            Aggressiveness::Medium => 3,
            Aggressiveness::High => 2,
        },
        tech_token_enabled: cfg.ignore_digits_symbols,
        password_check: true,
        masking_enabled: false, // включается в M4 отдельной секцией настроек
        masking_intensity: 50,
        undo_window_ms: 2000,
    }
}

/// Установить список исключений (настройки/пакеты Правил, M3).
#[allow(dead_code)] // Подключается в M3 (правила «Правил»/исключения).
pub fn set_exceptions(words: Vec<String>) {
    let mut engine = lock().lock();
    *engine.exceptions_mut() = Exceptions::new(words);
}
