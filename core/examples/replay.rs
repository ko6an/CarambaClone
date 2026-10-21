//! Песочница: `cargo run -p caramba-core --example replay -- 'ghbdtn '`
//!
//! Показывает обе интерпретации, оценки n-gram, решение и причину —
//! для отладки эвристики. Текст интерпретируется как нажатия в выбранной
//! раскладке (-l en|ru, по умолчанию en).

use caramba_core::engine::{Action, Decision, Engine, Reason, Token};
use caramba_core::layouts::{builtin_pair, Lang, Layout};

fn presses_of(text: &str, layout: &Layout) -> Vec<(u16, bool)> {
    let mut out = Vec::new();
    for c in text.chars() {
        let entry = layout
            .keys
            .iter()
            .find(|e| e.lower == c.to_string() || e.upper == c.to_string());
        match entry {
            Some(e) => out.push((e.kc, e.upper == c.to_string())),
            None => {
                eprintln!("клавиша для {c:?} не найдена в таблице {}", layout.id);
                std::process::exit(2);
            }
        }
    }
    out
}

fn reason_name(r: Reason) -> &'static str {
    match r {
        Reason::Skipped => "отфильтровано (исключение/технический/длина)",
        Reason::CurrentValid => "текущая интерпретация — валидное слово; не трогаем",
        Reason::PairValid => "парная интерпретация — валидное слово; конвертируем",
        Reason::Ngram => "эвристика n-gram: уверенная разница",
        Reason::Password => "токен похож на пароль: EN и без искажений",
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut layout_flag = "en".to_string();
    let mut text = String::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "-l" | "--layout" => layout_flag = args.next().unwrap_or_default(),
            "-h" | "--help" => {
                println!("replay: [-l en|ru] 'текст' — обе интерпретации, оценки, решение");
                return;
            }
            other => text = other.to_string(),
        }
    }
    if text.is_empty() {
        eprintln!("укажите текст: cargo run -p caramba-core --example replay -- 'ghbdtn '");
        std::process::exit(1);
    }

    let pair = builtin_pair();
    let (lang, layout) = match layout_flag.as_str() {
        "ru" => (Lang::Ru, pair.by_lang(Lang::Ru)),
        _ => (Lang::En, pair.by_lang(Lang::En)),
    };
    let mut engine = Engine::with_pair(pair.clone());
    engine.set_current_lang(lang);

    let presses: Vec<(u16, bool)> = presses_of(text.trim_end(), layout);
    let token = Token {
        presses: presses
            .into_iter()
            .map(|(kc, shift)| caramba_core::engine::Press {
                keycode: kc,
                shift,
                caps: false,
            })
            .collect(),
    };

    let Decision {
        actions,
        reason,
        cur_text,
        alt_text,
        cur_score,
        alt_score,
    } = engine.evaluate(&token, 0);
    let lang_name = if lang == Lang::En { "EN" } else { "RU" };
    println!("токен набран в {lang_name}-раскладке ({}):", layout.id);
    println!("  cur: {cur_text:?}  (score {cur_score:.3})");
    println!("  alt: {alt_text:?}  (score {alt_score:.3})");
    println!("  решение: {}", reason_name(reason));
    for a in &actions {
        match a {
            Action::Replace { backspaces, text } => {
                println!("  → замена: −{backspaces}, печать {text:?}")
            }
            Action::SwitchLayout { layout } => println!("  → смена раскладки: {layout}"),
            other => println!("  → {other:?}"),
        }
    }
}
