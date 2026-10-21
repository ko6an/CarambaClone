//! Undo-лог: журнал автоконвертаций для всех видов отмены.

/// Запись об одной автоконвертации.
#[derive(Debug, Clone, PartialEq)]
pub struct UndoRecord {
    /// Исходные нажатия (для точного повтора).
    pub presses: Vec<crate::engine::token::Press>,
    /// Как выглядело слово до конвертации (на экране).
    pub before: String,
    /// Как выглядело слово после конвертации (на экране).
    pub after: String,
    /// Длина слова в символах экрана (для backspaces).
    pub after_len: usize,
    /// Раскладка до конвертации (идентификатор TIS).
    pub prev_layout: String,
    /// Раскладка после конвертации.
    pub new_layout: String,
    /// Метка времени (мс от старта процесса).
    pub at_ms: u64,
}

/// Журнал — только память, максимум четыре последних записи.
#[derive(Debug, Default)]
pub struct UndoLog {
    records: Vec<UndoRecord>,
}

const CAPACITY: usize = 8;

impl UndoLog {
    /// Добавить запись.
    pub fn push(&mut self, rec: UndoRecord) {
        if self.records.len() >= CAPACITY {
            self.records.remove(0);
        }
        self.records.push(rec);
    }

    /// Последняя запись, откатываемая под действием `now_ms` в окне `window_ms`
    /// и при условии, что после отрисовки ничего не набрано (`buffer_empty`).
    pub fn last_revertible(
        &self,
        now_ms: u64,
        window_ms: u64,
        buffer_empty: bool,
    ) -> Option<&UndoRecord> {
        let last = self.records.last()?;
        if now_ms.saturating_sub(last.at_ms) <= window_ms && buffer_empty {
            Some(last)
        } else {
            None
        }
    }

    /// Откатить последнюю запись: действия для возврата текста и раскладки.
    pub fn revert_last(
        &mut self,
        now_ms: u64,
        window_ms: u64,
        buffer_empty: bool,
    ) -> Vec<crate::engine::Action> {
        let rec = match self.last_revertible(now_ms, window_ms, buffer_empty) {
            Some(r) => r.clone(),
            None => return Vec::new(),
        };
        self.records.pop();
        vec![
            crate::engine::Action::Replace {
                backspaces: rec.after_len,
                text: rec.before.clone(),
            },
            crate::engine::Action::SwitchLayout {
                layout: rec.prev_layout.clone(),
            },
        ]
    }

    /// Пуст ли журнал.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::token::Press;

    fn sample() -> UndoRecord {
        UndoRecord {
            presses: vec![Press {
                keycode: 0x31,
                shift: false,
                caps: false,
            }],
            before: "ghbdtn".into(),
            after: "привет".into(),
            after_len: 6,
            prev_layout: "com.apple.keylayout.US".into(),
            new_layout: "com.apple.keylayout.Russian".into(),
            at_ms: 1000,
        }
    }

    #[test]
    fn undo_within_window() {
        let mut log = UndoLog::default();
        log.push(sample());
        let acts = log.revert_last(1500, 2000, true);
        assert_eq!(acts.len(), 2);
        assert_eq!(
            acts[0],
            crate::engine::Action::Replace {
                backspaces: 6,
                text: "ghbdtn".into()
            }
        );
        assert_eq!(
            acts[1],
            crate::engine::Action::SwitchLayout {
                layout: "com.apple.keylayout.US".into()
            }
        );
        assert!(log.is_empty());
    }

    #[test]
    fn undo_expired_outside() {
        let mut log = UndoLog::default();
        log.push(sample());
        assert!(log.revert_last(5000, 2000, true).is_empty());
        assert!(!log.is_empty());
    }
}
