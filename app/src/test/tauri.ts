// `invoke` simulado: cada prueba define qué devuelve cada comando.
import { vi } from "vitest";

type Handler = (args?: Record<string, unknown>) => unknown;

const defaults: Record<string, Handler> = {
  icons: () => ({}),
};

class FakeTauri {
  handlers: Record<string, Handler> = { ...defaults };
  readonly calls = vi.fn<(cmd: string, args?: Record<string, unknown>) => void>();

  async invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
    this.calls(cmd, args);
    const h = this.handlers[cmd];
    if (!h) throw new Error(`comando sin simular: ${cmd}`);
    return h(args);
  }

  reset() {
    this.handlers = { ...defaults };
    this.calls.mockReset();
  }
}

export const tauri = new FakeTauri();
