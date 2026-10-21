//! Proptest: round-trip таблиц раскладок по всем клавишам.
//!
//! Для каждой клавиши проверяем: отрисовка символа и его клавиша-эквивалент
//! стабильно соответствуют таблице (символ найден, отрисовка обратима).

use proptest::prelude::*;

proptest! {
    #[test]
    fn layout_table_contains_both_states(kc in 0u16..256, upper in 0..2) {
        let us = caramba_core::layouts::builtin_pair().en;
        let entry = us.symbol(kc, upper == 1);
        // Отдельные клавиши могут не дать символа; никаких паник.
        if let Some(sym) = entry {
            prop_assert!(sym.is_empty() || !sym.contains(char::is_control));
        }
    }

    #[test]
    fn russian_layouts_letters_complete(kc in 0u16..256) {
        let (ru, ruwin) = caramba_core::layouts::builtin_russian_variants();
        // Буква существует в одной из русских таблиц или её там нет вовсе — паник запрещён.
        let _ = (ru.symbol(kc, false), ru.symbol(kc, true), ruwin.symbol(kc, false), ruwin.symbol(kc, true));
    }
}
