// Mapa de rectángulos estilo WinDirStat: rectángulos "squarified" (Bruls et
// al.) con sombreado de cojín (van Wijk y van de Wetering): cada nivel añade
// un relieve y la luz lo convierte en almohadillas brillantes, así se ve qué
// archivos van juntos sin dibujar bordes.

import type { Treemap } from "./api";

/** Colores por tipo de archivo, en orden de espacio ocupado. */
const PALETTE: [number, number, number][] = [
  [86, 92, 230], // azul violeta
  [222, 196, 0], // amarillo
  [64, 210, 92], // verde
  [236, 92, 88], // rojo salmón
  [212, 40, 214], // magenta
  [0, 196, 204], // cian
  [248, 140, 30], // naranja
  [64, 150, 255], // azul
  [150, 220, 40], // lima
  [255, 110, 170], // rosa
  [140, 100, 240], // lila
  [40, 200, 160], // turquesa
];
const OTHER: [number, number, number] = [150, 154, 162];
const DIR: [number, number, number] = [120, 124, 132];
const REST: [number, number, number] = [80, 83, 90];

/** Color de la extensión según su puesto (0 = la que más ocupa). */
export function extColor(rank: number): [number, number, number] {
  return rank >= 0 && rank < PALETTE.length ? PALETTE[rank] : OTHER;
}

export function rgb(c: [number, number, number]): string {
  return `rgb(${c[0]} ${c[1]} ${c[2]})`;
}

// Parámetros del cojín (los de WinDirStat).
const HEIGHT = 0.38;
const SCALE = 0.91;
const AMBIENT = 0.15;
const BRIGHT = 1.45;
const [LX, LY, LZ] = (() => {
  const l = Math.hypot(-1, -1, 10);
  return [-1 / l, -1 / l, 10 / l];
})();

export interface Layout {
  /** Rectángulo de cada pieza: x0, y0, x1, y1 (en píxeles del lienzo). */
  rects: Float64Array;
  /** Hijos de cada pieza. */
  children: number[][];
  /** Profundidad (0 = raíz). */
  depth: Uint16Array;
  /** Piezas sin hijos (las que se pintan). */
  leaves: number[];
}

/** Calcula los rectángulos de todas las piezas para un lienzo `w`×`h`. */
export function layoutTreemap(tm: Treemap, w: number, h: number): Layout {
  const n = tm.parent.length;
  const children: number[][] = Array.from({ length: n }, () => []);
  const depth = new Uint16Array(n);
  for (let i = 1; i < n; i++) {
    children[tm.parent[i]].push(i);
    depth[i] = depth[tm.parent[i]] + 1;
  }
  const rects = new Float64Array(n * 4);
  rects.set([0, 0, w, h], 0);
  const leaves: number[] = [];
  // Las piezas vienen por niveles: el padre siempre tiene su rectángulo antes.
  for (let i = 0; i < n; i++) {
    const kids = children[i];
    if (kids.length === 0) {
      leaves.push(i);
      continue;
    }
    kids.sort((a, b) => tm.size[b] - tm.size[a]); // "resto" puede no ir al final
    squarify(kids, tm.size, rects, rects[i * 4], rects[i * 4 + 1], rects[i * 4 + 2], rects[i * 4 + 3]);
  }
  return { rects, children, depth, leaves };
}

