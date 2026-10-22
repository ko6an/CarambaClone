//! Клиент обновлений пакета «Правил» (раздел 8 ТЗ).
//!
//! Скачивает подписанный пакет из GitHub Releases, верифицирует
//! подпись и SHA-256, атомарно подменяет файл
//! `~/Library/Application Support/CarambaClone/rules/rules.pack`.
//! Любая ошибка — приложение остаётся на встроенном пакете.
//! Единственный сетевой запрос — скачивание файла; автообновление
//! отключается настройкой `rules_autoupdate`.

use caramba_core::rules::pack::{self, SignedPack};

/// Репозиторий релизов (плейсхолдер владельца — D18, DECISIONS.md).
pub const GITHUB_OWNER: &str = "GITHUB_OWNER";
pub const GITHUB_REPO: &str = "CarambaClone";
pub const ASSET_NAME: &str = "rules-latest.pack";
const CHECK_INTERVAL_SECS: u64 = 7 * 24 * 60 * 60;

/// URL свежего пакета из GitHub Releases.
pub fn latest_url() -> String {
    format!(
        "https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases/download/rules-latest/{ASSET_NAME}"
    )
}

/// Каталог с установленным пакетом.
pub fn rules_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(std::path::PathBuf::from(home).join("Library/Application Support/CarambaClone/rules"))
}

fn installed_path() -> Option<std::path::PathBuf> {
    rules_dir().map(|d| d.join("rules.pack"))
}

fn last_check_path() -> Option<std::path::PathBuf> {
    rules_dir().map(|d| d.join("last_check"))
}

/// Установленный пакет (None — встроенный).
pub fn load_installed() -> Option<SignedPack> {
    let path = installed_path()?;
    let bytes = std::fs::read(&path).ok()?;
    pack::decode_verified(&bytes).ok()
}

/// Метаданные текущего «Правил» (для вкладки «Правила»).
#[derive(Clone, serde::Serialize)]
pub struct RulesInfo {
    /// «2026-10-22» — установлено; «builtin» — встроенный пакет.
    pub version: String,
    pub date: String,
    pub installed: bool,
    pub words_ru: usize,
    pub words_en: usize,
    pub app_exceptions: usize,
    pub last_check_secs_ago: Option<u64>,
}

pub fn info() -> RulesInfo {
    match load_installed() {
        Some(p) => RulesInfo {
            version: p.manifest.version.clone(),
            date: p.manifest.date.clone(),
            installed: true,
            words_ru: p.payload.words_ru.len(),
            words_en: p.payload.words_en.len(),
            app_exceptions: p.payload.app_exceptions.len(),
            last_check_secs_ago: last_check_secs_ago(),
        },
        None => RulesInfo {
            version: "builtin".into(),
            date: String::new(),
            installed: false,
            words_ru: caramba_core::dictionaries::builtin_words_ru().len(),
            words_en: caramba_core::dictionaries::builtin_words_en().len(),
            app_exceptions: caramba_core::config::start_list_exceptions().len(),
            last_check_secs_ago: last_check_secs_ago(),
        },
    }
}

fn last_check_secs_ago() -> Option<u64> {
    let path = last_check_path()?;
    let raw = std::fs::read_to_string(path).ok()?;
    let ts: u64 = raw.trim().parse().ok()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(now.saturating_sub(ts))
}

fn touch_last_check() {
    if let Some(dir) = rules_dir() {
        let _ = std::fs::create_dir_all(&dir);
        if let Some(path) = last_check_path() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = std::fs::write(path, now.to_string());
        }
    }
}

/// Нужна ли периодическая проверка (раз в неделю при запуске).
pub fn check_due() -> bool {
    last_check_secs_ago()
        .map(|a| a >= CHECK_INTERVAL_SECS)
        .unwrap_or(true)
}

/// Скачивание пакета во временный буфер (curl — системный HTTPS-клиент).
fn fetch_bytes() -> Result<Vec<u8>, String> {
    let url = latest_url();
    let out = std::process::Command::new("curl")
        .args(["-fsSL", "--max-time", "60", &url])
        .output()
        .map_err(|e| format!("curl запуск: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "curl: код {} (нет сети или релиза)",
            out.status.code().unwrap_or(-1)
        ));
    }
    Ok(out.stdout)
}

pub enum ApplyError {
    Fetch(String),
    Verify(String),
    Io(String),
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplyError::Fetch(e) => write!(f, "сеть: {e}"),
            ApplyError::Verify(e) => write!(f, "верификация: {e}"),
            ApplyError::Io(e) => write!(f, "диск: {e}"),
        }
    }
}

/// Скачать, проверить и атомарно установить пакет; применить к движку.
pub fn check_and_install() -> Result<RulesInfo, ApplyError> {
    let bytes = fetch_bytes().map_err(ApplyError::Fetch)?;
    let verified = pack::decode_verified(&bytes).map_err(|e| ApplyError::Verify(e.to_string()))?;
    if verified.manifest.version.is_empty() {
        return Err(ApplyError::Verify("пустая версия пакета".into()));
    }
    touch_last_check();
    // Атомарная подмена: tmp → rename.
    let dir = rules_dir().ok_or(ApplyError::Io("нет HOME".into()))?;
    std::fs::create_dir_all(&dir).map_err(|e| ApplyError::Io(e.to_string()))?;
    let final_path = dir.join("rules.pack");
    let tmp = dir.join("rules.pack.tmp");
    std::fs::write(&tmp, &bytes).map_err(|e| ApplyError::Io(e.to_string()))?;
    std::fs::rename(&tmp, &final_path).map_err(|e| ApplyError::Io(e.to_string()))?;
    // Применить к движку немедленно.
    apply_to_engine(&verified);
    Ok(info())
}

/// Удалить установленный пакет: «откат на встроенный пакет».
pub fn rollback_to_builtin() -> Result<RulesInfo, ApplyError> {
    if let Some(path) = installed_path() {
        std::fs::remove_file(&path).map_err(|e| ApplyError::Io(e.to_string()))?;
    }
    Ok(info())
}

/// Применить содержимое пакета к общему движку.
pub fn apply_to_engine(verified: &SignedPack) {
    crate::platform::macos::engine_state::apply_rules_pack(&verified.payload);
    tracing::info!(
        "пакет Правил {} применён: слов RU {}, EN {}",
        verified.manifest.version,
        verified.payload.words_ru.len(),
        verified.payload.words_en.len()
    );
}
