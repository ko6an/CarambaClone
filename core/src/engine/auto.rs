//! Автомат: события -> решения -> действия (раздел 3 ТЗ).

use super::token::{is_hard_boundary, Press, Token};
use super::undo::{UndoLog, UndoRecord};
use crate::dictionaries::ngram::NgramModel;
use crate::dictionaries::WordSet;
use crate::layouts::{classify, CharClass, Lang, Layout, Pair};
use crate::rules::{mask, MaskDirection, MaskPairs};

/// События, приходящие в движок от платформы.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// keyDown (реальные, не-синтетические события).
    KeyDown {
        keycode: u16,
        shift: bool,
        caps: bool,
        repeat: bool,
    },
    KeyUp {
        keycode: u16,
    },
    /// Смена активного приложения — буфер сбрасывается.
    AppChanged {
        bundle_id: String,
    },
    /// Смена источника ввода системой.
    LayoutChanged {
        new_layout: String,
    },
    /// Таймер.
    Tick {
        now_ms: u64,
    },
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

/// Настройки движка, важные для решений.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Минимальная длина токена (2–5, по умолчанию 3).
    pub min_len: usize,
    /// Игнорировать технические токены (цифры/символы, URL и т.п.).
    pub tech_token_enabled: bool,
    /// Проверка пароля — приоритетнее опции «цифры и символы».
    pub password_check: bool,
    /// «Маскировка» — только на выводе.
    pub masking_enabled: bool,
    /// Интенсивность маскировки 10–100.
    pub masking_intensity: u8,
    /// Окно Smart Undo, мс.
    pub undo_window_ms: u64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            min_len: 3,
            tech_token_enabled: true,
            password_check: true,
            masking_enabled: false,
            masking_intensity: 50,
            undo_window_ms: 2000,
        }
    }
}

/// Слова-исключения (не трогаются никогда, сравнение без учёта регистра).
#[derive(Debug, Default, Clone)]
pub struct Exceptions {
    words: std::collections::HashSet<String>,
}

impl Exceptions {
    pub fn new(words: Vec<String>) -> Self {
        let words = words.into_iter().map(|w| w.to_lowercase()).collect();
        Self { words }
    }

    pub fn contains(&self, word: &str) -> bool {
        let w = word.to_lowercase();
        self.words.contains(&w)
    }

    pub fn insert(&mut self, word: impl Into<String>) {
        let w: String = word.into().to_lowercase();
        self.words.insert(w);
    }
}

/// Причина решения — для журнала и теста «песочницы».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Слишком короткий/технический/исключение — не оцениваем.
    Skipped,
    /// Валидное слово текущего языка — не трогать.
    CurrentValid,
    /// Валидное слово парного языка — конвертировать.
    PairValid,
    /// Эвристика n-gram.
    Ngram,
    /// Пароль: переключиться на EN.
    Password,
}

/// Результат оценки одного токена.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub actions: Vec<Action>,
    pub reason: Reason,
    /// Интерпретация в текущей раскладке (для песочницы).
    pub cur_text: String,
    /// Интерпретация в парной раскладке.
    pub alt_text: String,
    /// Оценка n-gram текущей интерпретации.
    pub cur_score: f64,
    /// Оценка n-gram парной интерпретации.
    pub alt_score: f64,
}

/// Замыкание системной проверки орфографии (подключает платформа).
pub type SpellChecker = Box<dyn Fn(&str, &str) -> bool + Send>;

