//! Правила: опечатки, «фокусы», частицы, ёфикатор, маскировка.

use serde::{Deserialize, Serialize};

/// Режим направления «Маскировки».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_default() {
        assert_eq!(MaskDirection::default(), MaskDirection::RuToLat);
    }
}
