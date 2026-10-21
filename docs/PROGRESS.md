# Прогресс работ

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

**Статус проверки:** `cargo fmt --check`, `cargo clippy --workspace --all-targets
-- -D warnings`, `cargo test --workspace`, `npm run build`, `npx tauri build`
прижались локально: `CarambaClone.app` (arm64, ad-hoc подпись проверена
`codesign --verify --deep --strict`) и `CarambaClone_0.0.1_aarch64.dmg`.
Тег `v0.0.1` создан локально; в GitHub Actions проверяется при первом push
(удалённый репозиторий не настроен).

## Следующий этап

- **M1 Ядро:** таблицы раскладок EN/RU/RU-PC, токенизация, конвертация,
  FST-словари + n-gram модель, проверка паролей, undo-лог, `mask()`,
  CLI `replay`; приёмочные примеры из раздела 13, не требующие ОС.
