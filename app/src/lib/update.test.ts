import { beforeEach, describe, expect, it } from "vitest";
import { tauri } from "../test/tauri";
import { update } from "./update.svelte";

beforeEach(() => {
  update.info = null;
  update.status = "idle";
  update.error = null;
  update.progress = null;
});

describe("actualizaciones", () => {
  it("avisa cuando hay versión nueva", async () => {
    tauri.handlers.check_update = () => ({ version: "9.9.9", notes: "x", installable: true });
    await update.check();
    expect(update.status).toBe("available");
    expect(update.info?.version).toBe("9.9.9");
  });

  it("sin versión nueva dice que está al día", async () => {
    tauri.handlers.check_update = () => null;
    await update.check();
    expect(update.status).toBe("latest");
  });

  it("al abrir, un fallo (sin red) no muestra error", async () => {
    tauri.handlers.check_update = () => {
      throw new Error("offline");
    };
    await update.check(true);
    expect(update.status).toBe("idle");
    expect(update.error).toBeNull();
    await update.check();
    expect(update.status).toBe("error");
    expect(update.error).toContain("offline");
  });

  it("la versión portable abre la página de descargas en vez de instalar", async () => {
    tauri.handlers.open_link = () => null;
    update.info = { version: "9.9.9", notes: null, installable: false };
    await update.install();
    expect(tauri.calls).toHaveBeenCalledWith("open_link", { target: "releases" });
    expect(tauri.calls.mock.calls.some(([cmd]) => cmd === "install_update")).toBe(false);
  });

  it("calcula el progreso de la descarga", () => {
    update.onProgress({ done: 25, total: 100 });
    expect(update.fraction).toBe(0.25);
    update.onProgress({ done: 25, total: null });
    expect(update.fraction).toBeNull();
  });
});
