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
  /** Idioma efectivo del backend. */
  language: string;
}

export interface Settings {
  dev_roots: string[];
  exclusions: string[];
  default_mode: string | null;
  /** `null` = idioma del sistema. */
  language: string | null;
  /** `null` = tema del sistema. */
  theme: "light" | "dark" | null;
  always_admin: boolean;
  /** Minimizar a la bandeja del sistema. */
  tray: boolean;
  autostart: boolean;
  background: { enabled: boolean; every_hours: number; threshold_mb: number };
  /** Permitir elegir archivos de Windows protegidos (con aviso en cada uno). */
  allow_system: boolean;
  onboarded: boolean;
  /** Borrados que se guardan en el registro (0 = no guardar). */
  deletion_log: number;
  /** Ofrecer cookies e historial de los navegadores. */
  privacy: boolean;
  /** Buscar una versión nueva al abrir la app. */
  check_updates: boolean;
}

export interface UpdateInfo {
  version: string;
  notes: string | null;
  /** `false` en la versión portable: se ofrece descargar. */
  installable: boolean;
}

export interface UpdateProgress {
  done: number;
  total: number | null;
}

export interface ExclusionSize {
  path: string;
  exists: boolean;
  bytes: number;
  items: number;
}

export interface DeletionRecord {
  timestamp: number;
  source: "clean" | "space" | "dupes";
  path: string;
  bytes: number;
  permanent: boolean;
}

export interface BackgroundStatus {
  timestamp: number;
  bytes: number;
  findings: number;
}

