//! Автомат ввода: события, действия, буфер.

/// События, приходит в движок от платформы.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// keyDown: код клавиши, shift, caps lock.
    KeyDown {
        keycode: u16,
        shift: bool,
        caps: bool,
    },
    /// keyUp границ клавиш — нужен для «быстрого пути» и Shift-тапов.
    KeyUp { keycode: u16, shift: bool },
    /// Сменилось активное приложение.
    AppChanged { bundle_id: String },
    /// Сменился источник ввода.
    LayoutChanged { new_layout: String },
    /// Тик для таймеров движка.
    Tick,
}

/// Действия движка.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Событие проходит в приложение как есть.
    Pass,
    /// Событие поглощается.
    Swallow,
    /// Заменить текст: стереть `backspaces` символов и напечатать `text`.
    Replace { backspaces: usize, text: String },
    /// Переключить раскладку.
    SwitchLayout { layout: String },
    /// Проиграть звук.
    Sound,
    /// Неприрывающая панель-подсказка.
    Hint { text: String },
}