/// Детерминированный автомат ввода.
pub struct Engine {
    pair: Pair,
    config: EngineConfig,
    words_en: WordSet,
    words_ru: WordSet,
    model_en: NgramModel,
    model_ru: NgramModel,
    exceptions: Exceptions,
    undo: UndoLog,
    buffer: Vec<Press>,
    /// Язык текущей раскладки (синхронизируется по событиям системы).
    current_lang: Lang,
    /// Внешний источник валидности (NSSpellChecker) — подключает платформа.
    spell_check: Option<SpellChecker>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// Новый движок со встроенными словарями (полные пакеты — M6 «Правила»).
    pub fn new() -> Self {
        let en = crate::dictionaries::builtin_words_en();
        let ru = crate::dictionaries::builtin_words_ru();
        let mut model_en = NgramModel::new();
        model_en.train(&en.iter().map(|w| (w.clone(), 100u32)).collect::<Vec<_>>());
        let mut model_ru = NgramModel::new();
        model_ru.train(&ru.iter().map(|w| (w.clone(), 100u32)).collect::<Vec<_>>());
        Self {
            pair: crate::layouts::builtin_pair(),
            config: EngineConfig::default(),
            words_en: WordSet::new(en),
            words_ru: WordSet::new(ru),
            model_en,
            model_ru,
            exceptions: Exceptions::default(),
            undo: UndoLog::default(),
            buffer: Vec::new(),
            current_lang: Lang::En,
            spell_check: None,
        }
    }

    /// Конструктор с конкретными таблицами (для тестов и CLI).
    pub fn with_pair(pair: Pair) -> Self {
        let mut e = Self::new();
        e.pair = pair;
        e
    }

    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    pub fn set_config(&mut self, cfg: EngineConfig) {
        self.config = cfg;
    }

    pub fn set_spell_checker(&mut self, f: SpellChecker) {
        self.spell_check = Some(f);
    }

    pub fn current_lang(&self) -> Lang {
        self.current_lang
    }

    pub fn set_current_lang(&mut self, lang: Lang) {
        self.current_lang = lang;
    }

    pub fn current_layout_id(&self) -> &str {
        &self.pair.by_lang(self.current_lang).id
    }

    pub fn layout_of(&self, lang: Lang) -> &Layout {
        self.pair.by_lang(lang)
    }

    pub fn pair(&self) -> &Pair {
        &self.pair
    }

    pub fn undo_log(&self) -> &UndoLog {
        &self.undo
    }

    pub fn undo_mut(&mut self) -> &mut UndoLog {
        &mut self.undo
    }

    pub fn exceptions_mut(&mut self) -> &mut Exceptions {
        &mut self.exceptions
    }

    fn words(&self, lang: Lang) -> &WordSet {
        match lang {
            Lang::En => &self.words_en,
            Lang::Ru => &self.words_ru,
        }
    }

    fn model(&self, lang: Lang) -> &NgramModel {
        match lang {
            Lang::En => &self.model_en,
            Lang::Ru => &self.model_ru,
        }
    }

    /// Обработка события.
    pub fn on_event(&mut self, ev: Event, now_ms: u64) -> Vec<Action> {
        match ev {
            Event::AppChanged { .. } => {
                self.buffer.clear();
                Vec::new()
            }
            Event::LayoutChanged { new_layout } => {
                // Ручная смена раскладки: буфер сбрасываем, язык синхронизируем.
                self.buffer.clear();
                if new_layout.contains("Russian") {
                    self.current_lang = Lang::Ru;
                } else if new_layout.contains("keylayout.US") || new_layout.ends_with(".US") {
                    self.current_lang = Lang::En;
                }
                Vec::new()
            }
            Event::KeyDown {
                keycode,
                shift,
                caps,
                repeat,
            } => self.on_key_down(keycode, shift, caps, repeat, now_ms),
            Event::KeyUp { .. } | Event::Tick { .. } => Vec::new(),
        }
    }

    fn on_key_down(
        &mut self,
        keycode: u16,
        shift: bool,
        caps: bool,
        repeat: bool,
        now_ms: u64,
    ) -> Vec<Action> {
        // Автоповтор не оцениваем и не кладём в буфер.
        if repeat {
            return Vec::new();
        }
        // Жёсткая граница: оценить токен перед границей и пропустить клавишу.
        if is_hard_boundary(
            keycode,
            self.pair.by_lang(self.current_lang),
            self.pair.by_lang(self.pair.other(self.current_lang)),
        ) {
            let mut buffer = std::mem::take(&mut self.buffer);
            let decision = self.evaluate_boundary(&mut buffer, now_ms);
            self.buffer = buffer;
            return decision.actions;
        }
        self.buffer.push(Press {
            keycode,
            shift,
            caps,
        });
        Vec::new()
    }

