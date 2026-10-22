//! Сборщик подписанного пакета «Правил» (раздел 8 ТЗ).
//!
//! Использование:
//!   rules-builder genkey                       — сгенерировать пару ключей
//!   rules-builder build --out PATH --key KEY   — собрать и подписать пакет
//!
//! Источники данных: `data/` (словари), `resources/dictionaries/yo.toml`,
//! `packages/rules/*.toml` (таблицы опечаток, исключения).
//! Секретный ключ — из переменной окружения RULES_SIGNING_KEY (hex, 32 байта)
//! либо из файла --key.

use ed25519_dalek::{Signer, SigningKey};
use sha2::Digest;
use sha2::Sha256;

use serde_json::json;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_help();
        std::process::exit(2);
    }
    match args[1].as_str() {
        "genkey" => {
            // genkey [--seed HEX] — фиксированный ключ для тестов встроенно.
            let mut seed_of_args: Option<String> = None;
            for pair in args[2..].chunks(2) {
                if pair.len() == 2 && pair[0] == "--seed" {
                    seed_of_args = Some(pair[1].to_string());
                }
            }
            genkey(seed_of_args);
        }
        "build" => {
            let mut out = PathBuf::from("rules-latest.pack");
            let mut key_hex = std::env::var("RULES_SIGNING_KEY").ok();
            for pair in args[2..].chunks(2) {
                if pair.len() == 2 && pair[0] == "--out" {
                    out = PathBuf::from(&pair[1]);
                } else if pair.len() == 2 && pair[0] == "--key" {
                    key_hex = Some(
                        std::fs::read_to_string(&pair[1])
                            .unwrap_or_default()
                            .trim()
                            .to_string(),
                    );
                }
            }
            if let Err(e) = build(out, key_hex.as_deref()) {
                eprintln!("ошибка: {e}");
                std::process::exit(1);
            }
        }
        _ => print_help(),
    }
}

fn print_help() {
    eprintln!(
        "использование:\n  rules-builder genkey\n  rules-builder build --out rules-latest.pack --key key.hex"
    );
}

fn genkey(seed_hex: Option<String>) {
    let mut secret = [0u8; 32];
    match seed_hex {
        Some(hex_str) => {
            decode_hex_into(&mut secret, hex_str.trim()).unwrap_or_else(|e| {
                eprintln!("ошибка ключа: {e}");
                std::process::exit(1);
            });
        }
        None => {
            // Генерация через системный urandom (сборщик — отдельный крейт, не core).
            if let Err(e) = read_urandom(&mut secret) {
                eprintln!("ошибка чтения urandom: {e}");
                std::process::exit(1);
            }
        }
    }
    let vk = SigningKey::from_bytes(&secret).verifying_key();
    println!("RULES_SIGNING_KEY={}", hex(&secret));
    println!("PUBLIC_KEY_HEX={}", hex(&vk.to_bytes()));
}

