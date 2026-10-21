//! Профили приложений (FR-EXC-3/5): режим и перекрытия функций
//! на bundle id; папка Games = Off; категория игр = Off (опция).

use caramba_core::config::{AppMode, Config};
use caramba_core::engine::EngineConfig;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::OnceLock;

/// Приложения из папки Games (обновляет поток-наблюдатель).
pub struct GamesState {
    pub ids: HashSet<String>,
}

fn games() -> &'static Mutex<GamesState> {
    static GAMES: OnceLock<Mutex<GamesState>> = OnceLock::new();
    GAMES.get_or_init(|| {
        Mutex::new(GamesState {
            ids: HashSet::new(),
        })
    })
}

/// Заменить набор игр из папки Games.
pub fn set_games(ids: HashSet<String>) {
    games().lock().ids = ids;
}

/// Играет ли папка Games против приложения.
pub fn games_hit(bundle_id: &str) -> bool {
    games().lock().ids.contains(bundle_id)
}

/// Обновить каталог игр: ~/Library/Application Support/CarambaClone/Games
/// (.app, алиас/симлинк резолвим через NSBundle — FSGames poller).
pub fn games_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(std::path::PathBuf::from(home).join("Library/Application Support/CarambaClone/Games"))
}

/// Обновить список игр из папки Games (поллинг, ≤ 1 Гц).
pub fn poll_games() {
    let Some(dir) = games_dir() else { return };
    let mut ids = HashSet::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        let _ = std::fs::create_dir_all(&dir);
        set_games(ids);
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Алиас/симлинк резолвим системно; каталог .app — bundle.
        let candidates = [
            path.clone(),
            std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone()),
        ];
        for c in candidates {
            if let Some(id) = crate::platform::macos::workspace::bundle_id_for_path_dir(&c) {
                ids.insert(id);
            }
        }
    }
    set_games(ids);
}

/// Профиль по bundle id (поддерживается суффикс-префикс `com.jetbrains.*`).
fn profile_for<'a>(
    cfg: &'a Config,
    bundle_id: &str,
) -> Option<&'a caramba_core::config::AppProfile> {
    cfg.app_profiles.iter().find(|p| {
        if let Some(prefix) = p.bundle_id.strip_suffix('*') {
            bundle_id.starts_with(prefix)
        } else {
            p.bundle_id == bundle_id
        }
    })
}

/// Режим на «каждый тик» (уже с учётом игр из папки).
pub fn effective_mode_now(bundle_id: &str, games_override: bool) -> AppMode {
    let cfg = crate::platform::macos::engine_state::config();
    if games_override || games_hit(bundle_id) {
        return AppMode::Off;
    }
    mode_for(&cfg, bundle_id)
}

/// Эффективный режим для приложения.
pub fn mode_for(cfg: &Config, bundle_id: &str) -> AppMode {
    let mut mode = AppMode::Auto;
    if cfg.exclude_games_category {
        // failed чтение категории: «cats» только из Info.plist.
        let cat = crate::platform::macos::workspace::app_category(bundle_id);
        if cat.contains("app-category.games") {
            mode = AppMode::Off;
        }
    }
    if games_hit(bundle_id) {
        mode = AppMode::Off;
    }
    if let Some(p) = profile_for(cfg, bundle_id) {
        mode = p.mode;
    }
    mode
}

/// Merged EngineConfig с перекрытиями профиля.
pub fn merged_engine_config(base: &Config, bundle_id: &str) -> EngineConfig {
    let mut patched = base.clone();
    if let Some(p) = profile_for(base, bundle_id) {
        let t = |v: caramba_core::config::OverrideBool, g: bool| match v {
            Some(x) => x,
            None => g,
        };
        patched.typos_fix = t(p.typos_fix, base.typos_fix);
        patched.autoswitch = t(p.autoswitch, base.autoswitch);
        patched.yo_fixer = t(p.yo_fixer, base.yo_fixer);
        patched.sound_on_switch = t(p.sound, base.sound_on_switch);
        if let Some(v) = p.masking {
            patched.masking_on = v;
        }
        match p.tricks {
            Some(false) => {
                patched.trick_unit_sign = false;
                patched.trick_stuck_words = false;
                patched.trick_brackets = false;
                patched.trick_trailing_signs = false;
                patched.trick_cap_excl = false;
                patched.trick_particles = false;
            }
            Some(true) => {
                patched.trick_unit_sign = true;
                patched.trick_stuck_words = true;
                patched.trick_brackets = true;
                patched.trick_trailing_signs = true;
                patched.trick_cap_excl = true;
                patched.trick_particles = true;
            }
            None => {}
        }
    }
    crate::platform::macos::engine_state::engine_config(&patched)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn режимы_по_профилям() {
        let mut cfg = Config::default();
        cfg.app_profiles.push(caramba_core::config::AppProfile {
            bundle_id: "com.jetbrains.*".into(),
            mode: AppMode::Off,
            layout_policy: String::new(),
            autoswitch: None,
            typos_fix: None,
            tricks: None,
            yo_fixer: None,
            masking: None,
            sound: None,
        });
        // Префикс с «*» сработал.
        assert_eq!(
            mode_for(&cfg, "com.jetbrains.intellij"),
            AppMode::Off,
            "префикс-профиль применяется"
        );
        assert_eq!(mode_for(&cfg, "com.apple.Notes"), AppMode::Auto);
        // Точный профиль перекрывает игры.
        cfg.app_profiles[0].bundle_id = "com.apple.Notes".into();
        set_games(["com.apple.Notes".into()].into_iter().collect());
        assert_eq!(mode_for(&cfg, "com.apple.Notes"), AppMode::Off);
    }

    #[test]
    fn перекрытия_профиля() {
        let mut cfg = Config {
            sound_on_switch: true,
            trick_particles: false,
            ..Config::default()
        };
        cfg.app_profiles.push(caramba_core::config::AppProfile {
            bundle_id: "ru.test.*".into(),
            mode: AppMode::Auto,
            layout_policy: String::new(),
            autoswitch: Some(false),
            typos_fix: Some(true),
            tricks: Some(true),
            yo_fixer: Some(true),
            masking: Some(true),
            sound: Some(false),
        });
        let ec = merged_engine_config(&cfg, "ru.test.Thing");
        // autoswitch в профиль не «печатается» в EngineConfig (ручной
        // режим контролируется тапом) — проверяем маскировку вместо.
        assert!(ec.masking_enabled, "перекрытие маскировки on");
        assert!(ec.typos_fix);
        assert!(ec.trick_particles, "tricks=true включает все фокусы");
        assert!(ec.yo_fixer);
        assert!(!ec.sound_on_switch, "перекрытие звука");
        // Без профиля — глобальные значения.
        let ec2 = merged_engine_config(&cfg, "com.apple.Notes");
        assert!(ec2.sound_on_switch);
        assert!(!ec2.trick_particles);
    }
}
