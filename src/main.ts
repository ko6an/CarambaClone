// Точка входа интерфейса: вкладки настроек, i18n (ru/en на лету), macOS-стиль.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Config, AppProfile } from "./lib/types";
import { setSystemLang, t } from "./lib/i18n";

setSystemLang(navigator.languages);

const tabsEl = document.getElementById("tabs");
const versionBadgeEl = document.getElementById("app-version-badge");

interface TabDef {
  id: string;
  key: string;
  icon: string;
}

const TABS: readonly TabDef[] = [
  {
    id: "general",
    key: "tab.general",
    icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/></svg>`,
  },
  {
    id: "fixes",
    key: "tab.fixes",
    icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/></svg>`,
  },
  {
    id: "dictionary",
    key: "tab.dictionary",
    icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253"/></svg>`,
  },
  {
    id: "apps",
    key: "tab.apps",
    icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"/></svg>`,
  },
  {
    id: "rules",
    key: "tab.rules",
    icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"/></svg>`,
  },
  {
    id: "hotkeys",
    key: "tab.hotkeys",
    icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 11c0 3.517-1.009 6.799-2.753 9.571m-3.44-2.04l.054-.09A13.916 13.916 0 008 11a4 4 0 118 0c0 1.017-.07 2.019-.203 3m-2.118 6.844A21.88 21.88 0 0015.171 17m3.839 1.132c.645-2.266.99-4.659.99-7.132A8 8 0 008 4.07M3 15.364c.64-1.319 1-2.8 1-4.364 0-1.457.39-2.823 1.07-4"/></svg>`,
  },
  {
    id: "about",
    key: "tab.about",
    icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>`,
  },
] as const;

let activeTabId: string = "general";
let cachedConfig: Config | null = null;
let onExternalEnabledChange: ((enabled: boolean) => void) | null = null;

function selectTab(id: string): void {
  const panel = document.getElementById("panel");
  if (!panel) return;
  activeTabId = id;

  // Обновляем активный класс в сайдбаре
  if (tabsEl) {
    for (const btn of tabsEl.querySelectorAll<HTMLButtonElement>(".sidebar-tab")) {
      if (btn.dataset.tab === id) {
        btn.classList.add("active");
      } else {
        btn.classList.remove("active");
      }
    }
  }

  panel.innerHTML = "";
  onExternalEnabledChange = null;

  if (id === "general") {
    void renderGeneral(panel);
  } else if (id === "fixes") {
    void renderFixes(panel);
  } else if (id === "dictionary") {
    void renderDictionary(panel);
  } else if (id === "apps") {
    void renderApps(panel);
  } else if (id === "rules") {
    void renderRules(panel);
  } else if (id === "hotkeys") {
    renderHotkeys(panel);
  } else if (id === "about") {
    renderAbout(panel);
  }
}

interface BoolToggle {
  key: keyof Config;
  title: string;
  desc?: string;
}

const FIX_TOGGLES: Array<BoolToggle> = [
  { key: "typos_fix", title: "Опечатки", desc: "Перестановки, пропуск, удвоение букв" },
  { key: "trick_unit_sign", title: "Фокус 2", desc: "привет1 → привет!" },
  { key: "trick_stuck_words", title: "Фокус 3", desc: "Слипшиеся слова: сонатуБаха → сонату Баха" },
  { key: "trick_brackets", title: "Фокус 4", desc: "Скобки без Shift (9/0)" },
  { key: "trick_trailing_signs", title: "Фокус 5", desc: "Знаки препинания в конце слова (б, ю)" },
  { key: "trick_cap_excl", title: "Фокус 6", desc: "Заглавная Ё → «!»" },
  { key: "trick_particles", title: "Фокус 7", desc: "Частицы через дефис: чтото → что-то" },
  { key: "yo_fixer", title: "Ёфикатор", desc: "Однозначные «е» → «ё» по словарю" },
];

const GENERAL_TOGGLES: Array<BoolToggle> = [
  { key: "autoswitch", title: "Автопереключение раскладки", desc: "Автоматически определять и переключать язык ввода" },
  { key: "sound_on_switch", title: "Звук при переключении", desc: "Короткий звуковой сигнал при смене раскладки" },
  { key: "sound_on_fix", title: "Звук при исправлении", desc: "Звуковое оповещение при исправлении опечатки" },
  { key: "smart_backspace_undo", title: "Умная отмена по Backspace", desc: "Возврат исходного слова при нажатии ⌫ в течение 2 с" },
  { key: "ignore_digits_symbols", title: "Игнорировать слова с цифрами", desc: "Пропускать технические идентификаторы и пароли" },
  { key: "fix_on_enter", title: "Исправлять по Enter", desc: "Принудительно применять исправление при переносе строки" },
  { key: "games_folder_watch", title: "Папка Games", desc: "Автоматически отключать работу в играх" },
  { key: "exclude_games_category", title: "Категория игр macOS", desc: "Исключать игровые приложения из App Store" },
];

export interface PermissionsStatus {
  accessibility: boolean;
  input_monitoring: boolean;
  posting: boolean;
  secure_input: boolean;
  tap_active: boolean;
}

// MARK: — Вкладка «Основные»
async function renderGeneral(el: HTMLElement): Promise<void> {
  let cfg: Config;
  try {
    cfg = await invoke<Config>("get_config");
    cachedConfig = cfg;
  } catch (e) {
    el.textContent = `Ошибка: ${String(e)}`;
    return;
  }

  // Заголовок раздела
  const header = document.createElement("div");
  header.className = "mb-4";
  header.innerHTML = `
    <h2 class="text-xl font-bold tracking-tight text-neutral-900 dark:text-neutral-50">Основные настройки</h2>
    <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Управление состоянием программы и базовым поведением</p>
  `;
  el.appendChild(header);

  // 1. Главная карточка состояния (Master Status Card)
  const masterCard = document.createElement("div");
  masterCard.className = `master-card ${cfg.enabled ? "active" : "disabled"}`;

  const statusInfo = document.createElement("div");
  statusInfo.className = "flex items-center gap-3";

  const dot = document.createElement("span");
  dot.className = `status-dot ${cfg.enabled ? "active" : "disabled"}`;

  const textBlock = document.createElement("div");
  const title = document.createElement("div");
  title.className = "text-sm font-semibold text-neutral-900 dark:text-neutral-100";
  title.textContent = cfg.enabled ? "Приложение включено" : "Приложение выключено";

  const subtitle = document.createElement("div");
  subtitle.className = "text-xs text-neutral-500 dark:text-neutral-400";
  subtitle.textContent = cfg.enabled
    ? "Автоматическое переключение раскладки активно"
    : "Работа программы и автозамена приостановлены";

  textBlock.append(title, subtitle);
  statusInfo.append(dot, textBlock);

  const toggleBtn = document.createElement("button");
  const updateBtnStyle = (enabled: boolean): void => {
    toggleBtn.textContent = enabled ? "Выключить" : "Включить";
    toggleBtn.className = enabled
      ? "rounded-md px-3.5 py-1.5 text-xs font-medium bg-neutral-200 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-200 hover:bg-neutral-300 dark:hover:bg-neutral-700 transition"
      : "rounded-md px-3.5 py-1.5 text-xs font-medium bg-blue-600 text-white hover:bg-blue-700 shadow-sm transition";
  };
  updateBtnStyle(cfg.enabled);

  const applyEnabledState = (enabled: boolean): void => {
    cfg.enabled = enabled;
    cachedConfig = cfg;
    masterCard.className = `master-card ${enabled ? "active" : "disabled"}`;
    dot.className = `status-dot ${enabled ? "active" : "disabled"}`;
    title.textContent = enabled ? "Приложение включено" : "Приложение выключено";
    subtitle.textContent = enabled
      ? "Автоматическое переключение раскладки активно"
      : "Работа программы и автозамена приостановлены";
    updateBtnStyle(enabled);
  };

  toggleBtn.addEventListener("click", () => {
    void (async () => {
      const nextState = !cfg.enabled;
      applyEnabledState(nextState);
      try {
        await invoke("set_config", { config: cfg });
      } catch (e) {
        applyEnabledState(!nextState);
        console.error(String(e));
      }
    })();
  });

  onExternalEnabledChange = (enabled) => {
    applyEnabledState(enabled);
  };

  masterCard.append(statusInfo, toggleBtn);
  el.appendChild(masterCard);

  // 2. Секция: Интерфейс
  const uiSectionTitle = document.createElement("div");
  uiSectionTitle.className = "settings-section-title";
  uiSectionTitle.textContent = "Интерфейс";
  el.appendChild(uiSectionTitle);

  const uiCard = document.createElement("div");
  uiCard.className = "settings-card";

  const langRow = document.createElement("div");
  langRow.className = "settings-row";

  const langLabel = document.createElement("div");
  langLabel.className = "settings-label";
  langLabel.innerHTML = `
    <span class="settings-title">Язык интерфейса</span>
    <span class="settings-desc">Язык для меню и окна настроек</span>
  `;

  const langControl = document.createElement("div");
  langControl.className = "settings-control";

  const langSel = document.createElement("select");
  langSel.className = "settings-select";
  for (const [val, lab] of [
    ["system", "Системный язык"],
    ["ru", "Русский"],
    ["en", "English"],
  ] as const) {
    const opt = document.createElement("option");
    opt.value = val;
    opt.textContent = lab;
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

  langControl.appendChild(langSel);
  langRow.append(langLabel, langControl);
  uiCard.appendChild(langRow);
  el.appendChild(uiCard);

  // 3. Секция: Поведение
  const behSectionTitle = document.createElement("div");
  behSectionTitle.className = "settings-section-title";
  behSectionTitle.textContent = "Поведение и звуки";
  el.appendChild(behSectionTitle);

  const behCard = document.createElement("div");
  behCard.className = "settings-card";

  for (const gt of GENERAL_TOGGLES) {
    const row = document.createElement("div");
    row.className = "settings-row";

    const labelDiv = document.createElement("div");
    labelDiv.className = "settings-label";
    const tSpan = document.createElement("span");
    tSpan.className = "settings-title";
    tSpan.textContent = gt.title;
    labelDiv.appendChild(tSpan);

    if (gt.desc) {
      const dSpan = document.createElement("span");
      dSpan.className = "settings-desc";
      dSpan.textContent = gt.desc;
      labelDiv.appendChild(dSpan);
    }

    const controlDiv = document.createElement("div");
    controlDiv.className = "settings-control";

    const switchLabel = document.createElement("label");
    switchLabel.className = "switch";

    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = cfg[gt.key] as boolean;

    const slider = document.createElement("span");
    slider.className = "slider";

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

    switchLabel.append(box, slider);
    controlDiv.appendChild(switchLabel);

    row.append(labelDiv, controlDiv);
    behCard.appendChild(row);
  }
  el.appendChild(behCard);

  // 4. Секция: Система и автозапуск
  const sysSectionTitle = document.createElement("div");
  sysSectionTitle.className = "settings-section-title";
  sysSectionTitle.textContent = "Система и автозапуск";
  el.appendChild(sysSectionTitle);

  const sysCard = document.createElement("div");
  sysCard.className = "settings-card";

  const autoRow = document.createElement("div");
  autoRow.className = "settings-row";

  const autoLabelDiv = document.createElement("div");
  autoLabelDiv.className = "settings-label";
  const autoTitle = document.createElement("span");
  autoTitle.className = "settings-title";
  autoTitle.textContent = "Запуск при входе в систему";
  const autoDesc = document.createElement("span");
  autoDesc.className = "settings-desc";
  autoDesc.textContent = "Автоматический запуск программы через SMAppService";
  autoLabelDiv.append(autoTitle, autoDesc);

  const autoControl = document.createElement("div");
  autoControl.className = "settings-control";

  const autoSwitch = document.createElement("label");
  autoSwitch.className = "switch";
  const autoBox = document.createElement("input");
  autoBox.type = "checkbox";
  autoBox.checked = cfg.autostart;
  const autoSlider = document.createElement("span");
  autoSlider.className = "slider";

  autoBox.addEventListener("change", () => {
    void (async () => {
      try {
        await invoke("autostart_set", { on: autoBox.checked });
        cfg.autostart = autoBox.checked;
      } catch (e) {
        autoBox.checked = !autoBox.checked;
        console.error(String(e));
      }
    })();
  });

  autoSwitch.append(autoBox, autoSlider);
  autoControl.appendChild(autoSwitch);
  autoRow.append(autoLabelDiv, autoControl);
  sysCard.appendChild(autoRow);
  el.appendChild(sysCard);

  // 5. Секция: Статистика и диагностика
  const statsSectionTitle = document.createElement("div");
  statsSectionTitle.className = "settings-section-title";
  statsSectionTitle.textContent = "Статистика и диагностика";
  el.appendChild(statsSectionTitle);

  const statsCard = document.createElement("div");
  statsCard.className = "settings-card";

  const statsRow = document.createElement("div");
  statsRow.className = "settings-row";

  const statsLabelDiv = document.createElement("div");
  statsLabelDiv.className = "settings-label";
  statsLabelDiv.innerHTML = `
    <span class="settings-title">Исправлено опечаток</span>
    <span class="settings-desc">Количество исправленных слов</span>
  `;

  const statsBadge = document.createElement("div");
  statsBadge.className = "settings-control font-mono text-xs font-semibold text-neutral-700 dark:text-neutral-300 bg-neutral-100 dark:bg-neutral-800 px-2.5 py-1 rounded-md border border-neutral-200 dark:border-neutral-700";
  statsBadge.textContent = "Загрузка…";

  const refreshStats = (): void => {
    void invoke<[number, number]>("stats_cmd")
      .then(([today, total]) => {
        statsBadge.textContent = `Сегодня: ${today} · Всего: ${total}`;
      })
      .catch(() => {});
  };
  refreshStats();
  const statsInterval = setInterval(refreshStats, 5000);

  const observer = new MutationObserver(() => {
    if (!el.isConnected) {
      clearInterval(statsInterval);
      observer.disconnect();
    }
  });
  observer.observe(el.parentElement ?? document.body, { childList: true });

  statsRow.append(statsLabelDiv, statsBadge);
  statsCard.appendChild(statsRow);

  const diagRow = document.createElement("div");
  diagRow.className = "settings-row";
  const diagLabelDiv = document.createElement("div");
  diagLabelDiv.className = "settings-label";
  diagLabelDiv.innerHTML = `
    <span class="settings-title">Диагностика</span>
    <span class="settings-desc">Проверка системных разрешений и лог-файла</span>
  `;

  const diagControls = document.createElement("div");
  diagControls.className = "settings-control";

  const logBtn = document.createElement("button");
  logBtn.textContent = "Журнал";
  logBtn.className = "rounded px-2.5 py-1 text-xs bg-neutral-200 dark:bg-neutral-800 hover:bg-neutral-300 dark:hover:bg-neutral-700 font-medium transition";
  logBtn.addEventListener("click", () => void invoke("open_log_cmd"));

  const permBtn = document.createElement("button");
  permBtn.textContent = "Разрешения";
  permBtn.className = "rounded px-2.5 py-1 text-xs bg-neutral-200 dark:bg-neutral-800 hover:bg-neutral-300 dark:hover:bg-neutral-700 font-medium transition";

  const permStatusRow = document.createElement("div");
  permStatusRow.className = "text-xs text-neutral-500 dark:text-neutral-400 p-2 hidden bg-neutral-50 dark:bg-neutral-800/50 border-t border-neutral-100 dark:border-neutral-800";

  permBtn.addEventListener("click", () => {
    void invoke<PermissionsStatus>("permissions_status")
      .then((st) => {
        permStatusRow.classList.remove("hidden");
        permStatusRow.innerHTML = `
          Accessibility: <b>${st.accessibility ? "выдано" : "не выдано"}</b> · 
          Input Monitoring: <b>${st.input_monitoring ? "выдано" : "не выдано"}</b> · 
          Тап: <b>${st.tap_active ? "активен" : "не создан"}</b> · 
          Secure Input: <b>${st.secure_input ? "да" : "нет"}</b>
        `;
      })
      .catch((e) => {
        permStatusRow.classList.remove("hidden");
        permStatusRow.textContent = `Ошибка: ${String(e)}`;
      });
  });

  diagControls.append(logBtn, permBtn);
  diagRow.append(diagLabelDiv, diagControls);
  statsCard.append(diagRow, permStatusRow);
  el.appendChild(statsCard);
}

// MARK: — Вкладка «Исправления»
async function renderFixes(el: HTMLElement): Promise<void> {
  let cfg: Config;
  try {
    cfg = await invoke<Config>("get_config");
  } catch (e) {
    el.textContent = `Ошибка: ${String(e)}`;
    return;
  }

  const header = document.createElement("div");
  header.className = "mb-4";
  header.innerHTML = `
    <h2 class="text-xl font-bold tracking-tight text-neutral-900 dark:text-neutral-50">Исправления и трюки</h2>
    <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Настройка алгоритмов эвристики и умной автозамены</p>
  `;
  el.appendChild(header);

  // Секция: Исправления
  const fixesTitle = document.createElement("div");
  fixesTitle.className = "settings-section-title";
  fixesTitle.textContent = "Правила замены";
  el.appendChild(fixesTitle);

  const fixesCard = document.createElement("div");
  fixesCard.className = "settings-card";

  for (const t of FIX_TOGGLES) {
    const row = document.createElement("div");
    row.className = "settings-row";

    const labelDiv = document.createElement("div");
    labelDiv.className = "settings-label";
    const tSpan = document.createElement("span");
    tSpan.className = "settings-title";
    tSpan.textContent = t.title;
    labelDiv.appendChild(tSpan);

    if (t.desc) {
      const dSpan = document.createElement("span");
      dSpan.className = "settings-desc";
      dSpan.textContent = t.desc;
      labelDiv.appendChild(dSpan);
    }

    const controlDiv = document.createElement("div");
    controlDiv.className = "settings-control";

    const switchLabel = document.createElement("label");
    switchLabel.className = "switch";

    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = cfg[t.key] as boolean;

    const slider = document.createElement("span");
    slider.className = "slider";

    box.addEventListener("change", () => {
      void (async () => {
        (cfg[t.key] as boolean) = box.checked;
        try {
          await invoke("set_config", { config: cfg });
        } catch {
          box.checked = !box.checked;
        }
      })();
    });

    switchLabel.append(box, slider);
    controlDiv.appendChild(switchLabel);

    row.append(labelDiv, controlDiv);
    fixesCard.appendChild(row);
  }
  el.appendChild(fixesCard);

  // Секция: Маскировка
  const maskTitle = document.createElement("div");
  maskTitle.className = "settings-section-title";
  maskTitle.textContent = "Маскировка текста (⌃⌥M)";
  el.appendChild(maskTitle);

  const maskCard = document.createElement("div");
  maskCard.className = "settings-card";

  // Маскировка: включена
  const maskOnRow = document.createElement("div");
  maskOnRow.className = "settings-row";
  maskOnRow.innerHTML = `
    <div class="settings-label">
      <span class="settings-title">Включена (⌃⌥M)</span>
      <span class="settings-desc">Замена букв на визуальные двойники из другой раскладки</span>
    </div>
  `;
  const maskOnControl = document.createElement("div");
  maskOnControl.className = "settings-control";
  const maskOnSwitch = document.createElement("label");
  maskOnSwitch.className = "switch";
  const maskOnBox = document.createElement("input");
  maskOnBox.type = "checkbox";
  maskOnBox.checked = cfg.masking_on;
  const maskOnSlider = document.createElement("span");
  maskOnSlider.className = "slider";
  maskOnBox.addEventListener("change", () => {
    void (async () => {
      cfg.masking_on = maskOnBox.checked;
      try {
        await invoke("set_masking_cmd", { on: maskOnBox.checked });
      } catch {
        maskOnBox.checked = !maskOnBox.checked;
      }
    })();
  });
  maskOnSwitch.append(maskOnBox, maskOnSlider);
  maskOnControl.appendChild(maskOnSwitch);
  maskOnRow.appendChild(maskOnControl);
  maskCard.appendChild(maskOnRow);

  // Маскировка: запоминать состояние
  const maskRemRow = document.createElement("div");
  maskRemRow.className = "settings-row";
  maskRemRow.innerHTML = `
    <div class="settings-label">
      <span class="settings-title">Запоминать состояние</span>
      <span class="settings-desc">Сохранять включение маскировки между перезапусками</span>
    </div>
  `;
  const maskRemControl = document.createElement("div");
  maskRemControl.className = "settings-control";
  const maskRemSwitch = document.createElement("label");
  maskRemSwitch.className = "switch";
  const maskRemBox = document.createElement("input");
  maskRemBox.type = "checkbox";
  maskRemBox.checked = cfg.masking_remember;
  const maskRemSlider = document.createElement("span");
  maskRemSlider.className = "slider";
  maskRemBox.addEventListener("change", () => {
    void (async () => {
      cfg.masking_remember = maskRemBox.checked;
      try {
        await invoke("set_config", { config: cfg });
      } catch {
        maskRemBox.checked = !maskRemBox.checked;
      }
    })();
  });
  maskRemSwitch.append(maskRemBox, maskRemSlider);
  maskRemControl.appendChild(maskRemSwitch);
  maskRemRow.appendChild(maskRemControl);
  maskCard.appendChild(maskRemRow);

  // Интенсивность
  const intensityRow = document.createElement("div");
  intensityRow.className = "settings-row";
  intensityRow.innerHTML = `
    <div class="settings-label">
      <span class="settings-title">Интенсивность</span>
      <span class="settings-desc">Процент заменяемых символов-двойников</span>
    </div>
  `;
  const intensityControl = document.createElement("div");
  intensityControl.className = "settings-control";
  const intensityInput = document.createElement("input");
  intensityInput.type = "range";
  intensityInput.min = "10";
  intensityInput.max = "100";
  intensityInput.step = "5";
  intensityInput.value = String(cfg.masking_intensity);
  intensityInput.className = "w-28 accent-blue-600";
  const intensityVal = document.createElement("span");
  intensityVal.className = "text-xs font-mono w-10 text-right";
  intensityVal.textContent = `${cfg.masking_intensity}%`;

  intensityInput.addEventListener("input", () => {
    intensityVal.textContent = `${intensityInput.value}%`;
  });
  intensityInput.addEventListener("change", () => {
    void (async () => {
      const v = Number(intensityInput.value);
      cfg.masking_intensity = v;
      try {
        await invoke("set_config", { config: cfg });
      } catch {
        intensityInput.value = String(cfg.masking_intensity);
        intensityVal.textContent = `${cfg.masking_intensity}%`;
      }
    })();
  });
  intensityControl.append(intensityInput, intensityVal);
  intensityRow.appendChild(intensityControl);
  maskCard.appendChild(intensityRow);

  // Направление
  const dirRow = document.createElement("div");
  dirRow.className = "settings-row";
  dirRow.innerHTML = `
    <div class="settings-label">
      <span class="settings-title">Направление</span>
      <span class="settings-desc">Правило подстановки символов</span>
    </div>
  `;
  const dirControl = document.createElement("div");
  dirControl.className = "settings-control";
  const dirSel = document.createElement("select");
  dirSel.className = "settings-select";
  for (const [val, lab] of [
    ["ru_to_lat", "Русские буквы → латинские двойники"],
    ["lat_to_ru", "Латинские буквы → русские двойники"],
  ] as const) {
    const opt = document.createElement("option");
    opt.value = val;
    opt.textContent = lab;
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
  dirControl.appendChild(dirSel);
  dirRow.appendChild(dirControl);
  maskCard.appendChild(dirRow);

  el.appendChild(maskCard);
}

// MARK: — Вкладка «Словарь»
async function renderDictionary(el: HTMLElement): Promise<void> {
  let cfg: Config;
  try {
    cfg = await invoke<Config>("get_config");
  } catch (e) {
    el.textContent = `Ошибка: ${String(e)}`;
    return;
  }

  const header = document.createElement("div");
  header.className = "mb-4";
  header.innerHTML = `
    <h2 class="text-xl font-bold tracking-tight text-neutral-900 dark:text-neutral-50">Словарь исключений</h2>
    <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Слова, которые программа никогда не переключает и не исправляет</p>
  `;
  el.appendChild(header);

  const card = document.createElement("div");
  card.className = "settings-card p-4";

  const search = document.createElement("input");
  search.placeholder = "Поиск слова…";
  search.className = "w-full rounded-md border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-800 px-3 py-1.5 text-sm outline-none focus:border-blue-500";

  const list = document.createElement("div");
  list.className = "mt-3 flex flex-col gap-1 max-h-64 overflow-y-auto divide-y divide-neutral-100 dark:divide-neutral-800";

  const renderList = (filter: string): void => {
    list.innerHTML = "";
    const f = filter.trim().toLowerCase();
    const filtered = cfg.exceptions.filter((w) => f === "" || w.toLowerCase().includes(f));
    if (filtered.length === 0) {
      const empty = document.createElement("div");
      empty.className = "text-center py-6 text-xs text-neutral-400";
      empty.textContent = f === "" ? "Список исключений пуст" : "Ничего не найдено";
      list.appendChild(empty);
      return;
    }
    filtered.forEach((word) => {
      const row = document.createElement("div");
      row.className = "flex items-center justify-between py-1.5 px-1 text-sm font-mono";
      const span = document.createElement("span");
      span.textContent = word;
      const del = document.createElement("button");
      del.textContent = "✕";
      del.className = "text-neutral-400 hover:text-red-500 px-2 py-0.5 text-xs transition";
      del.title = "Удалить";
      del.addEventListener("click", () => {
        cfg.exceptions = cfg.exceptions.filter((x) => x !== word);
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
  addRow.className = "flex items-center gap-2 mt-4 pt-3 border-t border-neutral-200 dark:border-neutral-800";
  const addInput = document.createElement("input");
  addInput.placeholder = "Новое слово-исключение…";
  addInput.className = "flex-1 rounded-md border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-800 px-3 py-1.5 text-sm outline-none focus:border-blue-500";
  const addBtn = document.createElement("button");
  addBtn.textContent = "Добавить";
  addBtn.className = "rounded-md px-3.5 py-1.5 bg-blue-600 hover:bg-blue-700 text-white text-xs font-medium shadow-sm transition";

  const addWord = (): void => {
    const w = addInput.value.trim();
    if (w === "" || cfg.exceptions.includes(w)) return;
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

  const resetBtn = document.createElement("button");
  resetBtn.textContent = "Сбросить к списку по умолчанию";
  resetBtn.className = "mt-3 text-xs text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300 underline";
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

  card.append(search, list, addRow, resetBtn);
  el.appendChild(card);
}

// MARK: — Вкладка «Приложения»
async function renderApps(el: HTMLElement): Promise<void> {
  let cfg: Config;
  try {
    cfg = await invoke<Config>("get_config");
  } catch (e) {
    el.textContent = `Ошибка: ${String(e)}`;
    return;
  }

  const header = document.createElement("div");
  header.className = "mb-4";
  header.innerHTML = `
    <h2 class="text-xl font-bold tracking-tight text-neutral-900 dark:text-neutral-50">Профили приложений</h2>
    <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Особое поведение переключения для отдельных программ</p>
  `;
  el.appendChild(header);

  const card = document.createElement("div");
  card.className = "settings-card p-4";

  const list = document.createElement("div");
  list.className = "flex flex-col gap-2 max-h-72 overflow-y-auto";

  const renderList = (): void => {
    list.innerHTML = "";
    if (cfg.app_profiles.length === 0) {
      const empty = document.createElement("div");
      empty.className = "text-center py-6 text-xs text-neutral-400";
      empty.textContent = "Особые профили не заданы (работает во всех программах)";
      list.appendChild(empty);
      return;
    }
    for (const p of cfg.app_profiles) {
      const row = document.createElement("div");
      row.className = "flex items-center justify-between gap-3 p-2 rounded-md bg-neutral-50 dark:bg-neutral-800/60 border border-neutral-100 dark:border-neutral-700/60 text-sm";
      const name = document.createElement("span");
      name.className = "flex-1 font-mono text-xs truncate";
      name.textContent = p.bundle_id;

      const sel = document.createElement("select");
      sel.className = "settings-select text-xs";
      for (const [v, lab] of [
        ["off", "Выключено"],
        ["manual_only", "Только вручную"],
        ["auto", "Авто"],
      ] as const) {
        const opt = document.createElement("option");
        opt.value = v;
        opt.textContent = lab;
        sel.appendChild(opt);
      }
      sel.value = p.mode;
      sel.addEventListener("change", () => {
        p.mode = sel.value as AppProfile["mode"];
        void invoke("app_profiles_set_cmd", { profiles: cfg.app_profiles });
      });

      const del = document.createElement("button");
      del.textContent = "✕";
      del.className = "text-neutral-400 hover:text-red-500 px-1.5 py-0.5 text-xs transition";
      del.addEventListener("click", () => {
        cfg.app_profiles = cfg.app_profiles.filter((x) => x.bundle_id !== p.bundle_id);
        void invoke("app_profiles_set_cmd", { profiles: cfg.app_profiles });
        renderList();
      });

      row.append(name, sel, del);
      list.appendChild(row);
    }
  };
  renderList();

  const addRow = document.createElement("div");
  addRow.className = "flex items-center gap-2 mt-4 pt-3 border-t border-neutral-200 dark:border-neutral-800";
  const addInput = document.createElement("input");
  addInput.placeholder = "Bundle ID (например com.sublimetext.4)…";
  addInput.className = "flex-1 rounded-md border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-800 px-3 py-1.5 text-sm outline-none focus:border-blue-500 font-mono text-xs";
  const addBtn = document.createElement("button");
  addBtn.textContent = "Добавить";
  addBtn.className = "rounded-md px-3.5 py-1.5 bg-blue-600 hover:bg-blue-700 text-white text-xs font-medium shadow-sm transition";

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

  const gamesBtn = document.createElement("button");
  gamesBtn.textContent = "Открыть папку Games для исключений";
  gamesBtn.className = "mt-3 text-xs text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300 underline";
  gamesBtn.addEventListener("click", () => void invoke("open_games_folder"));

  card.append(list, addRow, gamesBtn);
  el.appendChild(card);
}

// MARK: — Вкладка «Правила»
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

  const header = document.createElement("div");
  header.className = "mb-4";
  header.innerHTML = `
    <h2 class="text-xl font-bold tracking-tight text-neutral-900 dark:text-neutral-50">Пакет «Правил»</h2>
    <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Словари, n-gram модели, таблицы опечаток и исключения</p>
  `;
  el.appendChild(header);

  const card = document.createElement("div");
  card.className = "settings-card p-4 space-y-4";

  const statusBox = document.createElement("div");
  statusBox.className = "p-3 rounded-md bg-neutral-50 dark:bg-neutral-800/50 border border-neutral-100 dark:border-neutral-700/60 text-xs space-y-1.5";
  statusBox.innerHTML = `
    <div class="font-medium text-neutral-800 dark:text-neutral-200">
      ${info.installed ? "Установлен обновлённый пакет от:" : "Используется встроенный пакет:"} <span class="font-mono text-blue-600 dark:text-blue-400 font-semibold">${info.version}</span>
    </div>
    <div class="text-neutral-500 dark:text-neutral-400">
      Словарная база: <b>RU ${info.words_ru.toLocaleString()}</b> слов · <b>EN ${info.words_en.toLocaleString()}</b> слов
    </div>
  `;
  card.appendChild(statusBox);

  const actionRow = document.createElement("div");
  actionRow.className = "flex items-center gap-2";

  const checkBtn = document.createElement("button");
  checkBtn.textContent = "Проверить обновление";
  checkBtn.className = "rounded-md px-3 py-1.5 text-xs font-medium bg-blue-600 text-white hover:bg-blue-700 shadow-sm transition";

  const stateSpan = document.createElement("span");
  stateSpan.className = "text-xs text-neutral-500";

  checkBtn.addEventListener("click", () => {
    stateSpan.textContent = "Проверка на сервере…";
    void (async () => {
      try {
        await invoke<RulesInfo>("rules_check_now");
        stateSpan.textContent = "Пакет успешно обновлён и применён.";
        void renderRules(el);
      } catch (e) {
        stateSpan.textContent = `Пакет актуален (${String(e)})`;
      }
    })();
  });

  const rollbackBtn = document.createElement("button");
  rollbackBtn.textContent = "Откатить к встроенному";
  rollbackBtn.className = "rounded-md px-3 py-1.5 text-xs font-medium bg-neutral-200 dark:bg-neutral-800 hover:bg-neutral-300 dark:hover:bg-neutral-700 transition";
  rollbackBtn.addEventListener("click", () => {
    void (async () => {
      try {
        await invoke("rules_rollback");
        void renderRules(el);
      } catch (e) {
        console.error(String(e));
      }
    })();
  });

  actionRow.append(checkBtn, rollbackBtn, stateSpan);
  card.appendChild(actionRow);

  const autoUpdateRow = document.createElement("div");
  autoUpdateRow.className = "flex items-center justify-between pt-3 border-t border-neutral-100 dark:border-neutral-800 text-xs";
  autoUpdateRow.innerHTML = `
    <span class="font-medium text-neutral-700 dark:text-neutral-300">Автоматическая проверка раз в неделю</span>
  `;
  const autoSwitch = document.createElement("label");
  autoSwitch.className = "switch";
  const autoBox = document.createElement("input");
  autoBox.type = "checkbox";
  void invoke<boolean>("get_rules_autoupdate").then((v) => {
    autoBox.checked = v;
  });
  autoBox.addEventListener("change", () => {
    void invoke("set_rules_autoupdate", { on: autoBox.checked });
  });
  const autoSlider = document.createElement("span");
  autoSlider.className = "slider";
  autoSwitch.append(autoBox, autoSlider);
  autoUpdateRow.appendChild(autoSwitch);
  card.appendChild(autoUpdateRow);

  el.appendChild(card);
}

// MARK: — Вкладка «Горячие клавиши»
function renderHotkeys(el: HTMLElement): void {
  const header = document.createElement("div");
  header.className = "mb-4";
  header.innerHTML = `
    <h2 class="text-xl font-bold tracking-tight text-neutral-900 dark:text-neutral-50">Горячие клавиши</h2>
    <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Быстрое управление раскладкой и специальными режимами</p>
  `;
  el.appendChild(header);

  const card = document.createElement("div");
  card.className = "settings-card";

  const SHORTCUTS = [
    { key: "⌃⌥M", title: "Маскировка текста", desc: "Быстрое включение/отключение режима маскировки" },
    { key: "DoubleShift", title: "Отмена конвертации", desc: "Двойное нажатие Shift возвращает исходный ввод" },
    { key: "⌥ (тап)", title: "Конвертация слова", desc: "Одиночное нажатие Option конвертирует текущее слово" },
    { key: "⌥⇧R", title: "Конвертация выделения", desc: "Исправление раскладки для выделенного фрагмента текста" },
    { key: "⌫ (2 с)", title: "Smart Undo", desc: "Умный откат последнего автоисправления по Backspace" },
  ];

  for (const s of SHORTCUTS) {
    const row = document.createElement("div");
    row.className = "settings-row";
    row.innerHTML = `
      <div class="settings-label">
        <span class="settings-title">${s.title}</span>
        <span class="settings-desc">${s.desc}</span>
      </div>
      <div class="settings-control">
        <kbd class="font-bold text-xs">${s.key}</kbd>
      </div>
    `;
    card.appendChild(row);
  }
  el.appendChild(card);
}

// MARK: — Вкладка «О программе»
function renderAbout(el: HTMLElement): void {
  const header = document.createElement("div");
  header.className = "mb-4";
  header.innerHTML = `
    <h2 class="text-xl font-bold tracking-tight text-neutral-900 dark:text-neutral-50">О программе</h2>
    <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Сведения о версии, лицензиях и разработке</p>
  `;
  el.appendChild(header);

  const card = document.createElement("div");
  card.className = "settings-card p-6 text-center flex flex-col items-center";

  const iconDiv = document.createElement("div");
  iconDiv.className = "w-16 h-16 rounded-2xl bg-gradient-to-br from-blue-500 to-indigo-600 flex items-center justify-center text-white text-2xl font-bold shadow-lg mb-4";
  iconDiv.textContent = "CC";

  const title = document.createElement("h3");
  title.className = "text-lg font-bold text-neutral-900 dark:text-neutral-50";
  title.textContent = "CarambaClone";

  const versionEl = document.createElement("p");
  versionEl.className = "text-xs font-mono text-neutral-500 dark:text-neutral-400 mt-1 mb-4";
  versionEl.textContent = "Загрузка версии…";

  void invoke<string>("get_app_version").then((ver) => {
    versionEl.textContent = `Версия ${ver} (Синхронизировано с GitHub Releases)`;
    if (versionBadgeEl) {
      versionBadgeEl.textContent = `v${ver}`;
    }
  }).catch(() => {
    versionEl.textContent = "Версия 1.0.7";
  });

  const desc = document.createElement("p");
  desc.className = "text-xs text-neutral-600 dark:text-neutral-300 max-w-md leading-relaxed mb-4";
  desc.textContent = "Умный, бесшумный автоматический переключатель раскладки клавиатуры нового поколения для macOS. Распознаёт язык на лету, исправляет случайные опечатки и уважает приватность.";

  const notices = document.createElement("div");
  notices.className = "text-[11px] text-neutral-400 dark:text-neutral-500 max-w-sm pt-4 border-t border-neutral-100 dark:border-neutral-800";
  notices.innerHTML = `Лицензии словарей: см. <code>THIRD_PARTY_NOTICES.md</code>.<br>Иконка Anonymous — Public Domain.`;

  card.append(iconDiv, title, versionEl, desc, notices);
  el.appendChild(card);
}

// Построение списка вкладок в левом сайдбаре
function renderTabs(): void {
  if (!tabsEl) return;
  tabsEl.innerHTML = "";
  let lang = "system";
  void invoke<string>("get_ui_lang")
    .then((l) => {
      lang = l;
      updateTabLabels(lang);
    })
    .catch(() => {
      updateTabLabels("system");
    });

  for (const tb of TABS) {
    const btn = document.createElement("button");
    btn.dataset.tab = tb.id;
    btn.className = `sidebar-tab ${tb.id === activeTabId ? "active" : ""}`;
    btn.innerHTML = `${tb.icon} <span class="tab-label">${t(lang, tb.key)}</span>`;
    btn.addEventListener("click", () => selectTab(tb.id));
    tabsEl.appendChild(btn);
  }
}

function updateTabLabels(lang: string): void {
  if (!tabsEl) return;
  for (const btn of tabsEl.querySelectorAll<HTMLButtonElement>(".sidebar-tab")) {
    const id = btn.dataset.tab;
    const tb = TABS.find((t) => t.id === id);
    if (tb) {
      const labelSpan = btn.querySelector<HTMLSpanElement>(".tab-label");
      if (labelSpan) {
        labelSpan.textContent = t(lang, tb.key);
      }
    }
  }
}

// Загрузка версии в сайдбар
function loadVersion(): void {
  void invoke<string>("get_app_version")
    .then((ver) => {
      if (versionBadgeEl) {
        versionBadgeEl.textContent = `v${ver}`;
      }
    })
    .catch(() => {});
}

// Инициализация при старте
function init(): void {
  renderTabs();
  loadVersion();
  selectTab("general");

  // Подписка на изменение статуса из трея
  void listen<boolean>("enabled-changed", (event) => {
    if (cachedConfig) {
      cachedConfig.enabled = event.payload;
    }
    if (onExternalEnabledChange) {
      onExternalEnabledChange(event.payload);
    }
  });
}

init();
