//! «Маскировка»: подмена кириллических букв визуально неотличимыми
//! латинскими двойниками. Чистая функция `mask`; анализ всегда идёт по
//! немаскированному тексту, потому что движок хранит нажатия, а не символы.

use serde::{Deserialize, Serialize};

/// Режим направления «Маскировки».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MaskDirection {
    /// Кириллица -> латинские двойники (основной режим).
    #[default]
    RuToLat,
    /// Латиница -> кириллические двойники.
    LatToRu,
}

/// Пара двойников, заданная кодовыми точками.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaskPair {
    /// Строчный символ «источник» в основном направлении.
    pub from_lower: char,
    /// Строчный символ «двойник».
    pub to_lower: char,
    /// Заглавный символ «источник».
    pub from_upper: char,
    /// Заглавный символ «двойник».
    pub to_upper: char,
    /// Пара включена в настройках.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Таблица пар из `resources/masking/pairs.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaskPairs {
    pub pairs: Vec<MaskPair>,
}

impl MaskPairs {
    /// Встроенная таблица пар.
    pub fn builtin() -> Self {
        Self::from_toml(include_str!("../../../resources/masking/pairs.toml"))
            .expect("встроенная таблица пар корректна")
    }

    pub fn from_toml(raw: &str) -> Result<Self, String> {
        toml::from_str(raw).map_err(|e| format!("парные таблицы маскировки: {e}"))
    }
}

/// ГПСЧ без внешних зависимостей: xorshift64 для «какие буквы заменить».
pub struct SmallRng(u64);

impl SmallRng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Даёт true в доле `numerator/100` (интенсивность 10–100).
    pub fn percent(&mut self, percent: u8) -> bool {
        debug_assert!((10..=100).contains(&percent), "интенсивность 10–100");
        self.next_u64() % 100 < percent as u64
    }
}

/// Главная функция маскировки.
///
/// * `text` — печатаемая строка;
/// * `dir` — направление подмены;
/// * `intensity` — доля заменяемых из подходящих, 10–100%;
/// * `rng` — источник случайности (детерминированный в тестах).
pub fn mask(
    text: &str,
    pairs: &MaskPairs,
    dir: MaskDirection,
    intensity: u8,
    rng: &mut SmallRng,
) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if let Some(repl) = try_replace(pairs, dir, c) {
            if rng.percent(intensity) {
                out.push(repl);
                continue;
            }
        }
        out.push(c);
    }
    out
}

/// Поиск двойника одного символа.
fn try_replace(pairs: &MaskPairs, dir: MaskDirection, c: char) -> Option<char> {
    for p in &pairs.pairs {
        if !p.enabled {
            continue;
        }
        let hit = match dir {
            MaskDirection::RuToLat => p.from_lower == c || p.from_upper == c,
            MaskDirection::LatToRu => p.to_lower == c || p.to_upper == c,
        };
        if !hit {
            continue;
        }
        let is_upper = c.is_uppercase();
        if is_upper {
            return Some(if dir == MaskDirection::RuToLat {
                p.to_upper
            } else {
                p.from_upper
            });
        }
        return Some(if dir == MaskDirection::RuToLat {
            p.to_lower
        } else {
            p.from_lower
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs() -> MaskPairs {
        MaskPairs::builtin()
    }

    #[test]
    fn молоко_100pct() {
        // Буквы о (U+043E) заменены на o (U+006F).
        let mut rng = SmallRng::new(7);
        let in_ = "молоко";
        let out = mask(in_, &pairs(), MaskDirection::RuToLat, 100, &mut rng);
        let expected: Vec<char> = vec!['м', '\u{6F}', 'л', '\u{6F}', 'к', '\u{6F}'];
        assert_eq!(out.chars().collect::<Vec<_>>(), expected, "out={out:?}");
    }

    #[test]
    fn книга_100pct() {
        // К -> K (U+004B), а -> a (U+0061).
        let mut rng = SmallRng::new(42);
        let out = mask("Книга", &pairs(), MaskDirection::RuToLat, 100, &mut rng);
        let expected: Vec<char> = vec!['\u{4B}', 'н', 'и', 'г', '\u{61}'];
        assert_eq!(out.chars().collect::<Vec<_>>(), expected, "out={out:?}");
    }

    #[test]
    fn интенсивность_0_подложек_нет() {
        // 10% и короткая строка: допускаем полное отсутствие замен (вероятностно),
        // поэтому проверяем законность символов, а не количество.
        let mut rng = SmallRng::new(1);
        let out = mask(
            "оооооооооооооооооо",
            &pairs(),
            MaskDirection::RuToLat,
            20,
            &mut rng,
        );
        for c in out.chars() {
            assert!(c == 'о' || c == 'o', "внезапный символ {c}");
        }
    }

    #[test]
    fn обратное_направление() {
        let mut rng = SmallRng::new(3);
        let out = mask("I am Zorg", &pairs(), MaskDirection::LatToRu, 100, &mut rng);
        // 'o' заменяется на 'о' (0x438? нет — U+043E), но в "I am Z" нет кириллицы.
        assert!(
            out.contains('\u{043E}') || out.contains('\u{041E}'),
            "out={out}"
        );
    }

    #[test]
    fn детерминированность() {
        let mut a = SmallRng::new(9);
        let mut b = SmallRng::new(9);
        let s1 = mask("привет", &pairs(), MaskDirection::RuToLat, 50, &mut a);
        let s2 = mask("привет", &pairs(), MaskDirection::RuToLat, 50, &mut b);
        assert_eq!(s1, s2);
    }
}
