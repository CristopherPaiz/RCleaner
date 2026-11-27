// Estado del analizador de espacio. El árbol vive en Rust; aquí sólo la
// vista actual (nodo, anillos, lista) y la "bandeja" de lo que se va a borrar.

import { SvelteMap } from "svelte/reactivity";
import { icons } from "./icons.svelte";
import {
  api,
  type Crumb,
  type DeleteResult,
  type DiskEntry,
  type DiskFile,
  type DiskProgress,
  type DiskSummary,
  type Treemap,
  type ViewNode,
} from "./api";

export type SpacePhase = "idle" | "scanning" | "results";
export type SpaceTab = "folder" | "largest" | "types";
export type SpaceChart = "radial" | "map";

const CHART_KEY = "rcleaner.space.chart";

function savedChart(): SpaceChart {
  try {
    return localStorage.getItem(CHART_KEY) === "map" ? "map" : "radial";
  } catch {
    return "radial";
  }
}

const RINGS = 4;

export interface Pick {
  id: number;
  name: string;
  size: number;
  /** Archivo de Windows protegido (elegido bajo responsabilidad del usuario). */
  system?: boolean;
}

class SpaceStore {
  phase = $state<SpacePhase>("idle");
  target = $state("");
  summary = $state<DiskSummary | null>(null);
  progress = $state<DiskProgress>({ files: 0, dirs: 0, bytes: 0, current: "" });

  node = $state(0);
  view = $state<ViewNode | null>(null);
  entries = $state<DiskEntry[]>([]);
  crumbs = $state<Crumb[]>([]);
  largest = $state<DiskFile[]>([]);
  tab = $state<SpaceTab>("folder");
  /** Pestaña Tipos: extensión elegida y sus archivos (de toda la unidad). */
  extFilter = $state<string | null>(null);
  extFiles = $state<DiskFile[]>([]);
  /** Mapa radial o de rectángulos (estilo WinDirStat). */
  chart = $state<SpaceChart>(savedChart());
  treemap = $state.raw<Treemap | null>(null);

  /** Elementos marcados para mover a la papelera. */
  readonly picks = new SvelteMap<number, Pick>();
  picksBytes = $derived([...this.picks.values()].reduce((s, p) => s + p.size, 0));

  error = $state<string | null>(null);
  lastDelete = $state<DeleteResult | null>(null);
  lastPermanent = $state(false);

  /** Recupera un análisis previo (al volver a la vista). */
  async restore() {
    if (this.phase !== "idle") return;
    const s = await api.diskSummary();
    if (s) {
      this.summary = s;
      this.phase = "results";
      await this.open(s.root);
    }
  }

  async scan(path: string) {
    this.error = null;
    this.target = path;
    this.phase = "scanning";
    this.progress = { files: 0, dirs: 0, bytes: 0, current: "" };
    this.picks.clear();
    this.lastDelete = null;
    try {
      this.summary = await api.diskScan(path);
      icons.forgetNodes();
      this.tab = "folder";
      this.extFilter = null;
      await this.open(this.summary.root);
      this.phase = "results";
    } catch (e) {
      const msg = String(e);
      this.phase = this.summary ? "results" : "idle";
      if (!msg.includes("cancel")) this.error = msg;
    }
  }

  cancel() {
    void api.cancel("disk");
  }

  /** Navega a un nodo: anillos, lista y migas. */
  async open(id: number) {
    const [view, entries, crumbs] = await Promise.all([
      api.diskView(id, RINGS),
      api.diskChildren(id),
      api.diskCrumbs(id),
    ]);
    if (!view) return;
    this.node = id;
    this.view = view;
    this.entries = entries;
    this.crumbs = crumbs;
    this.largest = this.tab === "largest" ? await api.diskLargest(id) : [];
    this.treemap = this.chart === "map" ? await api.diskTreemap(id) : null;
  }

  async setChart(chart: SpaceChart) {
    this.chart = chart;
    try {
      localStorage.setItem(CHART_KEY, chart);
    } catch {
      /* sin almacenamiento: sólo esta sesión */
    }
    try {
      this.treemap = chart === "map" ? await api.diskTreemap(this.node) : null;
    } catch (e) {
      this.error = String(e);
    }
  }

  async setTab(tab: SpaceTab) {
    this.tab = tab;
    this.extFilter = null;
    if (tab === "largest") this.largest = await api.diskLargest(this.node);
  }

  up() {
    if (this.crumbs.length > 1) void this.open(this.crumbs[this.crumbs.length - 2].id);
  }

  togglePick(p: Pick) {
    if (this.picks.has(p.id)) this.picks.delete(p.id);
    else this.picks.set(p.id, p);
  }

  /** Elimina lo marcado (papelera o permanente). `null` si falló. */
  async deletePicks(permanent: boolean): Promise<DeleteResult | null> {
    const ids = [...this.picks.keys()];
    try {
      const r = await api.diskDelete(ids, permanent);
      this.lastDelete = r;
      this.lastPermanent = permanent;
      this.picks.clear();
      this.summary = await api.diskSummary();
      // El nodo actual pudo desaparecer: si no hay vista, volver a la raíz.
      const view = await api.diskView(this.node, RINGS);
      await this.open(view ? this.node : (this.summary?.root ?? 0));
      return r;
    } catch (e) {
      this.error = String(e);
      return null;
    }
  }

  /** Vuelve a pedir la carpeta actual (p. ej. tras cambiar exclusiones). */
  async refresh() {
    if (this.phase !== "results") return;
    await this.open(this.node);
    if (this.extFilter !== null) await this.showExt(this.extFilter);
  }

  /** Archivos de un tipo en toda la unidad analizada. */
  async showExt(ext: string) {
    if (!this.summary) return;
    this.extFilter = ext;
    try {
      this.extFiles = await api.diskLargest(this.summary.root, ext);
    } catch (e) {
      this.error = String(e);
    }
  }

  clearExt() {
    this.extFilter = null;
    this.extFiles = [];
  }

  /** Vuelve a analizar lo mismo (p. ej. tras borrar algo a mano) y regresa
   *  a la carpeta en la que estabas si sigue existiendo. */
  async rescan() {
    if (!this.summary) return;
    const names = this.crumbs.slice(1).map((c) => c.name);
    const tab = this.tab;
    await this.scan(this.summary.path);
    if (this.phase !== "results") return;
    for (const name of names) {
      const next = this.entries.find((e) => e.kind === "dir" && e.name === name);
      if (!next) break;
      await this.open(next.id);
    }
    if (tab !== "folder") await this.setTab(tab);
  }

  reset() {
    this.phase = "idle";
    this.picks.clear();
    this.lastDelete = null;
  }
}

export const space = new SpaceStore();
