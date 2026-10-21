//! Таблицы раскладок и конвертация между парами.

use serde::{Deserialize, Serialize};

/// Язык раскладки (упрощённо: какая букво-система символом порождается).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    En,
    Ru,
}

/// Класс символа — нужен для разделения букв и знаков.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharClass {
    /// Кириллическая буква.
    Cyrillic,
    /// Латинская буква.
    Latin,
    /// Цифра 0–9.
    Digit,
    /// Знак: пунктуация, спецсимволы.
    Sign,
    /// Пробел.
    Space,
}

pub fn classify(c: char) -> CharClass {
    match c {
        ' ' => CharClass::Space,
        '0'..='9' => CharClass::Digit,
        'а'..='я' | 'ё' | 'А'..='Я' | 'Ё' => CharClass::Cyrillic,
        'a'..='z' | 'A'..='Z' => CharClass::Latin,
        _ => CharClass::Sign,
    }
}

/// Запись таблицы: код клавиши и символы без/с Shift.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyEntry {
    pub kc: u16,
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
    /// Язык таблицы.
    pub lang: Lang,
    /// Таблица клавиш, отсортированная по `kc`.
    pub keys: Vec<KeyEntry>,
}

impl Layout {
    /// Загрузка таблицы из TOML (файл вшивается в бинарник в тестах и M0).
    pub fn from_toml(raw: &str) -> Result<Self, String> {
        toml::from_str(raw).map_err(|e| format!("не удалось разобрать таблицу: {e}"))
    }

    /// Символ по (keycode, эффективный Shift).
    pub fn symbol(&self, kc: u16, shift: bool) -> Option<&str> {
        self.keys.binary_search_by_key(&kc, |e| e.kc).ok().map(|i| {
            if shift {
                self.keys[i].upper.as_str()
            } else {
                self.keys[i].lower.as_str()
            }
        })
    }

    /// Есть ли буква по этому keycode с этим Shift.
    pub fn letter(&self, kc: u16, shift: bool) -> Option<char> {
        self.symbol(kc, shift)
            .and_then(|s| s.chars().next())
            .filter(|c| matches!(classify(*c), CharClass::Latin | CharClass::Cyrillic))
    }

    /// Есть ли знак (небуква) по этому keycode с этим Shift.
    pub fn sign(&self, kc: u16, shift: bool) -> Option<char> {
        self.symbol(kc, shift)
            .and_then(|s| s.chars().next())
            .filter(|c| matches!(classify(*c), CharClass::Sign | CharClass::Digit))
    }
}

/// Пара раскладок EN/RU, между которыми переключается движок.
#[derive(Debug, Clone)]
pub struct Pair {
    pub en: Layout,
    pub ru: Layout,
}

impl Pair {
    pub fn by_lang(&self, lang: Lang) -> &Layout {
        match lang {
            Lang::En => &self.en,
            Lang::Ru => &self.ru,
        }
    }

    pub fn other(&self, lang: Lang) -> Lang {
        match lang {
            Lang::En => Lang::Ru,
            Lang::Ru => Lang::En,
        }
    }
}

/// Встроенные таблицы (по реальным системным раскладкам macOS,
/// выгруженным через `UCKeyTranslate`; см. docs/PROGRESS.md).
pub fn builtin_pair() -> Pair {
    let us = Layout::from_toml(include_str!("../../../resources/layouts/us.toml"))
        .expect("таблица US корректна");
    let ru = Layout::from_toml(include_str!("../../../resources/layouts/ru.toml"))
        .expect("таблица RU корректна");
    Pair { en: us, ru }
}

/// Встроенные таблицы обеих русских раскладок (для сверки M2/M12).
pub fn builtin_russian_variants() -> (Layout, Layout) {
    let ru =
        Layout::from_toml(include_str!("../../../resources/layouts/ru.toml")).expect("корректна");
    let ruwin = Layout::from_toml(include_str!("../../../resources/layouts/ruwin.toml"))
        .expect("корректна");
    (ru, ruwin)
}
