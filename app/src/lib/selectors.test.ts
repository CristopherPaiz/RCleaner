import { describe, expect, it } from "vitest";
import type { ModuleMeta } from "./api";
import { compact, isSelected, resolve } from "./selectors";

const mod = (id: string): ModuleMeta => ({
  id,
  name: id,
  description: "",
  category: "system",
  risk: "safe",
  requires_admin: false,
  processes: [],
});
const modules = ["browser.chrome", "browser.brave", "system.temp", "dev.npm"].map(mod);

describe("selectores de modos", () => {
  it("comodines, categorías y exclusiones", () => {
    expect(isSelected(["*"], "dev.npm")).toBe(true);
    expect(isSelected(["browser.*"], "browser.brave")).toBe(true);
    expect(isSelected(["browser.*", "!browser.brave"], "browser.brave")).toBe(false);
    expect(isSelected(["system.temp"], "dev.npm")).toBe(false);
  });

  it("resuelve y compacta (igual que en Rust)", () => {
    expect([...resolve(["browser.*", "!browser.brave"], modules)]).toEqual(["browser.chrome"]);
    expect(compact(new Set(["browser.chrome", "browser.brave", "dev.npm"]), modules)).toEqual(["browser.*", "dev.*"]);
    expect(compact(new Set(modules.map((m) => m.id)), modules)).toEqual(["*"]);
  });
});
