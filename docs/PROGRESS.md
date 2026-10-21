# Прогресс работ

## M2 Платформа — готово (локальная проверка)

**Дата:** 2026-10-21

Сделано:

- **CGEventTap** (session, фильтрующий, отдельный поток с CFRunLoop):
  keyDown/keyUp/flagsChanged/левый клик; синтетика помечается
  `CARAMBA_MAGIC` в `kCGEventSourceUserData` и пропускается; сторож
  включает тап заново при выключении системой + обработка DisabledByTimeout.
- Печать: backspaces, текст юникод-чанками (≤20 UTF-16), хоткеи ⌘C/⌘V
  (`CGEventKeyboardSetUnicodeString`; CGEventRef освобождается CFRelease).
- Ручные хоткеи M2: ⌥⇧R (конвертация выделенного через буфер обмена,
  восстановление текста), одиночный Shift-tap (смена раскладки), Option-tap
  (конвертация текущего слова / откат), Pause=F15 (откат).
- TIS: выбор источника по id, `current_input_source_id`; Darwin-подписка
  `AppleSelectedInputChanged` (на главном рун-лупе) — синхронизация движка
  и флага раскладки в трее (short_name, `tray.set_title`).
- NSWorkspace (frontmost через objc2-app-kit), NSPasteboard, AX
  (`AXIsProcessTrustedWithOptions`, `CGPreflight{Listen,Post}EventAccess`),
  Secure Input (`IsSecureEventInputEnabled`, опрос 250 мс; при входе — сброс
  буфера и passthrough).
- Общий движок для тапа и команд (`engine_state`), маппинг настроек:
  aggressiveness → min_len (5/3/2), ignore_digits_symbols → tech-токен,
  включение/выключение, autoswitch, dry-run.
- Команды UI: `permissions_status`, `open_permissions_pane`, `convert_current_word`,
  `set_enabled_cmd`; вкладка «Разрешения» в настройках с кнопками открытия панелей
  (Accessibility / Input Monitoring).
- Core-мелочи: `flip_text`/`flip_auto` (снапшот-конверт текста), 
  `Engine::evaluate_current_word`, `buffer_is_empty`, тесты дисциплины таблиц
  RU vs Russian-PC (+8 новых тестов, всего 58+).
- Проверено: fmt/clippy/test/workspace, `npm run build`, прожект запускается:
  тап стартует, без разрешений ожидаемо `CGEventTapCreate вернул NULL` —
  мастер разрешений открыт, после ручного подтверждения Accessibility и
  Input Monitoring + перезапуска ручная проверка на тексте (Notes/Safari).

Не перенесено в M2: восстановление нетекстовых буферов обмена (разные структуры),
⌥⇧R для повторного «сохранения формата» — M3; тапuto-фикс от fast path при
Secure Input: passthrough — уже.

## M1 Ядро — готово (локальная проверка)

**Дата:** 2026-10-21

Сделано:

- Таблицы раскладок: `resources/layouts/{us,ru,ruwin}.toml` сгенерированы из
  **реальных системных таблиц macOS** (выгрузка через `UCKeyTranslate` локальным
  дампером; разница «Russian» и «Russian – PC» фиксируется: цифровой ряд Shift,
  кавычки клавиши `0x0A` и ё). Язык таблиц — данные, пары собираются в `Pair`.
- Токенизация: `Press (keycode, shift, caps)`, жёсткие границы (навигация,
  пробел, клавиши знаков в обеих раскладках; цифры входят в токен — D10),
  неоднозначные клавиши и краевые варианты (EdgeVariant).
- Словари: `fst::Set` + встроенные EN/RU списки по 30 тыс. частотных слов
  (hermitdave/FrequencyWords, CC BY-SA 4.0 — THIRD_PARTY_NOTICES.md) +
  приёмочные дополнения; подключение внешнего орфографа (NSSpellChecker — M3).
- n-gram модель: символьные 3-граммы, веса и «невозможные» сочетания;
  эврика: score(alt) − score(cur) ≥ τ с порогом по длине.
