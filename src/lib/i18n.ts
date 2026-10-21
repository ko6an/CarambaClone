import ru from "../../resources/i18n/ru.json";
import en from "../../resources/i18n/en.json";

export type Lang = "system" | "ru" | "en";

type Dict = Record<string, string>;

const DICTS: Record<"ru"|"en", Dict> = { ru, en } as unknown as Record<"ru"|"en", Dict>;

let systemLang: "ru" | "en" = "ru";

export function setSystemLang(navigatorLangs: readonly string[]): void {
  systemLang = navigatorLangs.some((l) => l.toLowerCase().startsWith("ru"))
    ? "ru"
    : "en";
}

export function currentLang(langSetting: string): "ru" | "en" {
  if (langSetting === "ru" || langSetting === "en") return langSetting;
  return systemLang;
}

export function t(langSetting: string, key: string): string {
  const lang = currentLang(langSetting);
  return DICTS[lang][key] ?? DICTS.ru[key] ?? key;
}
