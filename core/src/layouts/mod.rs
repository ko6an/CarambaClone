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

/// Конвертация текста «как набрано» между раскладками пары (ручные режимы).
///
/// Для каждого символа ищется клавиша в таблице источника, а символ
/// отрисовывается таблицей приёмника в том же состоянии Shift (регистр
/// сохраняется автоматически, таблицы хранят верх/низ явно).
/// Символы без пары (знаки, чужие буквы) возвращаются без изменений.
pub fn flip_text(text: &str, from: &Layout, to: &Layout) -> String {
    text.chars().map(|c| flip_char(c, from, to)).collect()
}

/// Один символ: пара (keycode, shift) в источнике -> символ в приёмнике.
fn flip_char(c: char, from: &Layout, to: &Layout) -> String {
    let key = c.to_string();
    // Неоптимальный линейный поиск допустим: вызов редко и по малым текстам.
    match from.keys.iter().find(|e| e.lower == key || e.upper == key) {
        Some(e) => {
            let shift = e.upper == key;
            match to.symbol(e.kc, shift) {
                Some(sym) => sym.to_string(),
                None => key,
            }
        }
        None => key,
    }
}

impl Pair {
    /// Автоопределение направления по буквам текста: кириллица -> EN,
    /// иначе латиница -> RU. Нет букв — без изменений.
    pub fn flip_auto(&self, text: &str) -> String {
        let has_ru = text.chars().any(|c| classify(c) == CharClass::Cyrillic);
        let has_en = text.chars().any(|c| classify(c) == CharClass::Latin);
        match (has_ru, has_en) {
            (true, _) => flip_text(text, self.by_lang(Lang::Ru), self.by_lang(Lang::En)),
            (false, true) => flip_text(text, self.by_lang(Lang::En), self.by_lang(Lang::Ru)),
            _ => text.to_string(),
        }
    }

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

#[cfg(test)]
mod flip_tests {
    use super::*;

    #[test]
    fn ghbdtn_to_privet() {
        assert_eq!(builtin_pair().flip_auto("ghbdtn"), "привет");
    }

    #[test]
    fn ru_to_hello() {
        assert_eq!(builtin_pair().flip_auto("руддщ"), "hello");
    }

    #[test]
    fn регистр_сохраняется() {
        assert_eq!(builtin_pair().flip_auto("Ghbdtn"), "Привет");
    }

    #[test]
    fn знаки_переводятся_по_клавише() {
        // Точка EN -> ю, запятая EN -> б (тавтологично «как набрано»).
        assert_eq!(builtin_pair().flip_auto("ghbdtn."), "приветю");
        assert_eq!(builtin_pair().flip_auto("ghbdtn, "), "приветб ");
    }

    #[test]
    fn цифры_не_меняются() {
        assert_eq!(builtin_pair().flip_auto("123"), "123");
    }

    #[test]
    fn пустой_и_смешанный() {
        assert_eq!(builtin_pair().flip_auto(""), "");
        assert_eq!(builtin_pair().flip_auto("ghb"), "при");
    }
}

// MARK: Сверка «Russian» и «Russian – PC» (M2: дисциплина таблиц)

#[cfg(test)]
mod russian_variants_tests {
    use super::*;
    use crate::engine::token::KVK_BACKSPACE;

    #[test]
    fn россиан_и_расклад_пк_различаются_знакомыми() {
        let (ru, ruwin) = builtin_russian_variants();
        // Различия в рядах цифр: Shift+4 и частично Shift+2-Kana row.
        assert_eq!(ru.symbol(0x15, true), Some("%"));
        assert_eq!(ruwin.symbol(0x15, true), Some(";"));
        // Обе - кириллица по буквенному ряду.
        assert_eq!(ru.symbol(0x0B, false), Some("и"));
        assert_eq!(ruwin.symbol(0x0B, false), Some("и"));
    }

    #[test]
    fn полнота_и_навигационные_границы() {
        let pair = builtin_pair();
        let (ru, ruwin) = builtin_russian_variants();
        // Все таблицы непусты и отсортированы.
        for (l, name) in [
            (&pair.en, "us"),
            (&pair.ru, "ru"),
            (&ru, "ru"),
            (&ruwin, "ruwin"),
        ] {
            assert!(!l.keys.is_empty(), "{name} пуста");
            for w in l.keys.windows(2) {
                assert!(w[0].kc < w[1].kc, "{name}: сортировка нарушена");
            }
        }
        // Навигация — граница по определению.
        assert!(crate::engine::token::is_nav_boundary(KVK_BACKSPACE));
    }
}
