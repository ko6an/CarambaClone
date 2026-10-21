//! «Фокусы Карамбы 2–7» (FR-TRICK-2..7). Чистые функции: на входе
//! отрисованный токен, на выходе — возможный результат. Правила
//! проверяются в движке: исходный токен не словарный, а части
//! результата — словарные.

/// Фокус 2: «привет1» → «привет!» — единица вместо знака.
pub fn trick_unit_sign(text: &str) -> Option<String> {
    let stripped = text.strip_suffix('1')?;
    if stripped.is_empty() || contains_digit(stripped) {
        return None;
    }
    Some(format!("{stripped}!"))
}

/// Фокус 4 (а): «9хорошо» → «(хорошо» — незацифрованная открывающая скобка.
pub fn trick_bracket_open(text: &str) -> Option<String> {
    let rest = text.strip_prefix('9')?;
    if rest.is_empty() || !rest.chars().all(char_is_letter) {
        return None;
    }
    Some(format!("({rest}"))
}

/// Фокус 4 (б): «хорошо0» → «хорошо)» — закрывающая скобка, если
/// открытая скобка есть (open_brackets > 0).
pub fn trick_bracket_close(text: &str, open_brackets: usize) -> Option<String> {
    if open_brackets == 0 {
        return None;
    }
    let base = text.strip_suffix('0')?;
    if base.is_empty() || contains_digit(base) {
        return None;
    }
    Some(format!("{base})"))
}

/// Фокус 5: «пирогб» → «пирог,», «рубю» → «руб.».
pub fn trick_trailing_signs(text: &str) -> Option<String> {
    let last = text.chars().last()?;
    let sign = match last {
        'б' => ',',
        'ю' => '.',
        _ => return None,
    };
    let base = &text[..text.len() - last.len_utf8()];
    if base.is_empty() || contains_digit(base) {
        return None;
    }
    Some(format!("{base}{sign}"))
}

/// Фокус 6: «хочуЁ» → «хочу!».
pub fn trick_cap_excl(text: &str) -> Option<String> {
    let base = text.strip_suffix('Ё')?;
    if base.is_empty() || contains_digit(base) {
        return None;
    }
    Some(format!("{base}!"))
}

/// Фокус 7: частицы — «почемуто» → «почему-то», «кембы» → «кем бы».
pub fn trick_particles(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    // Дефисные: «то», «либо», «нибудь», «таки» — с префиксной валидацией.
    for suffix in ["нибудь", "либо", "таки", "то"] {
        if let Some(prefix) = text.strip_suffix(suffix) {
            if prefix.len() >= 2 {
                out.push(format!("{prefix}-{suffix}"));
            }
        }
    }
    // Раздельные: «бы», «ли», «же».
    for suffix in ["бы", "ли", "же"] {
        if let Some(prefix) = text.strip_suffix(suffix) {
            if prefix.len() >= 2 {
                out.push(format!("{prefix} {suffix}"));
            }
        }
    }
    out
}

/// Фокус 3: слипшиеся слова — «сонатуБаха» → «сонату Баха».
/// Возвращает пару частей, если внутри кириллического слова ровно одна
/// заглавная и обе половины состоят из букв.
pub fn trick_stuck_words(text: &str) -> Option<(String, String)> {
    let first = text.chars().next()?;
    if first.is_uppercase() {
        return None; // «ВКонтакте», CamelCase — с начала, не наш случай
    }
    let up: Vec<usize> = text
        .char_indices()
        .filter(|(i, c)| *i > 0 && c.is_uppercase())
        .map(|(i, _)| i)
        .collect();
    if up.len() != 1 {
        return None;
    }
    let split = up[0];
    let a = &text[..split];
    let b = &text[split..];
    if a.chars().any(|c| !char_is_letter(c)) || b.chars().any(|c| !char_is_letter(c)) {
        return None;
    }
    Some((a.to_string(), b.to_string()))
}

fn contains_digit(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_digit())
}

fn char_is_letter(c: char) -> bool {
    c.is_alphabetic()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn фокус2() {
        assert_eq!(trick_unit_sign("привет1"), Some("привет!".to_string()));
        assert_eq!(trick_unit_sign("х11"), None); // цифры внутри — нет
        assert_eq!(trick_unit_sign("ghbdtn"), None);
    }

    #[test]
    fn фокус4() {
        assert_eq!(trick_bracket_open("9хорошо"), Some("(хорошо".to_string()));
        assert_eq!(
            trick_bracket_close("хорошо0", 1),
            Some("хорошо)".to_string())
        );
        assert_eq!(trick_bracket_close("хорошо0", 0), None);
    }

    #[test]
    fn фокус5_6_7() {
        assert_eq!(trick_trailing_signs("пирогб"), Some("пирог,".to_string()));
        assert_eq!(trick_trailing_signs("рубэ"), None);
        assert_eq!(trick_cap_excl("хочуЁ"), Some("хочу!".to_string()));
        assert!(trick_particles("кембы").contains(&"кем бы".to_string()));
    }
}
