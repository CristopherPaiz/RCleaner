// Actualizaciones: buscar al abrir (si está activado), avisar en el riel e
// instalar desde un diálogo.
import { api, type UpdateInfo, type UpdateProgress } from "./api";

export type UpdateStatus = "idle" | "checking" | "latest" | "available" | "installing" | "error";

class UpdateStore {
  info = $state<UpdateInfo | null>(null);
  status = $state<UpdateStatus>("idle");
  error = $state<string | null>(null);
  progress = $state<UpdateProgress | null>(null);
  /** Diálogo abierto. */
  open = $state(false);

  /** `silent`: al abrir la app; sin red o sin release no molesta. */
  async check(silent = false) {
    if (this.status === "checking" || this.status === "installing") return;
    this.status = "checking";
    this.error = null;
    try {
      this.info = await api.checkUpdate();
      this.status = this.info ? "available" : "latest";
    } catch (e) {
      this.status = silent ? "idle" : "error";
      this.error = silent ? null : String(e);
    }
  }

  async install() {
    if (!this.info?.installable) {
      void api.openLink("releases");
      return;
    }
    this.status = "installing";
    this.progress = null;
    try {
      await api.installUpdate(); // al terminar, el instalador reinicia RCleaner
    } catch (e) {
      this.status = "error";
      this.error = String(e);
    }
  }

  onProgress(p: UpdateProgress) {
    this.progress = p;
  }

  /** 0–1, o `null` si el servidor no dice el tamaño. */
  get fraction(): number | null {
    const p = this.progress;
    return p?.total ? Math.min(1, p.done / p.total) : null;
  }
}

export const update = new UpdateStore();
