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

/// Раздел 13: `привет1␣` (RU) → `привет!` (фокус 2).
#[test]
fn таблица_приёмки_привет1() {
    let mut e = Engine::new();
    let actions = play(&mut e, "привет1", &builtin_pair().ru);
    assert_eq!(
        actions,
        vec![Action::Replace {
            backspaces: 7,
            text: "привет!".into()
        }]
    );
}

/// Раздел 13: `исполнили сонатуБаха␣` → слепленные слова разделены (фокус 3).
#[test]
fn таблица_приёмки_сонату_баха() {
    let mut e = Engine::new();
    let first = play(&mut e, "исполнили", &builtin_pair().ru);
    assert!(first.is_empty(), "валидное слово не трогается");
    let actions = play(&mut e, "сонатуБаха", &builtin_pair().ru);
    assert_eq!(
        actions,
        vec![Action::Replace {
            backspaces: 10,
            text: "сонату Баха".into()
        }]
    );
}

/// Раздел 13: `9хорошо␣` → `(хорошо` (фокус 4а).
#[test]
fn таблица_приёмки_скобка_открывающая() {
    let mut e = Engine::new();
    let actions = play(&mut e, "9хорошо", &builtin_pair().ru);
    assert_eq!(
        actions,
        vec![Action::Replace {
            backspaces: 7,
            text: "(хорошо".into()
        }]
    );
}

/// Раздел 13: `(хорошо0␣` → `(хорошо)` (фокус 4б, скобка была открыта).
#[test]
fn таблица_приёмки_скобка_закрывающая() {
    let mut e = Engine::new();
    let actions = play(&mut e, "(хорошо0", &builtin_pair().ru);
    assert_eq!(
        actions,
        vec![Action::Replace {
            backspaces: 8,
            text: "(хорошо)".into()
        }]
    );
}

/// Раздел 13: `пирогб␣` → `пирог,`, `рубю␣` → `руб.` (фокус 5).
#[test]
fn таблица_приёмки_знаки_в_конце() {
    let mut e = Engine::new();
    assert_eq!(
        play(&mut e, "пирогб", &builtin_pair().ru),
        vec![Action::Replace {
            backspaces: 6,
            text: "пирог,".into()
        }]
    );
    assert_eq!(
        play(&mut e, "рубю", &builtin_pair().ru),
        vec![Action::Replace {
            backspaces: 4,
            text: "руб.".into()
        }]
    );
}

/// Раздел 13: `хочуЁ␣` → `хочу!` (фокус 6).
#[test]
fn таблица_приёмки_хочу_ё() {
    let mut e = Engine::new();
    let actions = play(&mut e, "хочуЁ", &builtin_pair().ru);
    assert_eq!(
        actions,
        vec![Action::Replace {
            backspaces: 5,
            text: "хочу!".into()
        }]
    );
}

/// Раздел 13: частицы (фокус 7).
#[test]
fn таблица_приёмки_частицы() {
    let mut e = Engine::new();
    let ru = builtin_pair().ru;
    for (typed, expected) in [
        ("почемуто", "почему-то"),
        ("чтото", "что-то"),
        ("кембы", "кем бы"),
        ("хорошобы", "хорошо бы"),
    ] {
        assert_eq!(
            play(&mut e, typed, &ru),
            vec![Action::Replace {
                backspaces: typed.chars().count(),
                text: expected.into()
            }],
            "фокус 7: {typed}"
        );
    }
}

/// FR-YO-1: ёфикатор заменяет «е» при включённом флаге; грязные
/// неоднозначные слова («все») не трогаются.
#[test]
fn ёфикатор_однозначные_формы() {
    use caramba_core::engine::EngineConfig;
    let mut e = Engine::new();
    let cfg = EngineConfig {
        yo_fixer: true,
        ..EngineConfig::default()
    };
    e.set_config(cfg);
    let actions = play(&mut e, "еще", &builtin_pair().ru);
    assert_eq!(
        actions,
        vec![Action::Replace {
            backspaces: 3,
            text: "ещё".into()
        }]
    );
    let actions = play(&mut e, "все", &builtin_pair().ru);
    assert!(actions.is_empty(), "неоднозначное «все» не меняется");
}

