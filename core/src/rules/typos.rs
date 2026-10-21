//! Автоисправление опечаток (FR-TYPO-1).
//!
//! Пока только алгоритмические классы (б) и (в): перестановка
//! соседних букв, пропущенная/удвоенная буква, «ДВе заглавные».
//! Таблица частотных опечаток из пакета «Правил» подключается в M6.

/// «ДВе заглавные» → «Две»: две первые буквы в верхнем регистре и
/// остальные строчные.
pub fn fix_double_capital(text: &str) -> Option<String> {
    let mut cs = text.chars();
    let f = cs.next()?;
    if !f.is_uppercase() {
        return None;
    }
    let s = cs.next()?;
    if !s.is_uppercase() {
        return None;
    }
    let rest: String = cs.collect();
    if rest.chars().any(|c| c.is_uppercase()) {
        return None;
    }
    if rest.is_empty() {
        return None;
    }
    let mut out = String::new();
    out.push(f);
    out.extend(s.to_lowercase());
    out.push_str(&rest);
    Some(out)
}

/// Кандидаты-опечатки, дающие ровно один словарный результат.
/// `is_valid` — проверка словарности кандидата.
pub fn typo_candidates(text: &str, is_valid: impl Fn(&str) -> bool) -> Option<String> {
    if text.chars().count() < 4 {
        return None;
    }
    let mut found: Option<String> = None;
    let mut count = 0usize;
    let mut candidate = |c: String| {
        if is_valid(&c) {
            count += 1;
            found = Some(c);
        }
    };

    let chars: Vec<char> = text.chars().collect();
    // (b.1) перестановка соседних букв.
    for i in 0..chars.len().saturating_sub(1) {
        if chars[i] == chars[i + 1] {
            continue;
        }
        let mut c = chars.clone();
        c.swap(i, i + 1);
        candidate(String::from_iter(&c[..]));
    }
    // (b.2) пропущенная буква.
    for i in 0..chars.len() {
        let mut c = chars.clone();
        c.remove(i);
        candidate(String::from_iter(&c[..]));
    }
    // (b.3) удвоение буквы.
    for i in 0..chars.len() {
        if i > 0 && chars[i] == chars[i - 1] {
            continue;
        }
        let mut c = Vec::with_capacity(chars.len() + 1);
        c.extend_from_slice(&chars[..=i]);
        c.push(chars[i]);
        c.extend_from_slice(&chars[i + 1..]);
        candidate(String::from_iter(&c[..]));
    }

    if count == 1 {
        found
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn две_заглавные() {
        assert_eq!(fix_double_capital("ДВе"), Some("Две".into()));
        assert_eq!(fix_double_capital("ПРИ"), None);
        assert_eq!(fix_double_capital("Две"), None);
    }

    #[test]
    fn перестановка() {
        // «своа» → единственный словарный кандидат «сова» (свап о/в).
        assert_eq!(
            typo_candidates("своа", |c| c == "сова"),
            Some("сова".to_string())
        );
    }

    #[test]
    fn неоднозначность_отбрасывается() {
        // Два валидных кандидата — исправления нет.
        assert_eq!(typo_candidates("ab", |c| c == "a" || c == "b"), None);
    }

    #[test]
    fn короткий_токен_не_трогаем() {
        assert_eq!(typo_candidates("до", |c| c == "до" || c == "лдо"), None);
    }
}