    /// Оценка токена перед границей (раздел 3.2); буфер опустошается.
    pub fn evaluate_boundary(&mut self, buffer: &mut Vec<Press>, now_ms: u64) -> Decision {
        let token = Token {
            presses: std::mem::take(buffer),
        };
        self.evaluate(&token, now_ms)
    }

    /// Решение по токену (раздел 3.2 ТЗ).
    pub fn evaluate(&mut self, token: &Token, now_ms: u64) -> Decision {
        let cur = self.pair.by_lang(self.current_lang);
        let other_lang = self.pair.other(self.current_lang);
        let other = self.pair.by_lang(other_lang);

        let cur_text = token.render(cur);
        let alt_text = token.render(other);

        // Оценки считаем сразу — используются на шаге 7 и в песочнице.
        let cur_score = self.model(self.current_lang).score(&cur_text);
        let alt_score = self.model(other_lang).score(&alt_text);

        // 1. Длина меньше настройки.
        if token.len() < self.config.min_len {
            return self.result(
                Vec::new(),
                Reason::Skipped,
                &cur_text,
                &alt_text,
                cur_score,
                alt_score,
            );
        }
        // 2. Слова-исключения.
        if self.exceptions.contains(&cur_text) {
            return self.result(
                Vec::new(),
                Reason::Skipped,
                &cur_text,
                &alt_text,
                cur_score,
                alt_score,
            );
        }
        // 3. Технический токен.
        if self.config.tech_token_enabled && is_tech_token(&cur_text) {
            return self.result(
                Vec::new(),
                Reason::Skipped,
                &cur_text,
                &alt_text,
                cur_score,
                alt_score,
            );
        }

        // 4. Пароль: раньше предфильтра «цифры и символы» (решение D11).
        //    EN-интерпретация (текущая или парная): > 6 знаков, смешанный
        //    регистр и цифры/знаки одновременно.
        if self.config.password_check {
            let en_candidate = if self.current_lang == Lang::En {
                cur_text.clone()
            } else {
                alt_text.clone()
            };
            if en_candidate.chars().count() > 6 && is_password(&en_candidate) {
                let en_text = token.render(&self.pair.en);
                let actions = vec![
                    Action::Replace {
                        backspaces: cur_text.chars().count(),
                        text: en_text,
                    },
                    Action::SwitchLayout {
                        layout: self.pair.en.id.clone(),
                    },
                ];
                return self.result(
                    actions,
                    Reason::Password,
                    &cur_text,
                    &alt_text,
                    cur_score,
                    alt_score,
                );
            }
        }

        // 5. cur — валидное слово текущего языка: не трогать.
        if self.valid(core_word(&cur_text, self.current_lang), self.current_lang) {
            return self.result(
                Vec::new(),
                Reason::CurrentValid,
                &cur_text,
                &alt_text,
                cur_score,
                alt_score,
            );
        }

        // 6. alt — валидное слово парного языка: конвертировать.
        if self.valid(core_word(&alt_text, other_lang), other_lang) {
            return self.convert(
                token, &alt_text, other_lang, &cur_text, now_ms, cur_score, alt_score,
            );
        }

        // 7. Эвристика n-gram: уверенная разница и «невозможные» сочетания.
        let threshold = threshold_for(token.len());
        let cur_impossible = self
            .model(self.current_lang)
            .has_impossible_combo(&cur_text, 2);
        let alt_impossible = self.model(other_lang).has_impossible_combo(&alt_text, 2);
        if alt_score - cur_score >= threshold && (cur_impossible || alt_impossible) {
            return self.convert(
                token, &alt_text, other_lang, &cur_text, now_ms, cur_score, alt_score,
            );
        }

        self.result(
            Vec::new(),
            Reason::Skipped,
            &cur_text,
            &alt_text,
            cur_score,
            alt_score,
        )
    }

