// Точка входа интерфейса: вкладки настроек, i18n (ru/en на лету).
import { invoke } from "@tauri-apps/api/core";
import type { Config, AppProfile } from "./lib/types";
import { setSystemLang, t } from "./lib/i18n";

setSystemLang(navigator.languages);

const statusEl = document.getElementById("status");
const tabsEl = document.getElementById("tabs");

const TABS = [
  { id: "general", key: "tab.general" },
  { id: "fixes", key: "tab.fixes" },
  { id: "dictionary", key: "tab.dictionary" },
  { id: "apps", key: "tab.apps" },
  { id: "rules", key: "tab.rules" },
  { id: "hotkeys", key: "tab.hotkeys" },
  { id: "about", key: "tab.about" },
] as const;

let activeTabId: string = "general";
void activeTabId;

function selectTab(id: string): void {
  const panel = document.getElementById("panel");
  if (!panel) return;
  activeTabId = id;
  panel.innerHTML = "";
  if (id === "general") {
    void renderGeneral(panel);
    return;
  }
  if (id === "fixes") {
    void renderFixes(panel);
    return;
  }
  if (id === "dictionary") {
    void renderDictionary(panel);
    return;
  }
  if (id === "apps") {
    void renderApps(panel);
    return;
  }
  if (id === "rules") {
    void renderRules(panel);
    return;
  }
  const p = document.createElement("p");
  p.className = "mt-4 text-sm";
  if (id === "hotkeys") {
    p.textContent =
      "⌃⌥M — Маскировка; DoubleShift — отмена конвертации; ⌥ (тап) — отмена/конвертация; ⌥⇧R — конвертация выделения; ⌫ (2 с после исправления) — Smart Undo.";
  } else if (id === "about") {
    p.textContent =
      "CarambaClone 1.0 — умный автопереключатель раскладки. Лицензии: словари см. THIRD_PARTY_NOTICES.md; иконка Anonymous — Public Domain.";
  }
  panel.appendChild(p);
}

interface BoolToggle {
  key: keyof Config;
  title: string;
}

const FIX_TOGGLES: Array<BoolToggle> = [
  { key: "typos_fix", title: "Опечатки: перестановки, пропуск, удвоение" },
  { key: "trick_unit_sign", title: "Фокус 2: привет1 → привет!" },
  { key: "trick_stuck_words", title: "Фокус 3: слипшиеся слова (сонатуБаха)" },
  { key: "trick_brackets", title: "Фокус 4: скобки без Shift (9/0)" },
  { key: "trick_trailing_signs", title: "Фокус 5: знак в конце (б, ю)" },
  { key: "trick_cap_excl", title: "Фокус 6: заглавная Ё → «!»" },
  { key: "trick_particles", title: "Фокус 7: частицы (чтото → что-то)" },
  { key: "yo_fixer", title: "Ёфикатор (однозначные «е» → «ё»)" },
];

