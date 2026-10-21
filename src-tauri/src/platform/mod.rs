//! Платформенный слой: trait Platform и заглушки для не-macOS.

pub mod macos;

/// Абстракция над системой; в тестах подставляется мок.
/// Полный набор задействуется в M2/M3; пока часть методов не вызывается.
#[allow(dead_code)]
pub trait Platform {
    /// Идентификатор текущего источника ввода.
    fn current_layout(&self) -> String;
    /// Выбрать источник ввода по идентификатору.
    fn select_layout(&self, id: &str) -> Result<(), String>;
    /// Послать нажатия клавиш (для замены).
    fn post_keys(&self, keys: Vec<(u16, bool)>) -> Result<(), String>;
    /// Напечатать текст юникод-строкой.
    fn post_text(&self, text: &str) -> Result<(), String>;
    /// Bundle id активного приложения.
    fn frontmost_app(&self) -> String;
    /// Включён ли защищённый ввод.
    fn is_secure_input(&self) -> bool;
    /// Проверка орфографии.
    fn spell_check(&self, word: &str, lang: &str) -> bool;
    /// Прочитать текст из буфера обмена.
    fn clipboard_get(&self) -> String;
    /// Записать текст в буфер обмена.
    fn clipboard_set(&self, text: &str);
    /// Проиграть звук по имени.
    fn play_sound(&self, name: &str);
}

/// Экземпляр платформы для текущей ОС.
pub fn create_default() -> Box<dyn Platform + Send> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacosPlatform::new());
    #[cfg(not(target_os = "macos"))]
    return Box::new(macos::stubs::StubPlatform);
}
