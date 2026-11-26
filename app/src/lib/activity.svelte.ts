// Lo que el backend está eliminando ahora mismo (limpieza, Espacio y
// Duplicados). Se alimenta de los eventos `crp://now` y `crp://delete`.

import type { DeleteProgress } from "./api";

class Activity {
  /** Ruta que se está eliminando (o `null`). */
  now = $state<string | null>(null);
  /** Progreso del borrado de Espacio/Duplicados. */
  del = $state<DeleteProgress>({ done: 0, total: 0, current: null });

  onDelete(p: DeleteProgress) {
    this.del = p;
    if (p.current) this.now = p.current;
  }

  reset() {
    this.now = null;
    this.del = { done: 0, total: 0, current: null };
  }
}

export const activity = new Activity();

/** Copia texto al portapapeles (sin fallar si no se puede). */
export async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    /* sin portapapeles: nada que hacer */
  }
}