    /// Валидно ли слово: словарь -> внешний орфограф.
    fn valid(&self, word: &str, lang: Lang) -> bool {
        if word.is_empty() {
            return false;
        }
        // Слова с дефисом сравниваем по частям (частицы «что-то», «кем-то»).
        if word.contains('-') {
            let parts: Vec<&str> = word.split('-').filter(|p| !p.is_empty()).collect();
            return !parts.is_empty() && parts.iter().all(|p| self.valid_simple(p, lang));
        }
        self.valid_simple(word, lang)
    }

    fn valid_simple(&self, word: &str, lang: Lang) -> bool {
        if word.is_empty() {
            return false;
        }
        if self.words(lang).is_valid(word) {
            return true;
        }
        if let Some(f) = &self.spell_check {
            let code = match lang {
                Lang::En => "en_US",
                Lang::Ru => "ru",
            };
            return f(word, code);
        }
        false
    }

    /// Конвертация: действия, undo-запись, смена внутреннего представления.
    #[allow(clippy::too_many_arguments)]
    fn convert(
        &mut self,
        token: &Token,
        alt_text: &str,
        other_lang: Lang,
        cur_text: &str,
        now_ms: u64,
        cur_score: f64,
        alt_score: f64,
    ) -> Decision {
        let other_id = self.pair.by_lang(other_lang).id.clone();
        let cur_id = self.pair.by_lang(self.current_lang).id.clone();
        // Маскировка — только на выводе; анализ шел по немаскированному тексту.
        let final_text = if self.config.masking_enabled {
            let masks = MaskPairs::builtin();
            let mut rng = crate::rules::masking::SmallRng::new(0x5EED_0000 ^ now_ms);
            mask(
                alt_text,
                &masks,
                MaskDirection::RuToLat,
                self.config.masking_intensity,
                &mut rng,
            )
        } else {
            alt_text.to_string()
        };
        let actions = vec![
            Action::Replace {
                backspaces: cur_text.chars().count(),
                text: final_text.clone(),
            },
            Action::SwitchLayout {
                layout: other_id.clone(),
            },
        ];
        self.undo.push(UndoRecord {
            presses: token.presses.clone(),
            before: cur_text.to_string(),
            after: final_text.clone(),
            after_len: final_text.chars().count(),
            prev_layout: cur_id,
            new_layout: other_id,
            at_ms: now_ms,
        });
        let verdict = self.result(
            actions,
            Reason::PairValid,
            cur_text,
            alt_text,
            cur_score,
            alt_score,
        );
        // После конвертации внутренний язык становится парным.
        self.current_lang = other_lang;
        verdict
    }

    #[allow(clippy::too_many_arguments)]
    fn result(
        &self,
        actions: Vec<Action>,
        reason: Reason,
        cur_text: &str,
        alt_text: &str,
        cur_score: f64,
        alt_score: f64,
    ) -> Decision {
        Decision {
            actions,
            reason,
            cur_text: cur_text.to_string(),
            alt_text: alt_text.to_string(),
            cur_score,
            alt_score,
        }
    }
}

/// Порог разницы оценок по длине токена.
fn threshold_for(len: usize) -> f64 {
    match len {
        0..=2 => 4.0,
        3..=4 => 2.0,
        _ => 1.0,
    }
}

/// Слово-«ядро»: обрезаем краевые знаки (не цифры) для словаря.
fn core_word(text: &str, lang: Lang) -> &str {
    let is_letter = |c: char| match lang {
        Lang::En => classify(c) == CharClass::Latin,
        Lang::Ru => classify(c) == CharClass::Cyrillic,
    };
    text.trim_matches(|c: char| !is_letter(c))
}