async function renderFixes(el: HTMLElement): Promise<void> {
  let cfg: Config;
  try {
    cfg = await invoke<Config>("get_config");
  } catch (e) {
    el.textContent = `Ошибка: ${String(e)}`;
    return;
  }
  const head = document.createElement("h2");
  head.className = "text-lg font-semibold mt-4";
  head.textContent = "Исправления";
  el.appendChild(head);
  for (const t of FIX_TOGGLES) {
    const row = document.createElement("label");
    row.className = "flex items-center gap-2 mt-2 text-sm";
    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = cfg[t.key] as boolean;
    box.addEventListener("change", () => {
      void (async () => {
        (cfg[t.key] as boolean) = box.checked;
        try {
          await invoke("set_config", { config: cfg });
        } catch (e) {
          box.checked = !box.checked;
          console.error(String(e));
        }
      })();
    });
    const span = document.createElement("span");
    span.textContent = t.title;
    row.append(box, span);
    el.appendChild(row);
  }
  const maskTitle = document.createElement("p");
  maskTitle.className = "mt-4 font-medium text-sm";
  maskTitle.textContent = "Маскировка (⌃⌥M)";
  el.appendChild(maskTitle);
  appendMaskControl(el, "Включена (⌃⌥M)", "masking_on", cfg);
  appendMaskControl(el, "Запоминать состояние между запусками", "masking_remember", cfg);
  const intensityRow = document.createElement("div");
  intensityRow.className = "flex items-center gap-2 mt-2 text-sm";
  const intensity = document.createElement("input");
  intensity.type = "range";
  intensity.min = "10";
  intensity.max = "100";
  intensity.step = "5";
  intensity.value = String(cfg.masking_intensity);
  const intensityLabel = document.createElement("span");
  intensityLabel.textContent = `Интенсивность: ${cfg.masking_intensity}%`;
  intensity.addEventListener("change", () => {
    void (async () => {
      const v = Number(intensity.value);
      cfg.masking_intensity = v;
      intensityLabel.textContent = `Интенсивность: ${v}%`;
      try {
        await invoke("set_config", { config: cfg });
      } catch {
        intensity.value = String(cfg.masking_intensity);
      }
    })();
  });
  intensityRow.append(intensity, intensityLabel);
  el.appendChild(intensityRow);
  const dirRow = document.createElement("div");
  dirRow.className = "flex items-center gap-2 mt-2 text-sm";
  const dirSel = document.createElement("select");
  for (const [value, label] of [
    ["ru_to_lat", "Русские буквы → латинские двойники"],
    ["lat_to_ru", "Латинские буквы → русские двойники"],
  ] as const) {
    const opt = document.createElement("option");
    opt.value = value;
    opt.textContent = label;
    dirSel.appendChild(opt);
  }
  dirSel.value = cfg.masking_direction;
  dirSel.addEventListener("change", () => {
    void (async () => {
      cfg.masking_direction = dirSel.value as Config["masking_direction"];
      try {
        await invoke("set_config", { config: cfg });
      } catch {
        dirSel.value = cfg.masking_direction;
      }
    })();
  });
  const dirLabel = document.createElement("span");
  dirLabel.textContent = "Направление:";
  dirRow.append(dirLabel, dirSel);
  el.appendChild(dirRow);
}

function appendMaskControl(
  el: HTMLElement,
  title: string,
  key: "masking_on" | "masking_remember",
  cfg: Config,
): void {
  const row = document.createElement("label");
  row.className = "flex items-center gap-2 mt-2 text-sm";
  const box = document.createElement("input");
  box.type = "checkbox";
  box.checked = Boolean(cfg[key]);
  box.addEventListener("change", () => {
    void (async () => {
      cfg[key] = box.checked;
      try {
        if (key === "masking_on") {
          await invoke("set_masking_cmd", { on: box.checked });
        } else {
          await invoke("set_config", { config: cfg });
        }
      } catch {
        box.checked = !box.checked;
      }
    })();
  });
  const span = document.createElement("span");
  span.textContent = title;
  row.append(box, span);
  el.appendChild(row);
}

// MARK: — Вкладка «Основные»

const GENERAL_TOGGLES: Array<BoolToggle> = [
  { key: "autoswitch", title: "Автопереключение раскладки" },
  { key: "sound_on_switch", title: "Звук при переключении" },
  { key: "sound_on_fix", title: "Звук при исправлении" },
  { key: "smart_backspace_undo", title: "Умная отмена по ⌫ (2 с после исправления)" },
  { key: "ignore_digits_symbols", title: "Игнорировать слова с цифрами и символами" },
  { key: "fix_on_enter", title: "Исправлять по Enter" },
  { key: "games_folder_watch", title: "Папка Games отключает автоматику" },
  { key: "exclude_games_category", title: "Игры (категория) исключать автоматически" },
];

export interface PermissionsStatus {
  accessibility: boolean;
  input_monitoring: boolean;
  posting: boolean;
  secure_input: boolean;
  tap_active: boolean;
}

