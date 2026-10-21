//! NSSpellChecker на главном потоке; подключается в движок как SpellChecker.
//!
//! Вызывается из потока тапа -> dispatch_sync на main queue (стандарт
//! CoreFoundation; тап — фоновый поток, взаимоблокировки нет).

use caramba_core::engine::SpellChecker;
use dispatch2::DispatchQueue;
use objc2_app_kit::NSSpellChecker;
use objc2_foundation::NSString;
use std::sync::atomic::AtomicBool;

/// Метки языков орфографии: ru -> ru_RU, остальные — en_US.
fn lang_tag(lang: &str) -> &'static str {
    if lang.starts_with("ru") {
        "ru_RU"
    } else {
        "en_US"
    }
}

/// Слово корректно по орфографии (misspelling с позиции 0 не найден).
pub fn spell_check(word: &str, lang: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let word = word.to_owned();
    let lang = lang.to_owned();
    let result = std::sync::Arc::new(AtomicBool::new(false));
    let flag = std::sync::Arc::clone(&result);
    let work = move || {
        let ns = NSString::from_str(&word);
        let tag = NSString::from_str(lang_tag(&lang));
        let checker = NSSpellChecker::sharedSpellChecker();
        let set_ok = checker.setLanguage(&tag);
        let range = checker.checkSpellingOfString_startingAt(&ns, 0);
        // Misspelling найден с самого начала — слово невалидно.
        let misspelled = range.location == 0;
        flag.store(set_ok && !misspelled, std::sync::atomic::Ordering::SeqCst);
    };
    DispatchQueue::main().exec_sync(work);
    result.load(std::sync::atomic::Ordering::SeqCst)
}

/// Замыкание для движка (подключается при инициализации).
pub fn hook() -> SpellChecker {
    Box::new(spell_check)
}
