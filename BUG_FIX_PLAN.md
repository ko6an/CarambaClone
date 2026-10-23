# BUG_FIX_PLAN.md — Caramba Bug Fixes (Выполнено)

## Bug 1: Синхронизация версии программы [ВЫПОЛНЕНО]

**Проблема:** В меню «О программе» (вкладка `about`) версия была захардкожена строкой `"CarambaClone 1.0"`. При этом реальная версия — `1.0.5` (указана в Cargo.toml, tauri.conf.json, package.json).
**Решение:**
- Добавлена Tauri-команда `get_app_version` в [src-tauri/src/commands.rs](file:///Users/dmitry/Downloads/CarambaClone/src-tauri/src/commands.rs), возвращающая версию из `env!("CARGO_PKG_VERSION")`.
- Команда зарегистрирована в invoke handler в [src-tauri/src/main.rs](file:///Users/dmitry/Downloads/CarambaClone/src-tauri/src/main.rs).
- Во вкладке «О программе» и в заголовке сайдбара [src/main.ts](file:///Users/dmitry/Downloads/CarambaClone/src/main.ts) версия подставляется динамически.

---

## Bug 2: Выравнивание настроек UI («пункты меню слева ровно, а то что выводят — правее») [ВЫПОЛНЕНО]

**Проблема:** Вкладки настроек были расположены сверху в один сжатый ряд; элементы внутри секций не имели общей сетки и выравнивания.
**Решение:**
- Окно настроек переработано в классический двухколоночный layout macOS Settings:
  - **Слева**: Сайдбар навигации с вертикальным списком вкладок (Основные, Исправления, Словари, Приложения, Правила, Горячие клавиши, О программе) с иконками и аккуратным выравниванием по левому краю.
  - **Справа**: Область содержимого («то что они выводят — правее»).
  - Внутри секций используется сетка `.settings-row`: слева название и описание настройки (`.settings-label`), справа — элемент управления (`.settings-control`: toggle switch, select, slider, badge), выровненный по правому краю.
- Стили и компоненты оформлены в [src/styles.css](file:///Users/dmitry/Downloads/CarambaClone/src/styles.css) и [index.html](file:///Users/dmitry/Downloads/CarambaClone/index.html).

---

## Bug 3: Кнопка «Включить/Выключить» не переключает текст и статус [ВЫПОЛНЕНО]

**Проблема:** 
1. При старте в трее создавался пункт «Включить приложение / Enable», хотя программа по умолчанию уже запущена.
2. Функции обновления пунктов трея (`refresh_enable_item`, `update_menu_texts`, `set_masking_indicator`) ошибочно вызывали `app.menu()`, возвращавший `None` (так как у Accessory-приложения нет верхнего меню окна). В итоге текст меню трея не менялся вообще никогда.
3. В окне настроек чекбокс имел статический текст и не давал явного понимания текущего состояния.
**Решение:**
- В [src-tauri/src/tray.rs](file:///Users/dmitry/Downloads/CarambaClone/src-tauri/src/tray.rs) сохраняется ссылка на меню трея `TRAY_MENU: OnceLock<Menu<Wry>>`.
- При создании трея пункт `MENU_TOGGLE` инициализируется в зависимости от `cfg.enabled` («Выключить приложение / Disable» при активном, «Включить приложение / Enable» при выключенном).
- `refresh_enable_item` обновляет текст в реальном меню трея и обновляет индикатор раскладки трея (добавляет `⊘` при отключенном режиме).
- При переключении через трей или настройки вызывается событие Tauri `enabled-changed`.
- В [src/main.ts](file:///Users/dmitry/Downloads/CarambaClone/src/main.ts) добавлена интерактивная Master Status Card:
  - Зелёный светящийся индикатор и статус «Приложение включено» + кнопка «Выключить».
  - Серый индикатор и статус «Приложение выключено» + синяя акцентная кнопка «Включить».
  - Мгновенная двусторонняя синхронизация между окном настроек и меню трея.

---

## Bug 4: Счётчик «Исправлено» не обновлялся в UI [ВЫПОЛНЕНО]

**Проблема:** В UI счётчик опечаток запрашивался только 1 раз при рендере; в трее обновлялся слишком редко.
**Решение:**
- В [src-tauri/src/platform/macos/workers.rs](file:///Users/dmitry/Downloads/CarambaClone/src-tauri/src/platform/macos/workers.rs) интервал обновления сокращён с 30 до 5 секунд.
- В [src/main.ts](file:///Users/dmitry/Downloads/CarambaClone/src/main.ts) добавлен `setInterval(refreshStats, 5000)` с очисткой по MutationObserver.