async function renderGeneral(el: HTMLElement): Promise<void> {
  let cfg: Config;
  try {
    cfg = await invoke<Config>("get_config");
  } catch (e) {
    el.textContent = `Ошибка: ${String(e)}`;
    return;
  }
  const enabledRow = document.createElement("label");
  enabledRow.className = "flex items-center gap-2 mt-4 text-sm font-medium";
  const enabledBox = document.createElement("input");
  enabledBox.type = "checkbox";
  enabledBox.checked = cfg.enabled;
  enabledBox.addEventListener("change", () => {
    void (async () => {
      cfg.enabled = enabledBox.checked;
      try {
        await invoke("set_config", { config: cfg });
        if (statusEl) {
          statusEl.textContent = cfg.enabled
            ? "Движок включён."
            : "Движок выключен.";
        }
      } catch {
        enabledBox.checked = !enabledBox.checked;
      }
    })();
  });
  enabledRow.append(enabledBox, document.createTextNode("Включить приложение"));
  el.appendChild(enabledRow);

  const langRow = document.createElement("div");
  langRow.className = "flex items-center gap-2 mt-4 text-sm";
  const langSel = document.createElement("select");
  for (const [value, label] of [
    ["system", "Язык интерфейса: системный"],
    ["ru", "Язык интерфейса: русский"],
    ["en", "Язык интерфейса: English"],
  ] as const) {
    const opt = document.createElement("option");
    opt.value = value;
    opt.textContent = label;
    langSel.appendChild(opt);
  }
  langSel.value = cfg.ui_lang;
  langSel.addEventListener("change", () => {
    void (async () => {
      cfg.ui_lang = langSel.value;
      try {
        await invoke("set_config", { config: cfg });
        renderTabs();
        void selectTab("general");
      } catch {
        langSel.value = cfg.ui_lang;
      }
    })();
  });
  langRow.append(langSel);
  el.appendChild(langRow);

  for (const gt of GENERAL_TOGGLES) {
    const row = document.createElement("label");
    row.className = "flex items-center gap-2 mt-2 text-sm";
    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = cfg[gt.key] as boolean;
    box.addEventListener("change", () => {
      void (async () => {
        (cfg[gt.key] as boolean) = box.checked;
        try {
          await invoke("set_config", { config: cfg });
        } catch {
          box.checked = !box.checked;
        }
      })();
    });
    row.append(box, document.createTextNode(gt.title));
    el.appendChild(row);
  }

  // Автозапуск (SMAppService).
  const autoRow = document.createElement("div");
  autoRow.className = "flex items-center gap-2 mt-4 text-sm";
  const autoBox = document.createElement("input");
  autoBox.type = "checkbox";
  autoBox.checked = cfg.autostart;
  const autoLabel = document.createElement("span");
  autoLabel.textContent = "Запуск при входе в систему";
  autoBox.addEventListener("change", () => {
    void (async () => {
      try {
        await invoke("autostart_set", { on: autoBox.checked });
        cfg.autostart = autoBox.checked;
      } catch (e) {
        autoBox.checked = !autoBox.checked;
        try {
          const st = await invoke<{ status: string; registered: boolean }>(
            "autostart_status",
          );
          autoLabel.textContent = `Запуск при входе (статус: ${st.status})`;
        } catch {
          console.error(String(e));
        }
      }
    })();
  });
  autoRow.append(autoBox, autoLabel);
  el.appendChild(autoRow);
  void invoke("autostart_status")
    .then((st) => {
      autoLabel.textContent = `Запуск при входе (статус: ${(st as { status: string }).status})`;
    })
    .catch(() => {});

  // Счётчик и диагностика.
  const statsP = document.createElement("p");
  statsP.className = "mt-4 text-sm";
  el.appendChild(statsP);
  const refreshStats = (): void => {
    void invoke<[number, number]>("stats_cmd")
      .then(([today, total]) => {
        statsP.textContent = `Исправлено сегодня / всего: ${today} / ${total}`;
      })
      .catch(() => {});
  };
  refreshStats();

  const diagRow = document.createElement("div");
  diagRow.className = "flex items-center gap-2 mt-2 text-sm";
  const logBtn = document.createElement("button");
  logBtn.textContent = "Открыть журнал";
  logBtn.className =
    "rounded px-3 py-1 bg-neutral-200 dark:bg-neutral-800 hover:bg-neutral-300 dark:hover:bg-neutral-700";
  logBtn.addEventListener("click", () => void invoke("open_log_cmd"));
  diagRow.append(logBtn);
  el.appendChild(diagRow);
  const permBtn = document.createElement("button");
  permBtn.textContent = "Проверить разрешения";
  permBtn.className =
    "rounded px-3 py-1 mt-2 bg-neutral-200 dark:bg-neutral-800 hover:bg-neutral-300 dark:hover:bg-neutral-700";
  permBtn.addEventListener("click", () => {
    void invoke<PermissionsStatus>("permissions_status")
      .then((st) => {
        const lines = [
          `Accessibility: ${st.accessibility ? "выдано" : "не выдано"}`,
          `Input Monitoring: ${st.input_monitoring ? "выдано" : "не выдано"}`,
          `Тап: ${st.tap_active ? "работает" : "не создан"}`,
          `Secure Input активен: ${st.secure_input ? "да" : "нет"}`,
        ];
        statsP.textContent = lines.join(" · ");
      })
      .catch((e) => {
        statsP.textContent = `Ошибка: ${String(e)}`;
      });
  });
  el.appendChild(permBtn);
}

