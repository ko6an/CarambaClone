//! Выполнение замыканий в главном потоке с тайм-аутом.
//!
//! TIS (TISCreateInputSourceList, TISSelectInputSource) и NSSpellChecker
//! обязаны работать в главном потоке: HIToolbox на «чужом» потоке
//! ловит dispatch_assert_queue и процесс гибнет (SIGTRAP). Из колбэка
//! тапа вызывать их напрямую нельзя — только через этот модуль.
//!
//! Тайм-аут обязателен: вызывающий поток (тап) может держать лок движка,
//! а главный поток в этот же момент может ждать этот же лок — без
//! тайм-аута взаимоблокировка навсегда, с тайм-аутом — деградация
//! (не переключили / слово считаем неизвестным).

use std::sync::mpsc;
use std::time::Duration;

/// Выполняется ли код уже в главном потоке (рун-луп Tauri живёт в нём).
fn on_main_thread() -> bool {
    std::thread::current().name().is_some_and(|n| n == "main")
}

/// Выполнить `f` в главном потоке и вернуть её результат.
/// `None` — главный поток не ответил за `timeout` или недоступен.
pub fn run_with_timeout<F, R>(f: F, timeout: Duration) -> Option<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    if on_main_thread() {
        return Some(f());
    }
    let app = crate::tray::app_handle()?;
    let (tx, rx) = mpsc::channel();
    let job = move || {
        // Отправитель может уйти по тайм-ауту — ошибку игнорируем.
        let _ = tx.send(f());
    };
    app.run_on_main_thread(job).ok()?;
    rx.recv_timeout(timeout).ok()
}
