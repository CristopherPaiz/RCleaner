// Historial de navegación para los botones Atrás/Adelante del ratón (y
// Alt+←/→). Cada paso es una sección y, en Espacio, la carpeta abierta.

import { tick } from "svelte";
import { app, type View } from "./app.svelte";
import { space } from "./space.svelte";

interface Loc {
  view: View;
  /** Carpeta abierta en Espacio. */
  node?: number;
}

const MAX = 60;
const same = (a: Loc, b: Loc) => a.view === b.view && a.node === b.node;

class Nav {
  private back: Loc[] = [];
  private fwd: Loc[] = [];
  private cur: Loc | null = null;
  private moving = false;

  /** Lugar actual (lo llama un `$effect` de App al cambiar de vista o carpeta). */
  record(loc: Loc) {
    if (this.moving) return;
    if (this.cur && same(this.cur, loc)) return;
    if (this.cur) {
      // Moverse dentro de Espacio antes de que haya análisis no cuenta.
      this.back.push(this.cur);
      if (this.back.length > MAX) this.back.shift();
    }
    this.fwd = [];
    this.cur = loc;
  }

  async go(step: -1 | 1) {
    const from = step < 0 ? this.back : this.fwd;
    const to = step < 0 ? this.fwd : this.back;
    const loc = from.pop();
    if (!loc || app.busy) {
      if (loc) from.push(loc);
      return;
    }
    if (this.cur) to.push(this.cur);
    this.moving = true;
    try {
      app.view = loc.view;
      if (loc.view === "space" && loc.node !== undefined && space.phase === "results") await space.open(loc.node);
      this.cur = loc;
      await tick();
    } finally {
      this.moving = false;
    }
  }
}

export const nav = new Nav();
