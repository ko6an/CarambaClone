//! Словари: FST-наборы слов, кэш и n-gram модель.

pub mod ngram;

use std::io::BufRead;
use std::path::Path;

use fst::Set;

/// Источник «валидных слов» — словарь языка.
pub struct WordSet {
    fst: Set<Vec<u8>>,
    count: usize,
}

impl WordSet {
    /// Строит FST из отсортированного списка слов.
    pub fn new(mut words: Vec<String>) -> Self {
        words.sort_unstable();
        words.dedup();
        let count = words.len();
        let fst = Set::from_iter(words.iter().map(|w| w.as_bytes()))
            .expect("FST из отсортированного списка не может быть некорректен");
        Self { fst, count }
    }

    /// Является ли слово (в нижнем регистре) валидным.
    pub fn is_valid(&self, word: &str) -> bool {
        let w = word.to_lowercase();
        !w.is_empty() && self.fst.contains(w.as_bytes())
    }

    /// Сколько слов в наборе.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Пуст ли набор.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Пустой набор.
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// Загрузка из текстового файла: по слову в строке.
    pub fn load_file(path: &Path) -> std::io::Result<Self> {
        let f = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(f);
        let mut words = Vec::new();
        for line in reader.lines() {
            let w = line?;
            let w = w.trim();
            if !w.is_empty() {
                words.push(w.to_string());
            }
        }
        Ok(Self::new(words))
    }
}

/// Встроенный EN-словарь (30 тыс. частотных слов + приёмочные; полные
/// пакеты приходят с «Правилами», раздел 8 ТЗ).
pub fn builtin_words_en() -> Vec<String> {
    load_words_file(include_str!("../../../data/words_en.txt"))
}

/// Встроенный RU-словарь.
pub fn builtin_words_ru() -> Vec<String> {
    load_words_file(include_str!("../../../data/words_ru.txt"))
}

fn load_words_file(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}
