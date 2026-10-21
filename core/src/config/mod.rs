//! Схема настроек, профили приложений, миграции.

use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Текущая версия схемы конфигурации.
pub const SCHEMA_VERSION: u32 = 1;

/// Тема оформления.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// По системной теме.
    #[default]
    System,
    Light,
    Dark,
}

/// Уровень агрессивности автопереключения.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Aggressiveness {
    Low,
    #[default]
    Medium,
    High,
}

/// Основные настройки приложения (config.toml).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Версия схемы — поле обязано быть в файле.
    pub schema_version: u32,
    /// Глобальный on/off движка.
    pub enabled: bool,
    /// Язык интерфейса: "system", "ru", "en".
    pub ui_lang: String,
    /// Автопереключение раскладки.
    pub autoswitch: bool,
    /// Агрессивность автопереключения.
    pub aggressiveness: Aggressiveness,
    /// Звук при автопереключении.
    pub sound_on_switch: bool,
    /// Исправление по Enter (по умолчанию выключено).
    pub fix_on_enter: bool,
    /// Умная отмена по ⌫.
    pub smart_backspace_undo: bool,
    /// Ёфикатор (по умолчанию выключен).
    pub yo_fixer: bool,
    /// Игнорировать слова с цифрами и символами.
    pub ignore_digits_symbols: bool,
    /// Автозапуск при входе.
    pub autostart: bool,
    /// Автообновление пакета «Правил».
    pub rules_autoupdate: bool,
    /// Режим --dry-run: решения печатаются в журнал, ничего не печатает.
    pub dry_run: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            enabled: true,
            ui_lang: "system".into(),
            autoswitch: true,
            aggressiveness: Aggressiveness::default(),
            sound_on_switch: false,
            fix_on_enter: false,
            smart_backspace_undo: true,
            yo_fixer: false,
            ignore_digits_symbols: true,
            autostart: false,
            rules_autoupdate: true,
            dry_run: false,
        }
    }
}

/// Миграции конфигурации от старых версий схемы.
pub fn migrate(mut cfg: Config) -> Config {
    if cfg.schema_version < 1 {
        cfg.schema_version = 1;
    }
    cfg
}

/// Атомарная запись конфигурации: временный файл + rename.
pub fn save_atomic(path: &Path, cfg: &Config) -> std::io::Result<()> {
    let tmp = path.with_extension("toml.tmp");
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    {
        let mut f = fs::File::create(&tmp)?;
        let body = toml::to_string(cfg).map_err(std::io::Error::other)?;
        f.write_all(body.as_bytes())?;
        f.flush()?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

/// Загрузка конфигурации; повреждённый файл переименовывается и применяется дефолт.
pub fn load(path: &Path) -> Config {
    let raw = match fs::read_to_string(path) {
        Ok(r) => r,
        Err(_) => return Config::default(),
    };
    match toml::from_str::<Config>(&raw) {
        Ok(cfg) => migrate(cfg),
        Err(_) => {
            // Повреждённый файл сохраняем рядом как .bak и берём значения по умолчанию.
            let bak = path.with_extension("toml.bak");
            let _ = fs::rename(path, bak);
            Config::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let cfg = Config::default();
        let body = toml::to_string(&cfg).unwrap();
        let back: Config = toml::from_str(&body).unwrap();
        assert_eq!(back, cfg);
        assert_eq!(back.schema_version, 1);
    }

    #[test]
    fn damaged_file_becomes_backup() {
        let dir = std::env::temp_dir().join(format!("ccl-cfg-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("config.toml");
        fs::write(&p, "not valid {{ toml").unwrap();
        let cfg = load(&p);
        assert_eq!(cfg, Config::default());
        assert!(p.with_extension("toml.bak").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write() {
        let dir = std::env::temp_dir().join(format!("ccl-wr-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("config.toml");
        save_atomic(&p, &Config::default()).unwrap();
        let cfg = load(&p);
        assert!(cfg.enabled);
        let _ = fs::remove_dir_all(&dir);
    }
}
