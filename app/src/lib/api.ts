// Capa única de comunicación con el backend Rust (comandos Tauri).
// Los tipos reflejan los de `crp-core` (serde).

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Category = "system" | "browser" | "dev" | "ai" | "apps" | "registry";
export type Risk = "safe" | "moderate" | "advanced";

export interface Mode {
  id: string;
  name: string;
  description: string;
  icon: string;
  modules: string[];
  custom: boolean;
}

export interface ModeList {
  modes: Mode[];
  errors: string[];
}

export interface ModuleMeta {
  id: string;
  name: string;
  description: string;
  category: Category;
  risk: Risk;
  requires_admin: boolean;
  processes: string[];
}

export interface Finding {
  id: string;
  module_id: string;
  category: Category;
  label: string;
  detail?: string;
  path?: string;
  bytes: number;
  items: number;
  risk: Risk;
  requires_admin: boolean;
  selected: boolean;
  age_days?: number;
  warning?: string;
}

export interface ScanReport {
  findings: Finding[];
  errors: { module_id: string; message: string }[];
  elapsed_ms: number;
}

export interface CleanOutcome {
  finding_id: string;
  freed_bytes: number;
  removed_items: number;
  skipped_items: number;
  errors: string[];
}

export interface CleanReport {
  outcomes: CleanOutcome[];
  dry_run: boolean;
  elapsed_ms: number;
}

export interface SystemInfo {
  is_admin: boolean;
  home: string;
  dev_roots: string[];
  auto_dev_roots: string[];
  config_dir: string;
  version: string;
}

export interface Settings {
  dev_roots: string[];
  exclusions: string[];
  default_mode: string | null;
}

export interface Drive {
  root: string;
  label: string;
  total_bytes: number;
  free_bytes: number;
}

export interface HistoryEntry {
  timestamp: number;
  mode: string | null;
  freed_bytes: number;
  findings: number;
  skipped_items: number;
}

export type ProgressEvent =
  | { type: "scan_started"; total: number }
  | { type: "module_started"; module_id: string }
  | { type: "module_finished"; module_id: string; done: number; total: number; findings: number; bytes: number }
  | { type: "clean_started"; total: number }
  | { type: "finding_cleaned"; finding_id: string; done: number; total: number; freed_bytes: number };

export const api = {
  systemInfo: () => invoke<SystemInfo>("system_info"),
  drives: () => invoke<Drive[]>("drives"),
  history: () => invoke<HistoryEntry[]>("history"),

  modes: () => invoke<ModeList>("list_modes"),
  saveMode: (mode: Mode) => invoke<void>("save_mode", { mode }),
  deleteMode: (id: string) => invoke<void>("delete_mode", { id }),
  modules: () => invoke<ModuleMeta[]>("list_modules"),

  settings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  pickFolder: () => invoke<string | null>("pick_folder"),

  scan: (selectors: string[]) => invoke<ScanReport>("scan", { selectors }),
  /** El backend sólo acepta ids del último escaneo: la UI nunca envía rutas. */
  clean: (findingIds: string[], dryRun: boolean, modeName: string | null) =>
    invoke<CleanReport>("clean", { findingIds, dryRun, modeName }),
  cancel: () => invoke<void>("cancel"),
  reveal: (path: string) => invoke<void>("reveal", { path }),
  restartAsAdmin: () => invoke<void>("restart_as_admin"),

  onProgress: (cb: (e: ProgressEvent) => void): Promise<UnlistenFn> =>
    listen<ProgressEvent>("crp://progress", (e) => cb(e.payload)),
};