export interface DeleteProgress {
  done: number;
  total: number;
  current: string | null;
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

// ── Espacio (analizador) ──────────────────────────────────────────────────

export type NodeKind = "dir" | "file" | "link";

export interface DiskSummary {
  root: number;
  path: string;
  bytes: number;
  files: number;
  dirs: number;
  unreadable: number;
  cloud_files: number;
  elapsed_ms: number;
  extensions: { ext: string; bytes: number; files: number }[];
}

/** Árbol podado para el mapa de rectángulos (columnas; pieza 0 = raíz). */
export interface Treemap {
  parent: number[];
  id: number[];
  size: number[];
  name: string[];
  kind: ("dir" | "file" | "rest")[];
  /** Índice en `exts` o 65535. */
  ext: number[];
  exts: string[];
}

export interface ViewNode {
  id: number | null;
  name: string;
  size: number;
  items: number;
  kind: NodeKind | "rest";
  children: ViewNode[];
}

export interface DiskEntry {
  id: number;
  name: string;
  size: number;
  items: number;
  kind: NodeKind;
  modified: number;
  /** Por qué no se puede eliminar (archivo de Windows, carpeta base…). */
  protected: string | null;
  /** Está en las exclusiones del usuario. */
  excluded: boolean;
}

export interface DiskFile {
  id: number;
  name: string;
  path: string;
  size: number;
  modified: number;
  protected: string | null;
  excluded: boolean;
}

export interface Crumb {
  id: number;
  name: string;
}

export interface DiskProgress {
  files: number;
  dirs: number;
  bytes: number;
  current: string;
}

export interface DeleteResult {
  freed_bytes: number;
  removed: number;
  removed_paths: string[];
  errors: string[];
}

// ── Duplicados ────────────────────────────────────────────────────────────

export interface DupeFile {
  id: number;
  path: string;
  modified: number;
}

export interface DupeGroup {
  size: number;
  files: DupeFile[];
}

export interface DupeReport {
  groups: DupeGroup[];
  files_scanned: number;
  wasted_bytes: number;
  unreadable: number;
  cloud_skipped: number;
  elapsed_ms: number;
}

export type DupeProgress =
  | { phase: "listing"; files: number }
  | { phase: "hashing"; done: number; total: number };

export type Task = "clean" | "disk" | "dupes" | "delete";

export type Link = "repo" | "issues" | "new_issue" | "releases" | "license" | "contributors" | "changelog" | `user:${string}`;

export const api = {
  systemInfo: () => invoke<SystemInfo>("system_info"),
  drives: () => invoke<Drive[]>("drives"),
  history: () => invoke<HistoryEntry[]>("history"),
  deletions: (limit?: number) => invoke<DeletionRecord[]>("deletions", { limit }),
  deletionsCount: () => invoke<number>("deletions_count"),
  exclusionSizes: () => invoke<ExclusionSize[]>("exclusion_sizes"),
  /** Deja sólo los `keep` registros más recientes (0 = vaciar). Devuelve cuántos quitó. */
  trimDeletions: (keep: number) => invoke<number>("trim_deletions", { keep }),
  backgroundStatus: () => invoke<BackgroundStatus | null>("background_status"),
  recycleBin: () => invoke<{ bytes: number; items: number } | null>("recycle_bin"),
  openLink: (target: Link) => invoke<void>("open_link", { target }),
  toTray: () => invoke<void>("to_tray"),
  findingPath: (id: string) => invoke<string | null>("finding_path", { id }),
  /** Ruta que se está eliminando (o `null` al terminar). */
  onNow: (cb: (path: string | null) => void): Promise<UnlistenFn> =>
    listen<{ path: string | null }>("crp://now", (e) => cb(e.payload.path)),
  onDelete: (cb: (p: DeleteProgress) => void): Promise<UnlistenFn> =>
    listen<DeleteProgress>("crp://delete", (e) => cb(e.payload)),

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
  cancel: (task: Task = "clean") => invoke<void>("cancel", { task }),
  reveal: (path: string) => invoke<void>("reveal", { path }),
  restartAsAdmin: () => invoke<void>("restart_as_admin"),
  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  installUpdate: () => invoke<void>("install_update"),
  onUpdateProgress: (cb: (p: UpdateProgress) => void): Promise<UnlistenFn> =>
    listen<UpdateProgress>("crp://update", (e) => cb(e.payload)),

  diskScan: (path: string) => invoke<DiskSummary>("disk_scan", { path }),
  diskSummary: () => invoke<DiskSummary | null>("disk_summary"),
  diskView: (id: number, depth: number) => invoke<ViewNode | null>("disk_view", { id, depth }),
  diskTreemap: (id: number) => invoke<Treemap | null>("disk_treemap", { id }),
  diskChildren: (id: number) => invoke<DiskEntry[]>("disk_children", { id }),
  diskCrumbs: (id: number) => invoke<Crumb[]>("disk_crumbs", { id }),
  /** Los archivos más grandes bajo `id`; con `ext`, sólo de ese tipo ("" = sin extensión). */
  diskLargest: (id: number, ext?: string) => invoke<DiskFile[]>("disk_largest", { id, ext: ext ?? null }),
  diskReveal: (id: number) => invoke<void>("disk_reveal", { id }),
  diskPath: (id: number) => invoke<string | null>("disk_path", { id }),
  diskEntry: (id: number) => invoke<DiskEntry | null>("disk_entry", { id }),
  diskDelete: (ids: number[], permanent: boolean) => invoke<DeleteResult>("disk_delete", { ids, permanent }),
  onDiskProgress: (cb: (p: DiskProgress) => void): Promise<UnlistenFn> =>
    listen<DiskProgress>("crp://disk", (e) => cb(e.payload)),

  dupesScan: (roots: string[], minSize: number, includeSystem: boolean) =>
    invoke<DupeReport>("dupes_scan", { roots, minSize, includeSystem }),
  dupesReport: () => invoke<DupeReport | null>("dupes_report"),
  dupesReveal: (id: number) => invoke<void>("dupes_reveal", { id }),
  icons: (keys: string[]) => invoke<Record<string, string>>("icons", { keys }),
  dupesPath: (id: number) => invoke<string | null>("dupes_path", { id }),
  dupesDelete: (ids: number[], permanent: boolean) => invoke<DeleteResult>("dupes_delete", { ids, permanent }),
  onDupesProgress: (cb: (p: DupeProgress) => void): Promise<UnlistenFn> =>
    listen<DupeProgress>("crp://dupes", (e) => cb(e.payload)),

  onProgress: (cb: (e: ProgressEvent) => void): Promise<UnlistenFn> =>
    listen<ProgressEvent>("crp://progress", (e) => cb(e.payload)),
};