// MARK: — Вкладка «Правила» (подписанный пакет данных, раздел 8 ТЗ)

interface RulesInfo {
  version: string;
  date: string;
  installed: boolean;
  words_ru: number;
  words_en: number;
  app_exceptions: number;
  last_check_secs_ago: number | null;
}

async function renderRules(el: HTMLElement): Promise<void> {
  let info: RulesInfo;
  try {
    info = await invoke<RulesInfo>("rules_info");
  } catch (e) {
    el.textContent = `Ошибка: ${String(e)}`;
    return;
  }
  const head = document.createElement("h2");
  head.className = "text-lg font-semibold mt-4";
  head.textContent = "Пакет «Правил»";
  el.appendChild(head);
  const desc = document.createElement("p");
  desc.className = "text-sm text-neutral-500 mt-1";
  desc.textContent =
    "Словари, n-gram модель, таблицы правил и стартовый список исключений. Обновляется раз в неделю; пакет подписывается Ed25519 и проверяется приложением. Единственный сетевой запрос — скачивание файла пакета.";
  el.appendChild(desc);

  const statusP = document.createElement("p");
  statusP.className = "mt-3 text-sm";
  statusP.textContent =
    `${info.installed ? "Установлен пакет от" : "Встроенный пакет"}: ${info.version}. Словари: RU ${info.words_ru}, EN ${info.words_en}; исключений приложений: ${info.app_exceptions}.`;
  if (info.last_check_secs_ago !== null && info.installed) {
    const days = Math.floor(info.last_check_secs_ago / 86400);
    statusP.textContent += ` Последняя проверка: ${days} дн. назад.`;
  }
  el.appendChild(statusP);

  const row = document.createElement("div");
  row.className = "flex items-center gap-2 mt-3";
  const checkBtn = document.createElement("button");
  checkBtn.textContent = "Проверить сейчас";
  checkBtn.className =
    "rounded px-3 py-1 bg-neutral-200 dark:bg-neutral-800 hover:bg-neutral-300 dark:hover:bg-neutral-700 text-sm";
  const stateP = document.createElement("span");
  stateP.className = "text-sm text-neutral-500";
  stateP.textContent = "";
  checkBtn.addEventListener("click", () => {
    stateP.textContent = "Проверка…";
    void (async () => {
      try {
        await invoke<RulesInfo>("rules_check_now");
        stateP.textContent = "Обновлено; пакет применён.";
        void renderRules(el.parentElement ?? el);
      } catch (e) {
        stateP.textContent = `Остаться на текущем пакете: ${String(e)}`;
      }
    })();
  });
  const rollbackBtn = document.createElement("button");
  rollbackBtn.textContent = "Откатить к встроенному";
  rollbackBtn.className =
    "rounded px-3 py-1 bg-neutral-200 dark:bg-neutral-800 hover:bg-neutral-300 dark:hover:bg-neutral-700 text-sm";
  rollbackBtn.addEventListener("click", () => {
    void (async () => {
      try {
        await invoke("rules_rollback");
        void renderRules(el.parentElement ?? el);
      } catch (e) {
        console.error(String(e));
      }
    })();
  });
  row.append(checkBtn, rollbackBtn, stateP);
  el.appendChild(row);

  const autoRow = document.createElement("label");
  autoRow.className = "flex items-center gap-2 mt-3 text-sm";
  const autoBox = document.createElement("input");
  autoBox.type = "checkbox";
  autoBox.checked = await invoke<boolean>("get_rules_autoupdate");
  autoBox.addEventListener("change", () => {
    void (async () => {
      try {
        await invoke("set_rules_autoupdate", { on: autoBox.checked });
      } catch {
        autoBox.checked = !autoBox.checked;
      }
    })();
  });
  autoRow.append(autoBox, document.createTextNode("Автообновление раз в неделю"));
  el.appendChild(autoRow);
}

