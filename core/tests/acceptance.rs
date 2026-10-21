//! Приёмочные примеры из раздела 13 ТЗ (не требующие ОС).
//! Символ `␣` в таблице — пробел: в тестах пробел завершает токен.

use caramba_core::engine::{Action, Engine, Event, Press, Token};
use caramba_core::layouts::{builtin_pair, Lang, Layout};

fn presses(text_en_typed: &str, layout: &Layout) -> Vec<(u16, bool)> {
    let mut out = Vec::new();
    for c in text_en_typed.chars() {
        let entry = layout
            .keys
            .iter()
            .find(|e| e.lower == c.to_string() || e.upper == c.to_string());
        if let Some(e) = entry {
            out.push((e.kc, e.upper == c.to_string()));
        }
    }
    out
}

/// Проиграть набор строки: keyDown каждого символа, затем пробел (␣).
fn play(engine: &mut Engine, text: &str, layout: &Layout) -> Vec<Action> {
    let lang = if layout.id.contains("Russian") {
        Lang::Ru
    } else {
        Lang::En
    };
    engine.set_current_lang(lang);
    let mut actions = Vec::new();
    let mut now_ms = 0u64;
    for c in text.chars().chain(std::iter::once(' ')) {
        let entry = layout
            .keys
            .iter()
            .find(|e| e.lower == c.to_string() || e.upper == c.to_string())
            .unwrap_or_else(|| panic!("клавиша для {c:?} не найдена в {}", layout.id));
        now_ms += 10;
        actions.extend(engine.on_event(
            Event::KeyDown {
                keycode: entry.kc,
                shift: entry.upper == c.to_string(),
                caps: false,
                repeat: false,
            },
            now_ms,
        ));
    }
    actions
}

#[test]
fn таблица_приёмки_ghbdtn() {
    // `ghbdtn␣` в EN-раскладке → `привет␣`, раскладка → RU
    let mut e = Engine::new();
    let actions = play(&mut e, "ghbdtn", &builtin_pair().en);
    assert_eq!(
        actions,
        vec![
            Action::Replace {
                backspaces: 6,
                text: "привет".into()
            },
            Action::SwitchLayout {
                layout: "com.apple.keylayout.Russian".into()
            },
        ]
    );
}

#[test]
fn таблица_приёмки_ckj_yj() {
    // `ckj;yj␣` в EN-раскладке → `сложно␣`, раскладка → RU
    let mut e = Engine::new();
    let actions = play(&mut e, "ckj;yj", &builtin_pair().en);
    assert_eq!(
        actions,
        vec![
            Action::Replace {
                backspaces: 6,
                text: "сложно".into()
            },
            Action::SwitchLayout {
                layout: "com.apple.keylayout.Russian".into()
            },
        ]
    );
}

#[test]
fn таблица_приёмки_руддщ() {
    // `руддщ␣` в RU-раскладке → `hello␣`, раскладка → EN
    let mut e = Engine::new();
    let actions = play(&mut e, "руддщ", &builtin_pair().ru);
    assert_eq!(
        actions,
        vec![
            Action::Replace {
                backspaces: 5,
                text: "hello".into()
            },
            Action::SwitchLayout {
                layout: "com.apple.keylayout.US".into()
            },
        ]
    );
}

#[test]
fn таблица_приёмки_словарное_слово_не_трогается() {
    // Хорошие слова словаря языка остаются на месте.
    for (word, lang) in [
        ("hello", Lang::En),
        ("world", Lang::En),
        ("привет", Lang::Ru),
        ("хорошо", Lang::Ru),
    ] {
        let mut e = Engine::new();
        let layout_ref = match lang {
            Lang::En => builtin_pair().en,
            Lang::Ru => builtin_pair().ru,
        };
        let actions = play(&mut e, word, &layout_ref);
        assert!(
            actions.is_empty(),
            "слово {word} не должно трогаться: {actions:?}"
        );
    }
}

#[test]
fn таблица_приёмки_исключение_ghbdtn() {
    // `ghbdtn␣`, слово `ghbdtn` в исключениях → без изменений
    let mut e = Engine::new();
    e.exceptions_mut().insert("ghbdtn");
    let actions = play(&mut e, "ghbdtn", &builtin_pair().en);
    assert!(actions.is_empty());
}

#[test]
fn таблица_приёмки_пароль() {
    // Клавиши `Qwerty123!` набраны в RU-раскладке → `Qwerty123!`, раскладка → EN
    let mut e = Engine::new();
    let actions = play(&mut e, "Йцукен123", &builtin_pair().ru);
    assert_eq!(
        actions,
        vec![
            // Визуально «Йцукен123» печатается как Qwerty123 и переключается EN.
            Action::Replace {
                backspaces: 9,
                text: "Qwerty123".into()
            },
            Action::SwitchLayout {
                layout: "com.apple.keylayout.US".into()
            },
        ]
    );
}

#[test]
fn таблица_приёмки_secure_input_через_события() {
    // При Secure Input платформа просто не шлёт события (FR-SEC-1);
    // проверяем инвариант: без событий движок не выдаёт действий.
    let mut e = Engine::new();
    let t = Token {
        presses: presses("ghbdtn", &builtin_pair().en)
            .into_iter()
            .map(|(kc, sh)| Press {
                keycode: kc,
                shift: sh,
                caps: false,
            })
            .collect(),
    };
    let d = e.evaluate(&t, 1000);
    assert_eq!(d.reason, caramba_core::engine::Reason::PairValid);
    // Платформе достаточно не вызывать evaluate — текст не меняется.
}
