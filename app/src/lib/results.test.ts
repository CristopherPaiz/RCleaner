import { describe, expect, it } from "vitest";
import type { Finding, Risk } from "./api";
import { bulkPick, bySubgroup, groupCandidates, shortLabel } from "./results";

const finding = (id: string, module_id: string, bytes: number, risk: Risk = "safe"): Finding => ({
  id,
  module_id,
  category: "browser",
  label: id,
  bytes,
  items: 1,
  risk,
  requires_admin: false,
  selected: false,
});

describe("shortLabel", () => {
  it("quita el nombre del navegador en español e inglés", () => {
    expect(shortLabel("Caché de Brave", "Brave")).toBe("Caché");
    expect(shortLabel("Cookies de Google Chrome", "Google Chrome")).toBe("Cookies");
    expect(shortLabel("Brave cookies", "Brave")).toBe("Cookies");
    expect(shortLabel("Microsoft Edge history", "Microsoft Edge")).toBe("History");
  });

  it("deja la etiqueta si no queda nada", () => {
    expect(shortLabel("Brave", "Brave")).toBe("Brave");
  });
});

describe("bySubgroup", () => {
  it("agrupa por módulo y ordena por tamaño", () => {
    const groups = bySubgroup(
      [finding("a", "browser.chrome", 10), finding("b", "browser.brave", 50), finding("c", "browser.chrome", 60)],
      (id) => id.split(".")[1],
    );
    expect(groups.map((g) => [g.name, g.bytes, g.items.length])).toEqual([
      ["chrome", 70, 2],
      ["brave", 50, 1],
    ]);
  });
});

describe("selección en bloque", () => {
  const safe = finding("safe", "m", 1, "safe");
  const moderate = finding("mod", "m", 1, "moderate");
  const advanced = finding("adv", "m", 1, "advanced");

  it("Seguros sólo marca lo seguro; Todo nunca marca datos del usuario", () => {
    expect([safe, moderate, advanced].filter((f) => bulkPick(f, "safe"))).toEqual([safe]);
    expect([safe, moderate, advanced].filter((f) => bulkPick(f, "all"))).toEqual([safe, moderate]);
  });

  it("la casilla de grupo ignora lo avanzado salvo que ya esté marcado", () => {
    const all = [safe, moderate, advanced];
    expect(groupCandidates(all, () => true, new Set())).toEqual([safe, moderate]);
    expect(groupCandidates(all, () => true, new Set(["adv"]))).toEqual(all);
    expect(groupCandidates(all, (f) => f !== moderate, new Set())).toEqual([safe]);
  });
});