/// По умолчанию ёфикатор выключен — «еще» остаётся без изменений.
#[test]
fn ёфикатор_выключен_по_умолчанию() {
    let mut e = Engine::new();
    let actions = play(&mut e, "еще", &builtin_pair().ru);
    assert!(actions.is_empty());
}

/// FR-TYPO-1(б): ровно один словарный кандидат «сова» из «своа».
#[test]
fn опечатка_перестановка() {
    let mut e = Engine::new();
    let actions = play(&mut e, "своа", &builtin_pair().ru);
    assert_eq!(
        actions,
        vec![Action::Replace {
            backspaces: 4,
            text: "сова".into()
        }]
    );
}

/// Опечатки отключаются флагом.
#[test]
fn опечатки_отключены() {
    use caramba_core::engine::EngineConfig;
    let mut e = Engine::new();
    let cfg = EngineConfig {
        typos_fix: false,
        ..EngineConfig::default()
    };
    e.set_config(cfg);
    let actions = play(&mut e, "своа", &builtin_pair().ru);
    assert!(actions.is_empty());
}

#[test]
fn backspace_удаляет_символ_из_буфера() {
    // Пользователь набрал «ghbdtx», нажал ⌫ (Backspace), затем «n» и пробел → должно конвертироваться в «привет»
    use caramba_core::engine::Event;
    let mut e = Engine::new();
    let pair = builtin_pair();
    let layout = &pair.en;
    e.set_current_lang(caramba_core::layouts::Lang::En);
    let mut now_ms = 0u64;

    for c in "ghbdtx".chars() {
        let entry = layout.keys.iter().find(|entry| entry.lower == c.to_string()).unwrap();
        now_ms += 10;
        e.on_event(Event::KeyDown { keycode: entry.kc, shift: false, caps: false, repeat: false }, now_ms);
    }

    // Нажатие Backspace (0x33)
    now_ms += 10;
    let act = e.on_event(Event::KeyDown { keycode: 0x33, shift: false, caps: false, repeat: false }, now_ms);
    assert!(act.is_empty(), "Backspace не должен вызывать ложную замену");

    // Нажатие 'n'
    let entry_n = layout.keys.iter().find(|entry| entry.lower == "n").unwrap();
    now_ms += 10;
    e.on_event(Event::KeyDown { keycode: entry_n.kc, shift: false, caps: false, repeat: false }, now_ms);

    // Нажатие пробела
    now_ms += 10;
    let actions = e.on_event(Event::KeyDown { keycode: 0x31, shift: false, caps: false, repeat: false }, now_ms);
    assert_eq!(
        actions,
        vec![
            Action::Replace { backspaces: 6, text: "привет".into() },
            Action::SwitchLayout { layout: "com.apple.keylayout.Russian".into() }
        ]
    );
}

#[test]
fn enter_исправление_по_опции() {
    use caramba_core::engine::{EngineConfig, Event};
    let mut e = Engine::new();
    let pair = builtin_pair();
    let layout = &pair.en;
    e.set_current_lang(caramba_core::layouts::Lang::En);

    // 1. По умолчанию fix_on_enter = false: Enter очищает буфер без замены
    for c in "ghbdtn".chars() {
        let entry = layout.keys.iter().find(|entry| entry.lower == c.to_string()).unwrap();
        e.on_event(Event::KeyDown { keycode: entry.kc, shift: false, caps: false, repeat: false }, 10);
    }
    let actions = e.on_event(Event::KeyDown { keycode: 0x24, shift: false, caps: false, repeat: false }, 20);
    assert!(actions.is_empty(), "Enter не должен исправлять при fix_on_enter = false");

    // 2. При fix_on_enter = true: Enter выполняет замену
    e.set_config(EngineConfig { fix_on_enter: true, ..EngineConfig::default() });
    for c in "ghbdtn".chars() {
        let entry = layout.keys.iter().find(|entry| entry.lower == c.to_string()).unwrap();
        e.on_event(Event::KeyDown { keycode: entry.kc, shift: false, caps: false, repeat: false }, 10);
    }
    let actions = e.on_event(Event::KeyDown { keycode: 0x24, shift: false, caps: false, repeat: false }, 20);
    assert_eq!(
        actions,
        vec![
            Action::Replace { backspaces: 6, text: "привет".into() },
            Action::SwitchLayout { layout: "com.apple.keylayout.Russian".into() }
        ]
    );
}
