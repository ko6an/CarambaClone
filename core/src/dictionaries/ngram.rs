//! Символьная n-gram модель (3- и 4-граммы, вес — 1 байт).
//!
//! Модель строится из словарных слов с учётом частоты; используется,
//! когда обе интерпретации токена не словарные.

use std::collections::HashMap;

/// Модель: таблица триграмм (архивированная) -> вес.
pub struct NgramModel {
    counts: HashMap<u64, u32>,
    /// Суммарный вес (для нормировки при поиске).
    total: f64,
}

impl Default for NgramModel {
    fn default() -> Self {
        Self::new()
    }
}

impl NgramModel {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
            total: 0.0,
        }
    }

    /// Строит модель по набору (слово, частота); границы моделируем через '#'.
    pub fn train(&mut self, words: &[(String, u32)]) {
        self.counts.clear();
        self.total = 0.0;
        for (word, freq) in words {
            let chars: Vec<char> = ['#'].into_iter().chain(word.chars()).chain(['#']).collect();
            for tri in windows(chars.iter().copied(), 3) {
                let k = pack3(&tri);
                *self.counts.entry(k).or_insert(0) += *freq;
                self.total += *freq as f64;
            }
        }
        // Произвольная высота: вес гистограммы фактически неограничен.
    }

    /// Средняя логарифмическая вероятность триграмм строки; неизвестные
    /// триграммы получают -9.
    pub fn score(&self, text: &str) -> f64 {
        let chars: Vec<char> = ['#'].into_iter().chain(text.chars()).chain(['#']).collect();
        if chars.len() < 3 {
            return 0.0;
        }
        let n = chars.len() - 2;
        let mut sum = 0.0;
        for tri in windows(chars.iter().copied(), 3) {
            let k = pack3(&tri);
            let seen = self.counts.get(&k).copied().unwrap_or(0);
            sum += if seen > 0 {
                (seen as f64 / self.total).ln().max(-18.0)
            } else {
                -9.0
            };
        }
        sum / n as f64
    }

    /// Модель из готовой таблицы «триграмма → вес» (пакет «Правил»).
    pub fn load_counts(map: &std::collections::BTreeMap<String, u8>) -> Self {
        let mut out = NgramModel::new();
        let mut total = 0u64;
        for (tri, w) in map {
            let chars: Vec<char> = tri.chars().collect();
            if chars.len() == 3 && *w > 0 {
                out.counts.insert(pack3(&chars), u32::from(*w));
                total += u32::from(*w) as u64;
            }
        }
        out.total = f64::from(u32::try_from(total).unwrap_or(u32::MAX));
        out
    }

    /// Встречается ли в тексте триграмма, «невозможная» для обученного языка.
    pub fn has_impossible_combo(&self, text: &str, min_weight: u32) -> bool {
        let chars: Vec<char> = ['#'].into_iter().chain(text.chars()).chain(['#']).collect();
        for tri in windows(chars.iter().copied(), 3) {
            let k = pack3(&tri);
            let seen = self.counts.get(&k).copied().unwrap_or(0);
            if seen < min_weight {
                return true;
            }
        }
        false
    }

    pub fn size(&self) -> usize {
        self.counts.len()
    }
}

/// Кодирование тройки символов в u64 (точное, без коллизий).
fn pack3(tri: &[char]) -> u64 {
    ((tri[0] as u64) << 42) | ((tri[1] as u64) << 21) | (tri[2] as u64)
}

/// Окно фиксированной ширины по срезу символов.
fn windows<I>(iter: I, n: usize) -> impl Iterator<Item = Vec<char>>
where
    I: Iterator<Item = char>,
{
    let chars: Vec<char> = iter.collect();
    let len = chars.len();
    (0..len.saturating_sub(n - 1)).map(move |i| chars[i..i + n].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mini() -> NgramModel {
        let mut m = NgramModel::new();
        m.train(&[
            ("привет".into(), 1000),
            ("пока".into(), 500),
            ("просто".into(), 300),
        ]);
        m
    }

    #[test]
    fn scored_like_statistically() {
        let m = mini();
        let good = m.score("привет");
        let bad = m.score("гхебе");
        assert!(good > bad, "good={good}, bad={bad}");
    }

    #[test]
    fn impossible_combos_detected() {
        let m = mini();
        assert!(m.has_impossible_combo("привт", 1));
        assert!(!m.has_impossible_combo("привет", 1));
    }
}
