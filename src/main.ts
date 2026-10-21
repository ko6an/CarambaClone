// Точка входа интерфейса. В M0 — каркас: статус и заглушки вкладок.
import { invoke } from "@tauri-apps/api/core";
import type { Config } from "./lib/types";

const statusEl = document.getElementById("status");
const tabsEl = document.getElementById("tabs");

const TABS = [
  { id: "general", title: "Основные" },
  { id: "fixes", title: "Исправления" },
  { id: "dictionary", title: "Словарь" },
  { id: "apps", title: "Приложения" },
  { id: "rules", title: "Правила" },
  { id: "hotkeys", title: "Горячие клавиши" },
  { id: "permissions", title: "Разрешения" },
  { id: "about", title: "О программе" },
] as const;

interface PermissionsStatus {
  accessibility: boolean;
  input_monitoring: boolean;
  posting: boolean;
  secure_input: boolean;
  tap_active: boolean;
}

function renderPermissions(el: HTMLElement, st: PermissionsStatus): void {
  const rows: Array<[string, boolean, string]> = [
    ["Accessibility (управление вводом)", st.accessibility, "accessibility"],
    ["Input Monitoring (наблюдение событий)", st.input_monitoring, "input_monitoring"],
  ];
  el.innerHTML = `<h2 class="text-lg font-semibold mt-4">Разрешения macOS</h2>
    <p class="text-sm mt-1">Для работы нужны оба разрешения; после выдачи
    перезапустите приложение.</p>`;
  for (const [title, ok, pane] of rows) {
    const row = document.createElement("div");
    row.className = "flex items-center justify-between mt-3 text-sm";
    const label = document.createElement("span");
    label.textContent = `${ok ? "выдано" : "не выдано"} — ${title}`;
    label.className = ok
      ? "text-green-700 dark:text-green-400"
      : "text-red-700 dark:text-red-400";
    const btn = document.createElement("button");
    btn.textContent = "Открыть настройки…";
    btn.className =
      "rounded px-3 py-1 bg-neutral-200 dark:bg-neutral-800 hover:bg-neutral-300 dark:hover:bg-neutral-700";
    btn.addEventListener("click", () => {
      void invoke("open_permissions_pane", { pane });
    });
    row.append(label, btn);
    el.appendChild(row);
  }
  const extra = document.createElement("p");
  extra.className = "mt-3 text-xs text-neutral-500";
  extra.textContent = `Secure Input активен: ${st.secure_input ? "да" : "нет"}; тап: ${
    st.tap_active ? "работает" : "не создан"
  }`;
  el.appendChild(extra);
}

function selectTab(id: string): void {
  const panel = document.getElementById("panel");
  if (!panel) return;
  panel.innerHTML = "";
  if (id === "general") {
    void loadConfig();
    return;
  }
  if (id === "permissions") {
    void invoke<PermissionsStatus>("permissions_status")
      .then((st) => renderPermissions(panel, st))
      .catch((e) => {
        panel.textContent = `Ошибка: ${String(e)}`;
      });
    return;
  }
  if (id === "fixes") {
    void renderFixes(panel);
    return;
  }
  const p = document.createElement("p");
  p.className = "mt-4 text-sm";
  p.textContent = `Вкладка «${id}» появится в M5.`;
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
  appendMaskControl(el, "Включена", "masking_on", cfg, (c) => Boolean(c.masking_on));
  appendMaskControl(
    el,
    "Запоминать состояние между запусками",
    "masking_remember",
    cfg,
    (c) => c.masking_remember,
  );
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
  get: (c: Config) => boolean,
): void {
  const row = document.createElement("label");
  row.className = "flex items-center gap-2 mt-2 text-sm";
  const box = document.createElement("input");
  box.type = "checkbox";
  box.checked = get(cfg);
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

function renderTabs(): void {
  if (!tabsEl) return;
  tabsEl.innerHTML = "";
  for (const t of TABS) {
    const btn = document.createElement("button");
    btn.textContent = t.title;
    btn.dataset.tab = t.id;
    btn.className =
      "rounded px-3 py-1.5 bg-neutral-200 dark:bg-neutral-800 hover:bg-neutral-300 dark:hover:bg-neutral-700";
    btn.addEventListener("click", () => selectTab(t.id));
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