// MARK: — Вкладка «Словарь» (слова-исключения, FR-EXC-1)

async function renderDictionary(el: HTMLElement): Promise<void> {
  let cfg: Config;
  try {
    cfg = await invoke<Config>("get_config");
  } catch (e) {
    el.textContent = `Ошибка: ${String(e)}`;
    return;
  }
  const head = document.createElement("h2");
  head.className = "text-lg font-semibold mt-4";
  head.textContent = "Слова-исключения";
  el.appendChild(head);
  const hint = document.createElement("p");
  hint.className = "text-sm text-neutral-500 mt-1";
  hint.textContent =
    "Эти слова никогда не исправляются (регистр не важен): команды, никнеймы, аббревиатуры.";
  el.appendChild(hint);

  const search = document.createElement("input");
  search.placeholder = "Поиск…";
  search.className =
    "mt-2 rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm";
  const list = document.createElement("div");
  list.className = "mt-2 flex flex-col gap-1 max-h-64 overflow-auto";
  el.append(search, list);

  const renderList = (filter: string): void => {
    list.innerHTML = "";
    const f = filter.trim().toLowerCase();
    cfg.exceptions
      .filter((w) => f === "" || w.toLowerCase().includes(f))
      .forEach((word, idx) => {
        const row = document.createElement("div");
        row.className = "flex items-center justify-between gap-2 text-sm";
        const span = document.createElement("span");
        span.textContent = word;
        const del = document.createElement("button");
        del.textContent = "✕";
        del.title = "Удалить";
        del.addEventListener("click", () => {
          cfg.exceptions.splice(idx, 1);
          void (async () => {
            try {
              await invoke("exceptions_set_cmd", { words: cfg.exceptions });
              renderList(search.value);
            } catch (e) {
              console.error(String(e));
            }
          })();
        });
        row.append(span, del);
        list.appendChild(row);
      });
  };
  renderList("");
  search.addEventListener("input", () => renderList(search.value));

  const addRow = document.createElement("div");
  addRow.className = "flex items-center gap-2 mt-2";
  const addInput = document.createElement("input");
  addInput.placeholder = "новое слово…";
  addInput.className =
    "rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm";
  const addBtn = document.createElement("button");
  addBtn.textContent = "Добавить";
  addBtn.className =
    "rounded px-3 py-1 bg-neutral-200 dark:bg-neutral-800 hover:bg-neutral-300 dark:hover:bg-neutral-700 text-sm";
  const addWord = (): void => {
    const w = addInput.value.trim();
    if (w === "") return;
    void (async () => {
      try {
        await invoke("exceptions_set_cmd", { words: [...cfg.exceptions, w] });
        cfg.exceptions.push(w);
        addInput.value = "";
        renderList(search.value);
      } catch (e) {
        console.error(String(e));
      }
    })();
  };
  addBtn.addEventListener("click", addWord);
  addInput.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter") addWord();
  });
  addRow.append(addInput, addBtn);
  el.appendChild(addRow);

  const buttonsRow = document.createElement("div");
  buttonsRow.className = "flex items-center gap-2 mt-4";
  const resetBtn = document.createElement("button");
  resetBtn.textContent = "Вернуть стандартный список";
  resetBtn.className =
    "rounded px-3 py-1 bg-neutral-200 dark:bg-neutral-800 hover:bg-neutral-300 dark:hover:bg-neutral-700 text-sm";
  resetBtn.addEventListener("click", () => {
    void (async () => {
      try {
        const def = await invoke<string[]>("exceptions_default_cmd");
        await invoke("exceptions_set_cmd", { words: def });
        cfg.exceptions = def;
        renderList(search.value);
      } catch (e) {
        console.error(String(e));
      }
    })();
  });
  buttonsRow.append(resetBtn);
  el.appendChild(buttonsRow);
}

// MARK: — Вкладка «Приложения» (FR-EXC-3…5)

