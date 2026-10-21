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
  { id: "about", title: "О программе" },
] as const;

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

function selectTab(id: string): void {
  if (!statusEl) return;
  // В M0 вкладки ещё пустые — выводим только идентификатор.
  statusEl.textContent = `Вкладка «${id}» появится в M5.`;
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
