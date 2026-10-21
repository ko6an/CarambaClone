//! Автомат ввода: события, действия, буфер, решения (раздел 3 ТЗ).

mod auto;
pub mod token;
pub mod undo;

pub use auto::{
    is_password, Action, Decision, Engine, EngineConfig, Event, Exceptions, Reason, SoundKind,
    SpellChecker,
};
pub use token::{is_hard_boundary, Press, Token};
pub use undo::{UndoLog, UndoRecord};