/** Reparte `items` (ya ordenados de mayor a menor) en el rectángulo dado. */
function squarify(items: number[], size: number[], out: Float64Array, x0: number, y0: number, x1: number, y1: number) {
  let total = 0;
  for (const i of items) total += size[i];
  if (total <= 0) return;
  let start = 0;
  while (start < items.length) {
    const w = x1 - x0;
    const h = y1 - y0;
    if (w <= 0 || h <= 0) {
      for (let k = start; k < items.length; k++) out.set([x0, y0, x0, y0], items[k] * 4);
      return;
    }
    const side = Math.min(w, h);
    const scale = (w * h) / total; // píxeles por byte
    // Añadir a la fila mientras el peor aspecto no empeore.
    let end = start;
    let rowSum = 0;
    let worst = Infinity;
    while (end < items.length) {
      const s = size[items[end]] * scale;
      const sum = rowSum + s;
      const max = size[items[start]] * scale;
      const min = s;
      const r = Math.max((side * side * max) / (sum * sum), (sum * sum) / (side * side * min));
      if (r > worst && end > start) break;
      worst = r;
      rowSum = sum;
      end++;
    }
    // Colocar la fila a lo largo del lado corto.
    const thick = rowSum / side;
    let pos = 0;
    for (let k = start; k < end; k++) {
      const len = (size[items[k]] * scale) / thick;
      if (w >= h) out.set([x0, y0 + pos, x0 + thick, y0 + pos + len], items[k] * 4);
      else out.set([x0 + pos, y0, x0 + pos + len, y0 + thick], items[k] * 4);
      pos += len;
    }
    if (w >= h) x0 += thick;
    else y0 += thick;
    for (let k = start; k < end; k++) total -= size[items[k]];
    start = end;
  }
}

/**
 * Pinta el mapa en `img` y rellena `hit` (pieza bajo cada píxel, -1 = nada).
 * `rank` = puesto de cada extensión de `tm.exts` (para el color).
 */
export function paintTreemap(tm: Treemap, lay: Layout, rank: Int32Array, img: ImageData, hit: Int32Array) {
  const { width, height, data } = img;
  hit.fill(-1);
  data.fill(0);
  const n = tm.parent.length;
  // Coeficientes de la superficie de cada pieza: [x², y², x, y].
  const surf = new Float64Array(n * 4);
  const rect = lay.rects;
  for (let i = 0; i < n; i++) {
    const p = tm.parent[i];
    if (i > 0) surf.set(surf.subarray(p * 4, p * 4 + 4), i * 4);
    const h = HEIGHT * Math.pow(SCALE, lay.depth[i]);
    const x0 = rect[i * 4], y0 = rect[i * 4 + 1], x1 = rect[i * 4 + 2], y1 = rect[i * 4 + 3];
    if (x1 - x0 > 0 && y1 - y0 > 0) {
      const wf = (4 * h) / (x1 - x0);
      surf[i * 4 + 2] += wf * (x1 + x0);
      surf[i * 4] -= wf;
      const hf = (4 * h) / (y1 - y0);
      surf[i * 4 + 3] += hf * (y1 + y0);
      surf[i * 4 + 1] -= hf;
    }
  }

  for (const i of lay.leaves) {
    const x0 = Math.round(rect[i * 4]), y0 = Math.round(rect[i * 4 + 1]);
    const x1 = Math.min(width, Math.round(rect[i * 4 + 2])), y1 = Math.min(height, Math.round(rect[i * 4 + 3]));
    if (x1 <= x0 || y1 <= y0) continue;
    const kind = tm.kind[i];
    const base = kind === "file" ? extColor(tm.ext[i] === 0xffff ? -1 : rank[tm.ext[i]]) : kind === "dir" ? DIR : REST;
    const s0 = surf[i * 4], s1 = surf[i * 4 + 1], s2 = surf[i * 4 + 2], s3 = surf[i * 4 + 3];
    for (let y = y0; y < y1; y++) {
      const ny = -(2 * s1 * (y + 0.5) + s3);
      let o = (y * width + x0) * 4;
      let hp = y * width + x0;
      for (let x = x0; x < x1; x++) {
        const nx = -(2 * s0 * (x + 0.5) + s2);
        let cos = (nx * LX + ny * LY + LZ) / Math.sqrt(nx * nx + ny * ny + 1);
        if (cos > 1) cos = 1;
        const light = (AMBIENT + (1 - AMBIENT) * Math.max(0, cos)) * BRIGHT;
        data[o] = Math.min(255, base[0] * light);
        data[o + 1] = Math.min(255, base[1] * light);
        data[o + 2] = Math.min(255, base[2] * light);
        data[o + 3] = 255;
        hit[hp++] = i;
        o += 4;
      }
    }
  }
}
