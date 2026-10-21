# Прогресс работ

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

- **M2 Платформа:** CGEventTap, TIS на главном потоке, печать событий,
  Secure Input, мастер разрешений, ручные хоткеи (⌥, ⌥⇧R, DoubleShift, Pause),
  флаг раскладки в трее; сверка таблиц с `UCKeyTranslate` на реальной системе.
  Локальная ручная конвертация работает в Notes и Safari.