fn read_urandom(buf: &mut [u8]) -> std::io::Result<()> {
    use std::io::Read;
    std::fs::File::open("/dev/urandom")?.read_exact(buf)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn load_words(path: &str) -> Vec<String> {
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    raw.lines()
        .map(str::trim)
        .filter(|w| !w.is_empty() && w.chars().all(char::is_alphabetic))
        .map(str::to_lowercase)
        .collect()
}

fn build(out: PathBuf, key_hex: Option<&str>) -> Result<(), String> {
    let key_hex = key_hex.ok_or("нет ключа: задайте RULES_SIGNING_KEY или --key файл")?;
    let mut seed = [0u8; 32];
    decode_hex_into(&mut seed, key_hex.trim())?;
    let key = SigningKey::from_bytes(&seed);

    // Словари.
    let mut words_en = load_words("data/words_en.txt");
    let mut words_ru = load_words("data/words_ru.txt");

    // Ё-пары из встроенного ресурса.
    let yo_src = include_str!("../../../resources/dictionaries/yo.toml");
    let mut yo_pairs: Vec<caramba_core::rules::pack::YoPair> = Vec::new();
    let mut cur_from: Option<String> = None;
    for line in yo_src.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("from = \"") {
            cur_from = Some(v.trim_end_matches('"').to_string());
        } else if let Some(v) = line.strip_prefix("to = \"") {
            if let Some(from) = cur_from.take() {
                yo_pairs.push(caramba_core::rules::pack::YoPair {
                    from,
                    to: v.trim_end_matches('"').to_string(),
                });
            }
        }
    }

    // Курируемые опечатки и исключения приложений.
    let typos: Vec<caramba_core::rules::pack::TypoEntry> = Vec::new();
    let app_exceptions: Vec<String> = caramba_core::config::start_list_exceptions();

    // n-gram: вес = число слов с триграммой (квантование в байт).
    let mut ngram_ru: std::collections::BTreeMap<String, u8> = std::collections::BTreeMap::new();
    count_trigrams(&mut ngram_ru, &words_ru);
    let mut ngram_en = std::collections::BTreeMap::new();
    count_trigrams(&mut ngram_en, &words_en);

    words_ru.sort();
    words_ru.dedup();
    words_en.sort();
    words_en.dedup();

    let payload = caramba_core::rules::pack::PackPayload {
        words_en,
        words_ru,
        ngram_ru,
        ngram_en,
        yo_pairs,
        typos,
        app_exceptions,
    };

    let digest: [u8; 32] =
        Sha256::digest(serde_json::to_vec(&payload).map_err(|e| e.to_string())?).into();
    let manifest = caramba_core::rules::pack::PackManifest {
        version: caramba_core::rules::pack::today(),
        date: caramba_core::rules::pack::today(),
        min_app_version: "0.1.0".into(),
        format: 1,
        payload_sha256: hex(&digest),
    };
    let man_bytes = serde_json::to_vec(&manifest).map_err(|e| e.to_string())?;
    let sig = key.sign(&man_bytes);
    let pack = caramba_core::rules::pack::SignedPack {
        manifest,
        signature: hex(&sig.to_bytes()),
        payload,
    };
    let bytes = caramba_core::rules::pack::encode(&pack).map_err(|e| e.to_string())?;
    std::fs::write(&out, &bytes).map_err(|e| e.to_string())?;
    // Манифест для CI.
    let meta = json!({
        "version": pack.manifest.version,
        "date": pack.manifest.date,
        "min_app_version": pack.manifest.min_app_version,
        "payload_sha256": pack.manifest.payload_sha256,
        "signature": pack.signature,
        "bytes": bytes.len(),
    });
    println!("пакет: {}", out.display());
    println!("{meta}");
    Ok(())
}

fn count_trigrams(map: &mut std::collections::BTreeMap<String, u8>, words: &[String]) {
    let mut counts: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    for w in words {
        let lc = w.to_lowercase();
        let mut chars: Vec<char> = vec!['#'];
        chars.extend(lc.chars());
        chars.push('#');
        if chars.len() < 3 {
            continue;
        }
        let mut seen_prev: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for i in 0..chars.len() - 2 {
            let tri: String = chars[i..i + 3].iter().collect();
            seen_prev.insert(tri);
        }
        for tri in seen_prev {
            *counts.entry(tri).or_insert(0) += 1;
        }
    }
    for (tri, count) in counts {
        map.insert(tri, count.min(255) as u8);
    }
}

fn decode_hex_into(out: &mut [u8], hex_str: &str) -> Result<(), String> {
    if hex_str.len() != out.len() * 2 {
        return Err(format!(
            "ключ hex длиной {} знаков, нужно {}",
            hex_str.len(),
            out.len() * 2
        ));
    }
    for (i, pair) in hex_str.as_bytes().chunks(2).enumerate() {
        out[i] = hex_val(pair[0])? * 16 + hex_val(pair[1])?;
    }
    Ok(())
}

fn hex_val(c: u8) -> Result<u8, String> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err("не hex".into()),
    }
}
