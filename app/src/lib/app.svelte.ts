// Estado global de la app (runas de Svelte 5). Vive fuera de los componentes
// para que el resultado del análisis no se pierda al cambiar de vista.

import { SvelteSet } from "svelte/reactivity";
import {
  api,
  type CleanReport,
  type Finding,
  type Mode,
  type ModuleMeta,
  type ProgressEvent,
  type ScanReport,
  type Settings,
  type SystemInfo,
} from "./api";

export type View = "home" | "clean" | "settings";
export type Phase = "idle" | "scanning" | "results" | "cleaning" | "done";

class AppStore {
  // ── Datos ────────────────────────────────────────────────────────────
  info = $state<SystemInfo | null>(null);
  modes = $state<Mode[]>([]);
  modeErrors = $state<string[]>([]);
  modules = $state<ModuleMeta[]>([]);
  settings = $state<Settings>({ dev_roots: [], exclusions: [], default_mode: null });

  // ── Navegación ───────────────────────────────────────────────────────
  view = $state<View>("home");
  mode = $state<Mode | null>(null);
  error = $state<string | null>(null);

  // ── Sesión de limpieza ───────────────────────────────────────────────
  phase = $state<Phase>("idle");
  report = $state<ScanReport | null>(null);
  cleanReport = $state<CleanReport | null>(null);
  progress = $state({ done: 0, total: 0, current: "" });
  readonly selected = new SvelteSet<string>();

  busy = $derived(this.phase === "scanning" || this.phase === "cleaning");

  async load() {
    try {
      const [info, modeList, modules, settings] = await Promise.all([
        api.systemInfo(),
        api.modes(),
        api.modules(),
        api.settings(),
      ]);
      this.info = info;
      this.modules = modules;
      this.settings = settings;
      this.setModes(modeList.modes, modeList.errors);
      this.mode =
        this.modes.find((m) => m.id === settings.default_mode) ??
        this.modes.find((m) => m.id === "rapido") ??
        this.modes[0] ??
        null;
    } catch (e) {
      this.error = String(e);
    }
  }

  private setModes(modes: Mode[], errors: string[]) {
    this.modes = modes;
    this.modeErrors = errors;
  }

  async reloadModes() {
    const list = await api.modes();
    this.setModes(list.modes, list.errors);
    if (this.mode && !this.modes.some((m) => m.id === this.mode!.id)) this.mode = this.modes[0] ?? null;
    else if (this.mode) this.mode = this.modes.find((m) => m.id === this.mode!.id) ?? null;
  }

  async saveSettings(s: Settings) {
    await api.saveSettings(s);
    this.settings = await api.settings();
    this.info = await api.systemInfo();
  }

  onProgress(e: ProgressEvent) {
    if (e.type === "scan_started" || e.type === "clean_started") this.progress = { done: 0, total: e.total, current: "" };
    else if (e.type === "module_finished") this.progress = { done: e.done, total: e.total, current: e.module_id };
    else if (e.type === "finding_cleaned") this.progress = { done: e.done, total: e.total, current: e.finding_id };
  }

  /** Abre la vista de limpieza con un modo (y opcionalmente analiza ya). */
  openMode(mode: Mode, scanNow = false) {
    if (this.busy) return;
    if (this.mode?.id !== mode.id) this.reset();
    this.mode = mode;
    this.view = "clean";
    if (scanNow) void this.scan();
  }

  reset() {
    this.phase = "idle";
    this.report = null;
    this.cleanReport = null;
    this.selected.clear();
  }

  selectable(f: Finding) {
    return !(f.requires_admin && !this.info?.is_admin);
  }

  async scan() {
    if (!this.mode || this.busy) return;
    this.error = null;
    this.cleanReport = null;
    this.phase = "scanning";
    try {
      this.report = await api.scan(this.mode.modules);
      this.selected.clear();
      for (const f of this.report.findings) if (f.selected && this.selectable(f)) this.selected.add(f.id);
      this.phase = "results";
    } catch (e) {
      this.error = String(e);
      this.phase = this.report ? "results" : "idle";
    }
  }

  async clean() {
    if (this.busy) return;
    this.phase = "cleaning";
    try {
      this.cleanReport = await api.clean([...this.selected], false, this.mode?.name ?? null);
      this.phase = "done";
    } catch (e) {
      this.error = String(e);
      this.phase = "results";
    }
  }
}

export const app = new AppStore();