async function renderApps(el: HTMLElement): Promise<void> {
  let cfg: Config;
  try {
    cfg = await invoke<Config>("get_config");
  } catch (e) {
    el.textContent = `Ошибка: ${String(e)}`;
    return;
  }
  const head = document.createElement("h2");
  head.className = "text-lg font-semibold mt-4";
  head.textContent = "Профили приложений";
  el.appendChild(head);
  const hint = document.createElement("p");
  hint.className = "text-sm text-neutral-500 mt-1";
  hint.textContent =
    "Bundle id допускает префикс с «*»: com.jetbrains.*. Режим: выключено / только вручную / авто.";
  el.appendChild(hint);

  const list = document.createElement("div");
  list.className = "mt-2 flex flex-col gap-2";
  el.appendChild(list);

  const renderList = (): void => {
    list.innerHTML = "";
    for (const p of cfg.app_profiles) {
      const row = document.createElement("div");
      row.className = "flex items-center gap-2 text-sm";
      const name = document.createElement("span");
      name.className = "flex-1 font-mono";
      name.textContent = p.bundle_id;
      const sel = document.createElement("select");
      for (const [v, label] of [
        ["off", "Выключено"],
        ["manual_only", "Только вручную"],
        ["auto", "Авто"],
      ] as const) {
        const opt = document.createElement("option");
        opt.value = v;
        opt.textContent = label;
        sel.appendChild(opt);
      }
      sel.value = p.mode;
      sel.addEventListener("change", () => {
        p.mode = sel.value as AppProfile["mode"];
        void invoke("app_profiles_set_cmd", { profiles: cfg.app_profiles });
      });
      const del = document.createElement("button");
      del.textContent = "✕";
      del.addEventListener("click", () => {
        cfg.app_profiles = cfg.app_profiles.filter(
          (x) => x.bundle_id !== p.bundle_id,
        );
        void invoke("app_profiles_set_cmd", { profiles: cfg.app_profiles });
        renderList();
      });
      row.append(name, sel, del);
      list.appendChild(row);
    }
  };
  renderList();

  const addRow = document.createElement("div");
  addRow.className = "flex items-center gap-2 mt-2";
  const addInput = document.createElement("input");
  addInput.placeholder = "com.example.app";
  addInput.className =
    "rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm font-mono";
  const addBtn = document.createElement("button");
  addBtn.textContent = "Добавить";
  addBtn.className =
    "rounded px-3 py-1 bg-neutral-200 dark:bg-neutral-800 hover:bg-neutral-300 dark:hover:bg-neutral-700 text-sm";
  addBtn.addEventListener("click", () => {
    const id = addInput.value.trim();
    if (id === "") return;
    const profile: AppProfile = {
      bundle_id: id,
      mode: "off",
      layout_policy: "remember",
      autoswitch: null,
      typos_fix: null,
      tricks: null,
      yo_fixer: null,
      masking: null,
      sound: null,
    };
    cfg.app_profiles.push(profile);
    void invoke("app_profiles_set_cmd", { profiles: cfg.app_profiles });
    addInput.value = "";
    renderList();
  });
  addRow.append(addInput, addBtn);
  el.appendChild(addRow);

  const gamesRow = document.createElement("div");
  gamesRow.className = "mt-4";
  const gamesBtn = document.createElement("button");
  gamesBtn.textContent = "Открыть папку Games";
  gamesBtn.className =
    "rounded px-3 py-1 bg-neutral-200 dark:bg-neutral-800 hover:bg-neutral-300 dark:hover:bg-neutral-700 text-sm";
  gamesBtn.addEventListener("click", () => void invoke("open_games_folder"));
  gamesRow.append(gamesBtn);
  el.appendChild(gamesRow);
}

function renderTabs(): void {
  if (!tabsEl) return;
  tabsEl.innerHTML = "";
  let lang = "system";
  void invoke<string>("get_ui_lang")
    .then((l) => {
      lang = l;
    })
    .catch(() => {});
  for (const tb of TABS) {
    const btn = document.createElement("button");
    btn.textContent = t(lang, tb.key);
    btn.dataset.tab = tb.id;
    btn.className =
      "rounded px-3 py-1.5 bg-neutral-200 dark:bg-neutral-800 hover:bg-neutral-300 dark:hover:bg-neutral-700";
    btn.addEventListener("click", () => selectTab(tb.id));
    tabsEl.appendChild(btn);
  }
}

async function loadConfig(): Promise<void> {
  try {
    const cfg = await invoke<Config>("get_config");
    if (statusEl) {
      statusEl.textContent = cfg.enabled
        ? "Движок включён."
        : "Движок выключен.";
    }
  } catch (e) {
    if (statusEl) statusEl.textContent = `Ошибка: ${String(e)}`;
  }
}

renderTabs();
void loadConfig();
