//! Таблицы раскладок и конвертация между парами.

use serde::{Deserialize, Serialize};

/// Запись таблицы: код клавиши и символы без/с Shift.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyEntry {
    pub keycode: u16,
    pub lower: String,
    pub upper: String,
}

/// Описание одной раскладки как данных.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    /// Идентификатор TIS, например `com.apple.keylayout.US`.
    pub id: String,
    /// Человекочитаемое имя, например «U.S.».
    pub title: String,
    /// Краткое имя, показываемое в трее (флаг или буква).
    pub short_name: String,
    /// Таблица клавиш, отсортированная по keycode.
    pub keys: Vec<KeyEntry>,
}

impl Layout {
    /// Символ по (keycode, shift), если есть.
    pub fn symbol(&self, keycode: u16, shift: bool) -> Option<&str> {
        self.keys
            .binary_search_by_key(&keycode, |e| e.keycode)
            .ok()
            .map(|i| {
                if shift {
                    self.keys[i].upper.as_str()
                } else {
                    self.keys[i].lower.as_str()
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_lookup() {
        let layout = Layout {
            id: "test".into(),
            title: "Test".into(),
            short_name: "T".into(),
            keys: vec![KeyEntry {
                keycode: 50,
                lower: "x".into(),
                upper: "X".into(),
            }],
        };
        assert_eq!(layout.symbol(50, false), Some("x"));
        assert_eq!(layout.symbol(50, true), Some("X"));
        assert_eq!(layout.symbol(51, false), None);
    }
}
