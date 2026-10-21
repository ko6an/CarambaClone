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
  const p = document.createElement("p");
  p.className = "mt-4 text-sm";
  p.textContent = `Вкладка «${id}» появится в M5.`;
  panel.appendChild(p);
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
