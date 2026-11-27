// Entorno de las pruebas: sin Tauri. `invoke` responde con `tauri.handlers`
// (por comando) y los eventos no llegan nunca.
import { afterEach, vi } from "vitest";
import { tauri } from "./tauri";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (cmd: string, args?: Record<string, unknown>) => tauri.invoke(cmd, args),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: async () => () => {},
}));

// APIs del navegador que jsdom no trae.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
};
Element.prototype.scrollIntoView ??= function () {};

afterEach(() => tauri.reset());
