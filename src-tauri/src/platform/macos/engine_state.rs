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
    *APP_CFG.lock() = Some(cfg.clone());
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
    *engine.exceptions_mut() = Exceptions::new(cfg.exceptions.clone());
    tracing::info!("конфигурация применена к движку");
}

/// Снимок звуковых настроек для тапа (файлы и громкость).
pub fn config_snapshot() -> (Option<String>, Option<String>, f32) {
    let cfg = config();
    (cfg.sound_switch_file, cfg.sound_fix_file, cfg.sound_volume)
}

/// Текущий конфиг (клон).
pub fn config() -> Config {
    app_cfg()
}

static APP_CFG: Mutex<Option<Config>> = Mutex::new(None);

/// Сохранить состояние актуальной конфигурации (старт + каждое применение).
fn app_cfg() -> Config {
    APP_CFG.lock().clone().unwrap_or_else(|| Config {
        exceptions: caramba_core::config::start_list_exceptions(),
        ..Config::default()
    })
}

/// Применить готовый (слитый с профилем) EngineConfig.
pub fn apply_engine_config_values(engine_cfg: EngineConfig) {
    let mut engine = lock().lock();
    engine.set_config(engine_cfg);
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
        masking_enabled: cfg.masking_on,
        masking_intensity: cfg.masking_intensity,
        masking_direction: cfg.masking_direction,
        typos_fix: cfg.typos_fix,
        trick_unit_sign: cfg.trick_unit_sign,
        trick_stuck_words: cfg.trick_stuck_words,
        trick_brackets: cfg.trick_brackets,
        trick_trailing_signs: cfg.trick_trailing_signs,
        trick_cap_excl: cfg.trick_cap_excl,
        trick_particles: cfg.trick_particles,
        yo_fixer: cfg.yo_fixer,
        sound_on_switch: cfg.sound_on_switch,
        sound_on_fix: cfg.sound_on_fix,
        undo_window_ms: 2000,
    }
}

/// Применить пакет «Правил» к движку (словари, n-gram, ё, опечатки).
pub fn apply_rules_pack(payload: &caramba_core::rules::PackPayload) {
    let mut engine = lock().lock();
    engine.apply_rules_pack(payload);
}

/// Установить список исключений (настройки/пакеты Правил).
pub fn set_exceptions(words: Vec<String>) {
    let mut engine = lock().lock();
    *engine.exceptions_mut() = Exceptions::new(words);
}

/// Подключить системную орфографию (NSSpellChecker) в движок.
pub fn set_spell_checker() {
    let checker = crate::platform::macos::spell::hook();
    let mut engine = lock().lock();
    engine.set_spell_checker(checker);
}
