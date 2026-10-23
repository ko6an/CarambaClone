//! NSSpellChecker на главном потоке; подключается в движок как SpellChecker.
//!
//! Вызывается из потока тапа -> диспетчеризация в главный поток с
//! тайм-аутом (main_thread.rs): тап в этот момент держит лок движка,
//! и без тайм-аута ожидание главного потока могло бы зависнуть навсегда.

use caramba_core::engine::SpellChecker;
use objc2_app_kit::NSSpellChecker;
use objc2_foundation::NSString;
use std::time::Duration;

/// Метки языков орфографии: ru -> ru_RU, остальные — en_US.
fn lang_tag(lang: &str) -> &'static str {
    if lang.starts_with("ru") {
        "ru_RU"
    } else {
        "en_US"
    }
}

/// Слово корректно по орфографии (misspelling с позиции 0 не найден).
/// Тайм-аут главного потока трактуется как «слово неизвестно» —
/// движок продолжит по n-gram-эвристике.
pub fn spell_check(word: &str, lang: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let word = word.to_owned();
    let lang = lang.to_owned();
    let work = move || {
        let ns = NSString::from_str(&word);
        let tag = NSString::from_str(lang_tag(&lang));
        let checker = NSSpellChecker::sharedSpellChecker();
        let set_ok = checker.setLanguage(&tag);
        let range = checker.checkSpellingOfString_startingAt(&ns, 0);
        // Misspelling найден с самого начала — слово невалидно.
        let misspelled = range.location == 0;
        set_ok && !misspelled
    };
    super::main_thread::run_with_timeout(work, Duration::from_millis(150)).unwrap_or(false)
}

/// Замыкание для движка (подключается при инициализации).
pub fn hook() -> SpellChecker {
    Box::new(spell_check)
}
