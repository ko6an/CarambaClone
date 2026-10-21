// Типы, зеркалящие caramba_core::config::Config (serde сериализует в camelCase
// для команд Tauri — в M0 оставлено snake_case одинаково с Rust).

export interface Config {
  schema_version: number;
  enabled: boolean;
  ui_lang: string;
  autoswitch: boolean;
  aggressiveness: "low" | "medium" | "high";
  sound_on_switch: boolean;
  fix_on_enter: boolean;
  smart_backspace_undo: boolean;
  yo_fixer: boolean;
  typos_fix: boolean;
  trick_unit_sign: boolean;
  trick_stuck_words: boolean;
  trick_brackets: boolean;
  trick_trailing_signs: boolean;
  trick_cap_excl: boolean;
  trick_particles: boolean;
  masking_on: boolean;
  masking_remember: boolean;
  masking_intensity: number;
  masking_direction: "ru_to_lat" | "lat_to_ru";
  ignore_digits_symbols: boolean;
  autostart: boolean;
  rules_autoupdate: boolean;
  dry_run: boolean;
}
