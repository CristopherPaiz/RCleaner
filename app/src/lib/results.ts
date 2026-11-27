// Reglas de la lista de resultados de Limpieza (sin estado: se prueban solas).

import type { Finding } from "./api";

export interface Subgroup {
  id: string;
  name: string;
  items: Finding[];
  bytes: number;
}

/** Hallazgos agrupados por módulo (un navegador = un grupo), el mayor primero. */
export function bySubgroup(items: Finding[], nameOf: (moduleId: string) => string): Subgroup[] {
  const map = new Map<string, Finding[]>();
  for (const f of items) map.set(f.module_id, [...(map.get(f.module_id) ?? []), f]);
  return [...map]
    .map(([id, list]) => ({ id, name: nameOf(id), items: list, bytes: list.reduce((s, f) => s + f.bytes, 0) }))
    .sort((a, b) => b.bytes - a.bytes);
}

/** "Caché de Brave" dentro del grupo "Brave" → "Caché"; "Google Chrome cookies" → "Cookies". */
export function shortLabel(label: string, name: string): string {
  const words = name.split(" ");
  let out = label;
  for (const n of [name, words.at(-1) ?? name]) out = out.replace(n, "");
  out = out
    .replace(/\s+(de|of|del)\s*$/i, "")
    .replace(/^\s*(de|of|del)\s+/i, "")
    .trim();
  return out ? out[0].toUpperCase() + out.slice(1) : label;
}

/**
 * ¿Se marca al elegir "Seguros" o "Todo"? Los datos del usuario (riesgo
 * avanzado: cookies, historial, modelos…) nunca se marcan en bloque.
 */
export function bulkPick(f: Finding, which: "safe" | "all"): boolean {
  if (f.risk === "safe") return true;
  return which === "all" && f.risk !== "advanced";
}

/** Casillas de grupo: cuentan los marcables, sin datos del usuario no marcados ya. */
export function groupCandidates(items: Finding[], selectable: (f: Finding) => boolean, selected: Set<string>): Finding[] {
  return items.filter((f) => selectable(f) && (f.risk !== "advanced" || selected.has(f.id)));
}