/// Технический токен: цифры/символы, URL/email, camelCase/snake_case.
fn is_tech_token(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    if has_camel_or_snake(text) {
        return true;
    }
    if text.contains("://") || text.contains('@') {
        return true;
    }
    // Косая черта внутри слова.
    if text.contains('/')
        && text
            .chars()
            .any(|c| matches!(classify(c), CharClass::Latin | CharClass::Cyrillic))
    {
        return true;
    }
    // Только знаки (без единой буквы/цифры) — технический.
    let letters_or_digits = text
        .chars()
        .filter(|c| !matches!(classify(*c), CharClass::Sign))
        .count();
    letters_or_digits == 0
}

fn has_camel_or_snake(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let wall = chars
        .windows(2)
        .any(|w| w[0].is_lowercase() && w[1].is_uppercase());
    (chars.contains(&'_') && chars.len() > 2) || wall
}

/// Пароль: длина > 6, строчные + заглавные + цифры/знаки одновременно
/// (или просто оба регистра и длиннее 10).
pub fn is_password(text: &str) -> bool {
    let len = text.chars().count();
    if len <= 6 {
        return false;
    }
    let mut lower = false;
    let mut upper = false;
    let mut other = false;
    for c in text.chars() {
        match classify(c) {
            CharClass::Latin | CharClass::Cyrillic => {
                if c.is_lowercase() {
                    lower = true;
                } else {
                    upper = true;
                }
            }
            CharClass::Digit | CharClass::Sign => other = true,
            CharClass::Space => {}
        }
    }
    // Строчные, заглавные и цифры/знаки одновременно (раздел 3.2 п. 6 ТЗ).
    lower && upper && other
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Клавиши для строки, набранной в EN-раскладке.
    fn presses_en(text: &str) -> Vec<Press> {
        presses_from_char(text, Lang::En)
    }

    /// Клавиши для строки, набранной в RU-раскладке.
    fn presses_ru(text: &str) -> Vec<Press> {
        presses_from_char(text, Lang::Ru)
    }

    fn presses_from_char(text: &str, lang: Lang) -> Vec<Press> {
        let pair = crate::layouts::builtin_pair();
        let layout = pair.by_lang(lang);
        let mut out = Vec::new();
        for c in text.chars() {
            let entry = layout
                .keys
                .iter()
                .find(|e| e.lower == c.to_string() || e.upper == c.to_string())
                .unwrap_or_else(|| panic!("клавиша для {c:?} в {layout:?} не найдена"));
            out.push(Press {
                keycode: entry.kc,
                shift: entry.upper == c.to_string(),
                caps: false,
            });
        }
        out
    }

    fn token_en(text: &str) -> Token {
        Token {
            presses: presses_en(text),
        }
    }

    fn token_ru(text: &str) -> Token {
        Token {
            presses: presses_ru(text),
        }
    }

    #[test]
    fn ghbdtn_converted_to_privet() {
        let mut e = Engine::new();
        // Токен «ghbdtn» набран в EN.
        let token = token_en("ghbdtn");
        let d = e.evaluate(&token, 1000);
        assert_eq!(d.reason, Reason::PairValid);
        assert_eq!(
            d.actions,
            vec![
                Action::Replace {
                    backspaces: 6,
                    text: "привет".into()
                },
                Action::SwitchLayout {
                    layout: "com.apple.keylayout.Russian".into()
                }
            ]
        );
    }

    #[test]
    fn ckj_yj_converted_to_slozhno() {
        let mut e = Engine::new();
        let token = token_en("ckj;yj");
        let d = e.evaluate(&token, 1000);
        assert_eq!(d.reason, Reason::PairValid, "{:?}", d);
        assert_eq!(
            d.actions[0],
            Action::Replace {
                backspaces: 6,
                text: "сложно".into()
            }
        );
    }

    #[test]
    fn ruddsch_converted_to_hello() {
        let mut e = Engine::new();
        e.set_current_lang(Lang::Ru);
        // «руддщ» набран в RU-раскладке.
        let token = token_ru("руддщ");
        let d = e.evaluate(&token, 1000);
        assert_eq!(d.reason, Reason::PairValid, "{:?}", d);
        assert_eq!(
            d.actions[0],
            Action::Replace {
                backspaces: 5,
                text: "hello".into()
            }
        );
        // Раскладка переключилась на EN.
        assert_eq!(
            d.actions[1],
            Action::SwitchLayout {
                layout: "com.apple.keylayout.US".into()
            }
        );
    }

    #[test]
    fn привет_current_valid_not_touched() {
        let mut e = Engine::new();
        e.set_current_lang(Lang::Ru);
        let d = e.evaluate(&token_ru("привет"), 1000);
        assert_eq!(d.reason, Reason::CurrentValid);
        assert!(d.actions.is_empty());
    }

    #[test]
    fn hello_not_touched_in_en() {
        let mut e = Engine::new();
        let d = e.evaluate(&token_en("hello"), 1000);
        assert_eq!(d.reason, Reason::CurrentValid);
        assert!(d.actions.is_empty());
    }

    #[test]
    fn правило_пароля() {
        let mut e = Engine::new();
        e.set_current_lang(Lang::Ru);
        // Визуально «Йцукен123!» → EN-интерпретация Qwerty123!
        let token = token_ru("Йцукен123!");
        let d = e.evaluate(&token, 1000);
        assert_eq!(d.reason, Reason::Password, "{:?}", d);
        assert_eq!(
            d.actions[0],
            Action::Replace {
                backspaces: 10,
                text: "Qwerty123!".into()
            }
        );
    }

    #[test]
    fn слово_в_исключениях() {
        let mut e = Engine::new();
        e.exceptions_mut().insert("ghbdtn");
        let d = e.evaluate(&token_en("ghbdtn"), 1000);
        assert_eq!(d.reason, Reason::Skipped);
        assert!(d.actions.is_empty());
    }

    #[test]
    fn технический_токен_пропущен() {
        let mut e = Engine::new();
        let d = e.evaluate(&token_en("myvar_name"), 1000);
        assert_eq!(d.reason, Reason::Skipped);
    }

    #[test]
    fn короткие_слова_пропущены() {
        let mut e = Engine::new();
        let d = e.evaluate(&token_en("hi"), 1000);
        assert_eq!(d.reason, Reason::Skipped);
    }

    #[test]
    fn smart_undo_возврат() {
        let mut e = Engine::new();
        let token = token_en("ghbdtn");
        let _ = e.evaluate(&token, 1000);
        // ⌫ через 500 мс: буфер не пуст не должен быть.
        let undo_window = e.config().undo_window_ms;
        let acts = e.undo_mut().revert_last(1500, undo_window, true);
        assert_eq!(acts.len(), 2);
        assert_eq!(
            acts[0],
            Action::Replace {
                backspaces: 6,
                text: "ghbdtn".into()
            }
        );
        assert_eq!(
            acts[1],
            Action::SwitchLayout {
                layout: "com.apple.keylayout.US".into()
            }
        );
    }

    #[test]
    fn маскировка_на_выводе() {
        let mut e = Engine::new();
        e.set_current_lang(Lang::En);
        let mut cfg = e.config().clone();
        cfg.masking_enabled = true;
        cfg.masking_intensity = 100;
        e.set_config(cfg);
        // Печатаем в RU слово «молоко» (о→a? нет: о→o латинская).
        let token = token_ru("молоко");
        let d = e.evaluate(&token, 1000);
        assert_eq!(d.reason, Reason::PairValid);
        match &d.actions[0] {
            Action::Replace { text, .. } => {
                // Все «о» заменены латинским o.
                let replaced: Vec<char> = text.chars().collect();
                assert_eq!(replaced, vec!['м', '\u{6F}', 'л', '\u{6F}', 'к', '\u{6F}']);
            }
            other => panic!("ожидалась Replace, получено {other:?}"),
        }
    }
}
