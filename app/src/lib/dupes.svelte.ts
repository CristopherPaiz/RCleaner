// Estado del buscador de duplicados.

import { SvelteSet } from "svelte/reactivity";
import { api, type DeleteResult, type DupeGroup, type DupeProgress, type DupeReport } from "./api";

export type DupesPhase = "idle" | "scanning" | "results";
/** Qué copia se conserva al seleccionar automáticamente. */
export type KeepRule = "oldest" | "newest" | "shortest";

export const MIN_SIZES = [
  { label: "100 KB", bytes: 100 * 1024 },
  { label: "1 MB", bytes: 1024 * 1024 },
  { label: "10 MB", bytes: 10 * 1024 * 1024 },
  { label: "100 MB", bytes: 100 * 1024 * 1024 },
];

class DupesStore {
  phase = $state<DupesPhase>("idle");
  roots = $state<string[]>([]);
  minSize = $state(1024 * 1024);
  includeSystem = $state(false);

  progress = $state<DupeProgress>({ phase: "listing", files: 0 });
  report = $state<DupeReport | null>(null);
  readonly selected = new SvelteSet<number>();
  keep = $state<KeepRule>("oldest");

  error = $state<string | null>(null);
  lastDelete = $state<DeleteResult | null>(null);

  selectedBytes = $derived.by(() => {
    let total = 0;
    for (const g of this.report?.groups ?? []) {
      for (const f of g.files) if (this.selected.has(f.id)) total += g.size;
    }
    return total;
  });

  /** Raíz por defecto: la carpeta del usuario. */
  init(home: string) {
    if (this.roots.length === 0 && home) this.roots = [home];
  }

  async restore() {
    if (this.phase !== "idle") return;
    const r = await api.dupesReport();
    if (r) {
      this.report = r;
      this.phase = "results";
    }
  }

  async scan() {
    this.error = null;
    this.lastDelete = null;
    this.phase = "scanning";
    this.progress = { phase: "listing", files: 0 };
    try {
      this.report = await api.dupesScan(this.roots, this.minSize, this.includeSystem);
      this.autoSelect(this.keep);
      this.phase = "results";
    } catch (e) {
      const msg = String(e);
      this.phase = this.report ? "results" : "idle";
      if (!msg.includes("cancel")) this.error = msg;
    }
  }

  cancel() {
    void api.cancel("dupes");
  }

  /** Marca todas las copias salvo la que indica la regla. */
  autoSelect(rule: KeepRule) {
    this.keep = rule;
    this.selected.clear();
    for (const g of this.report?.groups ?? []) {
      const keep = keeper(g, rule);
      for (const f of g.files) if (f.id !== keep) this.selected.add(f.id);
    }
  }

  /** Alterna un archivo sin permitir marcar todas las copias de un grupo. */
  toggle(g: DupeGroup, id: number) {
    if (this.selected.has(id)) {
      this.selected.delete(id);
      return;
    }
    const others = g.files.filter((f) => f.id !== id);
    if (others.every((f) => this.selected.has(f.id))) return;
    this.selected.add(id);
  }

  async deleteSelected() {
    try {
      this.lastDelete = await api.dupesDelete([...this.selected]);
      this.report = await api.dupesReport();
      this.selected.clear();
    } catch (e) {
      this.error = String(e);
    }
  }

  reset() {
    this.phase = "idle";
    this.selected.clear();
    this.lastDelete = null;
  }
}

function keeper(g: DupeGroup, rule: KeepRule): number {
  const files = [...g.files];
  if (rule === "newest") files.sort((a, b) => b.modified - a.modified);
  else if (rule === "shortest") files.sort((a, b) => a.path.length - b.path.length);
  else files.sort((a, b) => a.modified - b.modified);
  return files[0].id;
}

export const dupes = new DupesStore();