- Движок: автомат с предфильтрами (длина → исключения → технический токен),
  **пароль проверяется приоритетно** (D11), решения с 7 уровнями раздела 3.2,
  маскировка применяяется только на выводе; интерфейс Decision с оценками
  (песочница `--replay`).
- Undo-лог: записи конвертаций, откат последней (окно по времени, буфер пуст).
- CLI: `cargo run -p caramba-core --example replay -- [-l en|ru] 'текст'`.
- Тесты: 32 (юнит + приёмочные таблицы ghbdtn/ckj;yj/руддщ/пароль/исключения,
  маскировка, undo, n-gram) + proptest round-trip таблиц; бенчмарки criterion
  (анализ слова ~1.4–2.1 µs, старт движка ~30 мс).

Не перенесено в M1: `руддщ → hello` — работает по словарю; проверка орфографии
NSSpellChecker подключается в M3. Приёмочные с опечатками/«фокусами»/ёфикатором —
после M4.

## M0 Каркас — готово (локальная проверка)

**Дата:** 2026-10-21

Сделано:

- Cargo workspace: `core` (caramba-core), `src-tauri` (caramba-app), `tools/rules-builder` (пусто).
- Каркас `core`: модули engine/layouts/rules/config; схема `Config` с `schema_version`,
  атомарная запись, обработка повреждённого файла (.bak), тесты round-trip.
- Платформенный слой: `trait Platform`, тонкий FFI по TIS, реализация
  `current_layout`/`select_layout`; остальные методы — заглушки до M2/M3.
- Tauri: трей с иконкой-шаблоном и меню, окно настроек по требованию,
  `ActivationPolicy::Accessory` (нет иконки в Dock), single-instance,
  команды get/set_config, лог в `~/Library/Logs/CarambaClone/debug.log`.
- Фронтенд: Vite + TypeScript + Tailwind v4, каркас страницы настроек.
- Иконки: .icns приложения, трей-шаблон (монохромная клавиатура) @1x/@2x.
- `tauri.conf.json`: ad-hoc подпись (`signingIdentity: "-"`), минимум macOS 13.
- CI: `ci.yml`, `release.yml`; README ru/en (в т.ч. `xattr -dr com.apple.quarantine`).

Не сделано (переносится): цветов флагов в трее нет (только шаблон-иконка);
`rules-builder` пуст; i18n-модуль TS ждёт M5.

Критерий M0: тег `v0.0.1` собирается в Actions в рабочий `.app` (arm64),
иконка видна в строке меню.

**Статус проверки M0:** `cargo fmt --check`, `cargo clippy --workspace --all-targets
-- -D warnings`, `cargo test --workspace`, `npm run build`, `npx tauri build`
прижались локально: `CarambaClone.app` (arm64, ad-hoc подпись проверена
`codesign --verify --deep --strict`) и `CarambaClone_0.0.1_aarch64.dmg`.
Тег `v0.0.1` создан локально; в GitHub Actions проверяется при первом push
(удалённый репозиторий не настроен).

## Следующий этап

- **M3 Автопереключение:** NSSpellChecker на главном потоке + словарная
  валидность через движок,工程有限公司 приёмы исключений из UI, Smart Undo
  полноценно (⌫ и DoubleShift), профили приложений; локальная ручная
  проверка сценариев раздела 13.

- **M2 канал ручных проверок:** после выдачи разрешений проверяется:
  печатаем `ghbdtn␣` → `привет ` в Notes; `руддщ␣` → `hello`; Shift-tap
  между US/Russian; ⌥⇧R конвертация выделенного; Pause/F15 откат.
  Настоящая проверка выполняется пользователем (разрешения вручную).

## M2 «Разрешения»: что нужно выдать

1. Системные настройки → Конфиденциальность → Accessibility —
   разрешить CarambaClone/CarambaClone.app.
2. Конфиденциальность → Input Monitoring — разрешить (для перехвата
   событий клавиатуры, используется «наблюдение» + «порождение»).
3. Перезапустить приложение после выдачи.
