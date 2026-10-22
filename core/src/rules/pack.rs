//! Пакет «Правил»: контейнер `rules-YYYY-MM-DD.pack` (раздел 8 ТЗ).
//!
//! Формат файла (бинарный контейнер):
//! ```text
//! magic   — 8 байт: "CBCRULES" (версия формата — в поле `format` манифеста)
//! man_len — u32 LE: длина манифеста
//! сбоку   — манифест (JSON, канонические байты — ровно то, что подписано)
//! payload — сериализованный JSON содержимого
//! ```
//!
//! Подпись Ed25519 считается по байтам манифеста; манифест содержит
//! SHA-256 содержимого. Публичный ключ вшит в приложение
//! (константа `PUBLIC_KEY_HEX`), приватный — секрет сборки
//! `RULES_SIGNING_KEY` (только в `tools/rules-builder`).

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Магия + версия формата контейнера.
pub const MAGIC: &[u8; 8] = b"CBCRULES";

/// Вшитый публичный ключ подписанта пакета (Ed25519, 32 байта, hex).
/// Для локальных сборок разработчика ключ тестовый; прод-ключ
/// подставляется при выпуске в релиз (допущение D17, DECISIONS.md).
pub const PUBLIC_KEY_HEX: &str = "79b5562e8fe654f94078b112e8a98ba7901f853ae695bed7e0e3910bad049664";

/// Тестовый приватный ключ (seed 0x01..0x20) — ТОЛЬКО для тестов и
/// локальной отладки; прод-ключ выдаётся секретом сборки.
#[allow(dead_code)]
pub const TEST_SIGNING_KEY_HEX: &str =
    "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";

pub const MAX_PACK_BYTES: usize = 64 * 1024 * 1024;

/// Манифест пакета: версия, дата, требования к приложению, хеш содержимого.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackManifest {
    /// Версия пакета («YYYY-MM-DD» — дата сборки данных).
    pub version: String,
    /// Дата сборки (ISO).
    pub date: String,
    /// Минимальная версия приложения.
    pub min_app_version: String,
    /// Идентификатор формата содержимого.
    pub format: u32,
    /// SHA-256 содержимого (payload), hex.
    pub payload_sha256: String,
}

