//! Ёфикатор (FR-YO-1): однозначные пары «е» → «ё».
//!
//! Учитывается регистр первой буквы. Неоднозначные слова в списке
//! отсутствуют (D7).

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct YoPairs {
    p: Vec<Pair>,
}

#[derive(Debug, Deserialize)]
struct Pair {
    from: String,
    to: String,
}

/// Таблица пар (вшитая / из пакета «Правил» в M6).
pub fn builtin() -> Vec<(String, String)> {
    let raw = include_str!("../../../resources/dictionaries/yo.toml");
    let parsed: YoPairs = toml::from_str(raw).expect("yo.toml корректен");
    parsed.p.into_iter().map(|p| (p.from, p.to)).collect()
}

/// Одно слово: если однозначная ё-форма есть — вернуть её.
pub fn fix_word(word: &str, pairs: &[(String, String)]) -> Option<String> {
    let lower = word.to_lowercase();
    let found = pairs.iter().find(|(from, _)| *from == lower)?;
    let (_, to) = found;
    // Сохраняем регистр первой буквы.
    if word.chars().next().is_some_and(|f| f.is_uppercase()) {
        let mut cs = to.chars();
        match (cs.next(), word.chars().next()) {
            (Some(start), Some(first)) => {
                let mut out = String::new();
                out.push(first.to_uppercase().next().unwrap_or(start));
                out.push_str(cs.as_str());
                Some(out)
            }
            _ => Some(to.clone()),
        }
    } else {
        Some(to.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn однозначные_формы() {
        let pairs = builtin();
        assert_eq!(fix_word("еще", &pairs).as_deref(), Some("ещё"));
        assert_eq!(fix_word("ещё", &pairs), None); // уже ё
        assert_eq!(fix_word("идет", &pairs).as_deref(), Some("идёт"));
    }

    #[test]
    fn неоднозначные_не_трогаются() {
        let pairs = builtin();
        // Список содержит только явные: «все», «лет», «небо» — вне списка.
        assert_eq!(fix_word("все", &pairs), None);
        assert_eq!(fix_word("небо", &pairs), None);
    }
}
