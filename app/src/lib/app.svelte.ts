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
import { i18n, resolveLang } from "./i18n.svelte";

export type View = "home" | "clean" | "space" | "dupes" | "startup" | "exclusions" | "settings" | "about";
export type Phase = "idle" | "scanning" | "results" | "cleaning" | "done";

class AppStore {
  // ── Datos ────────────────────────────────────────────────────────────
  info = $state<SystemInfo | null>(null);
  modes = $state<Mode[]>([]);
  modeErrors = $state<string[]>([]);
  modules = $state<ModuleMeta[]>([]);
  settings = $state<Settings>({
    dev_roots: [],
    exclusions: [],
    default_mode: null,
    language: null,
    theme: null,
    always_admin: false,
    tray: true,
    autostart: false,
    background: { enabled: false, every_hours: 6, threshold_mb: 1024 },
    allow_system: false,
    onboarded: true,
    deletion_log: 10000,
    privacy: false,
    check_updates: true,
  });
  /** Datos iniciales cargados (para ocultar la pantalla de carga). */
  loaded = $state(false);
  /** Ya se hizo el análisis rápido del Resumen en esta sesión. */
  estimated = false;

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
      i18n.lang = resolveLang(info.language);
      applyTheme(settings.theme);
      this.setModes(modeList.modes, modeList.errors);
      this.mode =
        this.modes.find((m) => m.id === settings.default_mode) ??
        this.modes.find((m) => m.id === "rapido") ??
        this.modes[0] ??
        null;
    } catch (e) {
      this.error = String(e);
    } finally {
      this.loaded = true;
    }
  }

  /** Exclusión que contiene `path` (o `undefined`). */
  exclusionFor(path: string): string | undefined {
    const p = path.toLowerCase().replace(/\\+$/, "");
    return this.settings.exclusions.find((e) => {
      const x = e.toLowerCase().replace(/\\+$/, "");
      return p === x || p.startsWith(`${x}\\`);
    });
  }

  async addExclusion(path: string) {
    if (this.exclusionFor(path)) return;
    try {
      await this.saveSettings({ ...this.settings, exclusions: [...this.settings.exclusions, path] });
    } catch (e) {
      this.error = String(e);
    }
  }

  async removeExclusion(path: string) {
    try {
      await this.saveSettings({ ...this.settings, exclusions: this.settings.exclusions.filter((e) => e !== path) });
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

  setTheme(theme: Settings["theme"]) {
    applyTheme(theme);
    void this.saveSettings({ ...this.settings, theme }).catch((e) => (this.error = String(e)));
  }

  /** Cambia el idioma (`null` = el del sistema). Los textos del motor se
   *  regeneran, así que se recargan módulos y modos y se descarta el análisis. */
  async setLanguage(language: string | null) {
    if (this.busy) return;
    try {
      await this.saveSettings({ ...this.settings, language });
      i18n.lang = resolveLang(this.info?.language);
      this.modules = await api.modules();
      await this.reloadModes();
      this.reset();
    } catch (e) {
      this.error = String(e);
    }
  }

  onProgress(e: ProgressEvent) {
    if (e.type === "scan_started" || e.type === "clean_started") this.progress = { done: 0, total: e.total, current: "" };
    else if (e.type === "module_finished") this.progress = { done: e.done, total: e.total, current: e.module_id };
    else if (e.type === "finding_cleaned") this.progress = { done: e.done, total: e.total, current: e.finding_id };
  }

  /** Cambia el modo de limpieza (descarta un resultado de otro modo). */
  setMode(mode: Mode) {
    if (this.busy || this.mode?.id === mode.id) return;
    this.reset();
    this.mode = mode;
  }

  /** Abre la vista de limpieza con un modo (y opcionalmente analiza ya). */
  openMode(mode: Mode, scanNow = false) {
    if (this.busy) return;
    this.setMode(mode);
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
    this.report = null;
    this.phase = "scanning";
    try {
      this.report = await api.scan(this.mode.modules);
      this.selected.clear();
      for (const f of this.report.findings) if (f.selected && this.selectable(f)) this.selected.add(f.id);
      this.phase = "results";
    } catch (e) {
      this.error = String(e);
      this.phase = "idle";
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

/** `null` sigue al sistema; `light`/`dark` lo fuerzan (ver app.css). */
export function applyTheme(theme: Settings["theme"]) {
  const root = document.documentElement;
  if (theme) root.dataset.theme = theme;
  else delete root.dataset.theme;
}