/// Содержимое пакета: словари, n-gram, таблицы правил, исключения.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PackPayload {
    /// Словарь EN (по слову в строке).
    pub words_en: Vec<String>,
    /// Словарь RU.
    pub words_ru: Vec<String>,
    /// Модель RU: триграмма → вес (байт).
    pub ngram_ru: std::collections::BTreeMap<String, u8>,
    /// Модель EN.
    pub ngram_en: std::collections::BTreeMap<String, u8>,
    /// Таблица ёфикатора (однозначные пары).
    pub yo_pairs: Vec<YoPair>,
    /// Курируемая таблица опечаток «ошибка → правильно».
    pub typos: Vec<TypoEntry>,
    /// Стартовый список программ-исключений (bundle id, FR-EXC-4).
    pub app_exceptions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct YoPair {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypoEntry {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignedPack {
    pub manifest: PackManifest,
    /// Ed25519-подпись манифеста, hex.
    pub signature: String,
    pub payload: PackPayload,
}

/// Ошибки пакета.
#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("файл меньше заголовка илиamaged")]
    Truncated,
    #[error("магия не совпала: не пакет «Правил»")]
    BadMagic,
    #[error("манифест не разобран: {0}")]
    ManifestJson(String),
    #[error("содержимое не разобрано: {0}")]
    PayloadJson(String),
    #[error("файл превышает лимит {0} байт")]
    TooBig(usize),
    #[error("подпись не верифицирована")]
    BadSignature,
    #[error("публичный ключ не разобран")]
    BadKey,
    #[error("SHA-256 содержимого не совпал")]
    HashMismatch,
    #[error("вышедший из поддержки пакет: требуется приложение ≥ {0}")]
    TooOld(String),
}

fn parse_public_key() -> Result<VerifyingKey, PackError> {
    let mut key = [0u8; 32];
    for (i, pair) in PUBLIC_KEY_HEX.as_bytes().chunks(2).enumerate() {
        key[i] = hex_val(pair[0])? * 16 + hex_val(pair[1])?;
    }
    VerifyingKey::from_bytes(&key).map_err(|_| PackError::BadKey)
}

fn hex_val(c: u8) -> Result<u8, PackError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(PackError::BadKey),
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    let digest = h.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Контейнер: [magic 8][man_len u32 LE][manifest][sign 64][payload].
pub fn encode(pack: &SignedPack) -> Result<Vec<u8>, PackError> {
    let man =
        serde_json::to_vec(&pack.manifest).map_err(|e| PackError::ManifestJson(e.to_string()))?;
    let payload =
        serde_json::to_vec(&pack.payload).map_err(|e| PackError::PayloadJson(e.to_string()))?;
    let mut sig = [0u8; 64];
    let sig_hex = pack.signature.as_bytes();
    if sig_hex.len() != 128 {
        return Err(PackError::BadSignature);
    }
    for i in 0..64 {
        sig[i] = hex_val(sig_hex[i * 2])? * 16 + hex_val(sig_hex[i * 2 + 1])?;
    }
    let mut out = Vec::with_capacity(12 + man.len() + 64 + payload.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(man.len() as u32).to_le_bytes());
    out.extend_from_slice(&man);
    out.extend_from_slice(&sig);
    out.extend_from_slice(&payload);
    Ok(out)
}

/// Разбор + верификация подписи и SHA-256 содержимого.
pub fn decode_verified(bytes: &[u8]) -> Result<SignedPack, PackError> {
    if bytes.len() < 12 {
        return Err(PackError::Truncated);
    }
    if &bytes[0..8] != MAGIC {
        return Err(PackError::BadMagic);
    }
    if bytes.len() > MAX_PACK_BYTES {
        return Err(PackError::TooBig(MAX_PACK_BYTES));
    }
    let man_len =
        u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| PackError::Truncated)?) as usize;
    let sig_start = 12_usize.checked_add(man_len).ok_or(PackError::Truncated)?;
    let payload_start = sig_start.checked_add(64).ok_or(PackError::Truncated)?;
    if payload_start > bytes.len() {
        return Err(PackError::Truncated);
    }
    let man_bytes = &bytes[12..sig_start];
    let sig_bytes = &bytes[sig_start..payload_start];
    let payload_bytes = &bytes[payload_start..];
    let manifest: PackManifest =
        serde_json::from_slice(man_bytes).map_err(|e| PackError::ManifestJson(e.to_string()))?;
    let payload: PackPayload =
        serde_json::from_slice(payload_bytes).map_err(|e| PackError::PayloadJson(e.to_string()))?;
    if manifest.format != 1 {
        return Err(PackError::TooOld(manifest.min_app_version.clone()));
    }
    if sha256_hex(payload_bytes) != manifest.payload_sha256 {
        return Err(PackError::HashMismatch);
    }
    let vk = parse_public_key()?;
    let signature = Signature::from_bytes(
        &sig_bytes[..64]
            .try_into()
            .map_err(|_| PackError::Truncated)?,
    );
    vk.verify(man_bytes, &signature)
        .map_err(|_| PackError::BadSignature)?;
    Ok(SignedPack {
        manifest,
        signature: hex_sig(sig_bytes),
        payload,
    })
}

fn hex_sig(sig: &[u8]) -> String {
    sig.iter().map(|b| format!("{b:02x}")).collect()
}

