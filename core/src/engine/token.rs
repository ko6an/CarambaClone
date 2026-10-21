//! Буфер нажатий и токенизация (раздел 3.1 ТЗ).

use crate::layouts::{classify, CharClass, Layout};

/// Коды клавиш навигации и редактирования (kVK_*).
pub const KVK_RETURN: u16 = 0x24;
pub const KVK_TAB: u16 = 0x30;
pub const KVK_SPACE: u16 = 0x31;
pub const KVK_BACKSPACE: u16 = 0x33;
pub const KVK_ESCAPE: u16 = 0x35;
pub const KVK_LEFT: u16 = 0x7B;
pub const KVK_RIGHT: u16 = 0x7C;
pub const KVK_DOWN: u16 = 0x7D;
pub const KVK_UP: u16 = 0x7E;
pub const KVK_HOME: u16 = 0x73;
pub const KVK_END: u16 = 0x77;
pub const KVK_PAGEUP: u16 = 0x74;
pub const KVK_PAGEDOWN: u16 = 0x79;

/// Отдельное нажатие в буфере (3.1: храним `keycode+shift(+caps)`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Press {
    pub keycode: u16,
    /// Shift в момент нажатия.
    pub shift: bool,
    /// Caps Lock в момент нажатия.
    pub caps: bool,
}
/// Автоповтор помечается отдельным флагом события и в токен не попадает
/// (допущение D10: это уточнение закреплено в движке.)
impl Press {
    /// Эффективный Shift для отрисовки символа (учёт Caps для букв).
    pub fn eff_shift(&self, layout: &Layout) -> bool {
        let is_letter = layout
            .symbol(self.keycode, self.shift)
            .and_then(|s| s.chars().next())
            .map(|c| matches!(classify(c), CharClass::Latin | CharClass::Cyrillic))
            .unwrap_or(false);
        if is_letter && self.caps {
            // Caps инвертирует верх/низ букв.
            !self.shift
        } else {
            self.shift
        }
    }
}

/// Класс символа — нужен для разделения букв/цифр/знаков.
pub fn classify_char(c: char) -> CharClass {
    classify(c)
}

/// Токен — нажатия между жёсткими границами (3.1).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Token {
    pub presses: Vec<Press>,
}

impl Token {
    pub fn push(&mut self, p: Press) {
        self.presses.push(p);
    }

    pub fn len(&self) -> usize {
        self.presses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.presses.is_empty()
    }

    /// Отрисовать нажатия в заданной раскладке.
    pub fn render(&self, layout: &Layout) -> String {
        self.presses
            .iter()
            .filter_map(|p| {
                let eff = p.eff_shift(layout);
                layout.symbol(p.keycode, eff).map(str::to_string)
            })
            .collect()
    }
}

/// Клавиша-навигации/редактирования — жёсткая граница по определению.
pub fn is_nav_boundary(kc: u16) -> bool {
    matches!(
        kc,
        KVK_RETURN
            | KVK_TAB
            | KVK_BACKSPACE
            | KVK_ESCAPE
            | KVK_LEFT
            | KVK_RIGHT
            | KVK_DOWN
            | KVK_UP
            | KVK_HOME
            | KVK_END
            | KVK_PAGEUP
            | KVK_PAGEDOWN
    )
}

/// Жёсткая граница токена: навигация, пробел или клавиша, дающая
/// знак препинания в ОБЕИХ раскладках пары (цифры входят в токен — D10;
/// «неоднозначные» клавиши — буквы в одной из раскладок, не граница).
pub fn is_hard_boundary(kc: u16, cur: &Layout, other: &Layout) -> bool {
    if is_nav_boundary(kc) || kc == KVK_SPACE {
        return true;
    }
    let cur_c = cur.symbol(kc, false).and_then(|s| s.chars().next());
    let other_c = other.symbol(kc, false).and_then(|s| s.chars().next());
    match (cur_c, other_c) {
        (Some(a), Some(b)) => classify(a) == CharClass::Sign && classify(b) == CharClass::Sign,
        _ => false,
    }
}

/// Из пары таблиц: клавиши «буква/знак» (неоднозначные, 3.1).
pub fn ambiguous_keys(cur: &Layout, other: &Layout) -> Vec<u16> {
    let mut out = Vec::new();
    for e in &cur.keys {
        if is_ambiguous(e.kc, cur, other) {
            out.push(e.kc);
        }
    }
    out
}

/// Клавиша неоднозначна, если классы символа в паре разные буква/не-буква.
pub fn is_ambiguous(kc: u16, cur: &Layout, other: &Layout) -> bool {
    let cur_c = cur.symbol(kc, false).and_then(|s| s.chars().next());
    let other_c = other.symbol(kc, false).and_then(|s| s.chars().next());
    match (cur_c, other_c) {
        (Some(a), Some(b)) => {
            let is_letter = |c: char| matches!(classify(c), CharClass::Latin | CharClass::Cyrillic);
            is_letter(a) != is_letter(b)
        }
        _ => false,
    }
}

/// Варианты интерпретации краевых неоднозначных клавиш (3.2, п. 2):
/// сколько первых/последних нажатий трактовать как знаки парной раскладки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeVariant {
    /// Количество первых нажатий, интерпретируемых как знаки.
    pub lead: usize,
    /// Количество последних нажатий, интерпретируемых как знаки.
    pub tail: usize,
}

/// Допустимые разбиения краевых клавиш (обычно ≤ 4).
pub fn edge_variants(token: &Token, cur: &Layout, other: &Layout) -> Vec<EdgeVariant> {
    let n = token.len();
    let mut lead = 0;
    while lead < n && is_ambiguous(token.presses[lead].keycode, cur, other) {
        lead += 1;
    }
    let mut tail = 0;
    while tail < n
        && n - 1 - tail >= lead
        && is_ambiguous(token.presses[n - 1 - tail].keycode, cur, other)
    {
        tail += 1;
    }
    let mut out = vec![EdgeVariant { lead: 0, tail: 0 }];
    if tail > 0 {
        out.push(EdgeVariant { lead: 0, tail });
    }
    if lead > 0 {
        out.push(EdgeVariant { lead, tail: 0 });
        if tail > 0 {
            out.push(EdgeVariant { lead, tail });
        }
    }
    out
}
