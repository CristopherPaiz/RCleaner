import { afterEach, describe, expect, it } from "vitest";
import { i18n, resolveLang, t, tn } from "./i18n.svelte";

type Tree = { [k: string]: string | Tree };
const files = import.meta.glob<Tree>("../../../locales/*.json", { eager: true, import: "default" });
const catalogs: Record<string, Record<string, string>> = Object.fromEntries(
  Object.entries(files).map(([p, d]) => [p.match(/([\w-]+)\.json$/)![1], flat(d)]),
);

function flat(o: Tree, prefix = "", out: Record<string, string> = {}) {
  for (const [k, v] of Object.entries(o)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (typeof v === "string") out[key] = v;
    else flat(v, key, out);
  }
  return out;
}

const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
const keys = (d: Record<string, string>) => Object.keys(d).filter((k) => !k.startsWith("_meta"));

const lang = i18n.lang;
afterEach(() => {
  i18n.lang = lang;
});

describe("traducciones", () => {
  const en = catalogs.en;
  const others = Object.keys(catalogs).filter((c) => c !== "en");

  it.each(others)("%s tiene las mismas claves que en.json", (code) => {
    const other = catalogs[code];
    expect(keys(other).filter((k) => !(k in en))).toEqual([]);
    expect(keys(en).filter((k) => !(k in other))).toEqual([]);
  });

  it.each(others)("%s usa los mismos {parámetros} que en.json", (code) => {
    const bad = Object.entries(catalogs[code])
      .filter(([k, v]) => k in en && placeholders(v).join() !== placeholders(en[k]).join())
      .map(([k]) => k);
    expect(bad).toEqual([]);
  });

  it("los plurales van en parejas _one/_other", () => {
    const all = keys(en);
    const ones = all.filter((k) => k.endsWith("_one")).map((k) => k.slice(0, -4));
    expect(ones.filter((k) => !all.includes(`${k}_other`))).toEqual([]);
  });
});

describe("t / tn / resolveLang", () => {
  it("elige el mejor idioma disponible", () => {
    expect(resolveLang("es-MX")).toBe("es");
    expect(resolveLang("en_GB")).toBe("en");
    expect(resolveLang("xx")).toBe("en");
    expect(resolveLang(null)).toBe("en");
  });

  it("sustituye parámetros y elige el plural", () => {
    i18n.lang = "es";
    expect(t("ui.update.title", { v: "1.2.3" })).toBe("RCleaner 1.2.3 disponible");
    expect(tn("ui.results.options", 1)).toBe("1 opción");
    expect(tn("ui.results.options", 3)).toBe("3 opciones");
    i18n.lang = "en";
    expect(tn("ui.results.options", 3)).toBe("3 options");
  });

  it("una clave desconocida se devuelve tal cual", () => {
    expect(t("no.existe")).toBe("no.existe");
  });
});