/// Каноническая дата «YYYY-MM-DD» для версии/даты пакета.
pub fn today() -> String {
    // Без chrono: UTC-дата из Unix времени.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86_400;
    let civil = civil_from_days(days as i64);
    format!("{:04}-{:02}-{:02}", civil.0, civil.1, civil.2)
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z2 = z + 719_468;
    let era = z2.div_euclid(146_097);
    let doe = (z2 - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;

    fn test_signing() -> ed25519_dalek::SigningKey {
        let mut seed = [0u8; 32];
        for (i, pair) in TEST_SIGNING_KEY_HEX.as_bytes().chunks(2).enumerate() {
            seed[i] = hex_val(pair[0]).expect("hex") * 16 + hex_val(pair[1]).expect("hex");
        }
        ed25519_dalek::SigningKey::from_bytes(&seed)
    }

    fn sample_payload() -> PackPayload {
        let mut ngram = std::collections::BTreeMap::new();
        ngram.insert("#ат".to_string(), 3u8);
        ngram.insert("ата".to_string(), 2u8);
        PackPayload {
            words_en: vec!["hello".into(), "world".into()],
            words_ru: vec!["привет".into()],
            ngram_ru: ngram.clone(),
            ngram_en: ngram,
            yo_pairs: vec![YoPair {
                from: "еще".into(),
                to: "ещё".into(),
            }],
            typos: vec![TypoEntry {
                from: "тесет".into(),
                to: "тест".into(),
            }],
            app_exceptions: vec!["com.apple.Terminal".into()],
        }
    }

    fn signed_sample() -> SignedPack {
        let payload = sample_payload();
        let digest = sha256_hex(&serde_json::to_vec(&payload).expect("сериализация payload"));
        let manifest = PackManifest {
            version: "2026-10-21".into(),
            date: "2026-10-21".into(),
            min_app_version: "0.1.0".into(),
            format: 1,
            payload_sha256: digest,
        };
        let man_bytes = serde_json::to_vec(&manifest).expect("сериализация манифеста");
        let sig = test_signing().sign(&man_bytes);
        SignedPack {
            manifest,
            signature: hex_sig(&sig.to_bytes()),
            payload,
        }
    }

    #[test]
    fn контейнер_круговая_поездка() {
        let bytes = encode(&signed_sample()).expect("кодирование");
        let pack = decode_verified(&bytes).expect("верификация");
        assert_eq!(pack.payload, sample_payload());
        assert_eq!(pack.manifest.version, "2026-10-21");
        // Повреждение содержимого — отказ (хеш/подпись/разбор JSON).
        let mut bad = bytes.clone();
        let last = bad.len() - 1;
        bad[last] ^= 0x01;
        assert!(
            decode_verified(&bad).is_err(),
            "изменённый хвост отвергается"
        );
        // Подмена манифеста (другой payload такого же размера).
        let mut bad2 = bytes.clone();
        let first_payload = bad2.len() - 7;
        bad2[first_payload] ^= 0x01;
        assert!(
            decode_verified(&bad2).is_err(),
            "изменённый payload отвергается"
        );
    }

    #[test]
    fn магия_отвергается() {
        let bytes = encode(&signed_sample()).expect("кодирование");
        let mut bad = bytes.clone();
        bad[0] = b'X';
        assert!(matches!(decode_verified(&bad), Err(PackError::BadMagic)));
        let short = &bytes[..5];
        assert!(matches!(decode_verified(short), Err(PackError::Truncated)));
    }

    #[test]
    fn чужая_подпись_отвергается() {
        let mut pack = signed_sample();
        // Подпись «мусором» (не ключом-парой к вшитому публичному).
        pack.signature = "ff".repeat(64);
        let bytes = encode(&pack).expect("кодирование");
        assert!(matches!(
            decode_verified(&bytes),
            Err(PackError::BadSignature)
        ));
    }

    #[test]
    fn дата_формата_iso() {
        assert!(today().len() == 10 && today().split('-').count() == 3);
    }

    // Прикреплениеapply_rules_pack живёт в engine/auto.rs (тесты там).
}
