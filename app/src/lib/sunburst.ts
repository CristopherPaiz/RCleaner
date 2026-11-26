// Geometría y colores del mapa radial (estilo "sunburst").
// Coordenadas en un viewBox de -1..1; ángulo 0 arriba, sentido horario.

import type { ViewNode } from "./api";

export const CENTER = 0.3;
const OUTER = 0.985;
const MIN_ANGLE = 0.004;

/** Tonos para las carpetas de primer nivel (se repiten si hay más). */
const HUES = [208, 262, 328, 16, 42, 152, 184, 292, 92, 236];

export interface Arc {
  key: string;
  id: number | null;
  name: string;
  size: number;
  kind: ViewNode["kind"];
  depth: number;
  d: string;
  color: string;
}

export function colorFor(kind: ViewNode["kind"], top: number, depth: number, sibling: number): string {
  if (kind === "rest") return "hsl(220 8% 50% / 0.28)";
  if (kind !== "dir") return `hsl(220 7% ${44 + depth * 5}%)`;
  // Cada nivel gira un poco el tono y los hermanos alternan: así una carpeta
  // con un solo hijo grande no queda toda del mismo color.
  const hue = HUES[top % HUES.length] + depth * 16 + ((sibling % 3) - 1) * 11;
  return `hsl(${hue} ${72 - depth * 6}% ${57 + depth * 4 + (sibling % 2) * 5}%)`;
}

function pt(r: number, a: number): string {
  return `${(r * Math.sin(a)).toFixed(5)} ${(-r * Math.cos(a)).toFixed(5)}`;
}

function arcPath(a0: number, a1: number, r0: number, r1: number): string {
  if (a1 - a0 >= Math.PI * 2 - 1e-4) {
    // Anillo completo: dos medias vueltas por cada radio.
    const m = a0 + Math.PI;
    return (
      `M${pt(r1, a0)}A${r1} ${r1} 0 1 1 ${pt(r1, m)}A${r1} ${r1} 0 1 1 ${pt(r1, a0)}` +
      `M${pt(r0, a0)}A${r0} ${r0} 0 1 0 ${pt(r0, m)}A${r0} ${r0} 0 1 0 ${pt(r0, a0)}Z`
    );
  }
  const large = a1 - a0 > Math.PI ? 1 : 0;
  return (
    `M${pt(r1, a0)}A${r1} ${r1} 0 ${large} 1 ${pt(r1, a1)}` +
    `L${pt(r0, a1)}A${r0} ${r0} 0 ${large} 0 ${pt(r0, a0)}Z`
  );
}

/**
 * Anillos para `root` con `rings` niveles. Devuelve también el color asignado
 * a cada hijo directo (para la lista de al lado).
 */
export function layout(root: ViewNode, rings: number): { arcs: Arc[]; colors: Map<number, string> } {
  const arcs: Arc[] = [];
  const colors = new Map<number, string>();
  const width = (OUTER - CENTER) / rings;

  const walk = (node: ViewNode, a0: number, a1: number, depth: number, top: number, path: string) => {
    if (depth > rings || node.size <= 0) return;
    let a = a0;
    node.children.forEach((c, i) => {
      const span = ((a1 - a0) * c.size) / node.size;
      const start = a;
      a += span;
      if (span < MIN_ANGLE) return;
      const t = depth === 1 ? i : top;
      const color = colorFor(c.kind, t, depth - 1, i);
      if (depth === 1 && c.id !== null) colors.set(c.id, color);
      const r0 = CENTER + (depth - 1) * width;
      const key = `${path}/${c.id ?? `rest${i}`}`;
      arcs.push({ key, id: c.id, name: c.name, size: c.size, kind: c.kind, depth, d: arcPath(start, a, r0 + 0.004, r0 + width), color });
      if (c.children.length) walk(c, start, a, depth + 1, t, key);
    });
  };
  walk(root, 0, Math.PI * 2, 1, 0, "");
  return { arcs, colors };
}
