// jsdom no calcula layout, así que el scroll no se puede probar haciendo
// scroll. Esta prueba revisa el CSS: un contenedor flex en columna con
// `overflow: auto` encoge a sus hijos al alto de la ventana en vez de hacer
// scroll (un hijo con `overflow: hidden` se recorta y la lista no se ve
// entera). Todo contenedor así debe llevar `> * { flex-shrink: 0 }`.
/// <reference types="vite/client" />
import { describe, expect, it } from "vitest";

const sources = import.meta.glob<string>("./*.svelte", { eager: true, query: "?raw", import: "default" });
const files = Object.keys(sources).map((k) => k.slice(2));

function scrollColumns(css: string): string[] {
  const out: string[] = [];
  for (const m of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const sel = m[1].trim();
    const body = m[2];
    if (/overflow:\s*auto/.test(body) && /display:\s*flex/.test(body) && /flex-direction:\s*column/.test(body)) {
      for (const s of sel.split(",")) out.push(s.trim());
    }
  }
  return out;
}

describe("las vistas hacen scroll", () => {
  it.each(files)("%s: contenedores con scroll no encogen a sus hijos", (file: string) => {
    const src = sources[`./${file}`];
    const css = src.match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? "";
    for (const sel of scrollColumns(css)) {
      const esc = sel.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
      const guard = new RegExp(`${esc}\\s*>\\s*\\*\\s*\\{[^}]*flex-shrink:\\s*0`);
      expect(guard.test(css), `${file}: falta "${sel} > * { flex-shrink: 0; }"`).toBe(true);
    }
  });
});
