// Misma lógica de selectores que `crp_core::modes` (para el editor de modos).

import type { ModuleMeta } from "./api";

function matches(selector: string, id: string): boolean {
  if (selector === "*" || selector === id) return true;
  if (selector.endsWith(".*")) return id.startsWith(selector.slice(0, -1));
  return false;
}

/** ¿Selecciona esta lista de selectores el módulo `id`? */
export function isSelected(selectors: string[], id: string): boolean {
  const incl = selectors.filter((s) => !s.startsWith("!"));
  const excl = selectors.filter((s) => s.startsWith("!")).map((s) => s.slice(1));
  return incl.some((s) => matches(s, id)) && !excl.some((s) => matches(s, id));
}

/** Ids de módulos que cubre un modo. */
export function resolve(selectors: string[], modules: ModuleMeta[]): Set<string> {
  return new Set(modules.filter((m) => isSelected(selectors, m.id)).map((m) => m.id));
}

/** Lista compacta de selectores: usa `cat.*` cuando se eligió una categoría entera. */
export function compact(chosen: Set<string>, modules: ModuleMeta[]): string[] {
  if (modules.length > 0 && modules.every((m) => chosen.has(m.id))) return ["*"];
  const byPrefix = new Map<string, string[]>();
  for (const m of modules) {
    const prefix = m.id.split(".")[0];
    byPrefix.set(prefix, [...(byPrefix.get(prefix) ?? []), m.id]);
  }
  const out: string[] = [];
  for (const [prefix, ids] of byPrefix) {
    const picked = ids.filter((id) => chosen.has(id));
    if (picked.length === ids.length) out.push(`${prefix}.*`);
    else out.push(...picked);
  }
  return out;
}
