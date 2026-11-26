import type { Category, Risk } from "./api";

const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** Tamaño legible en unidades binarias (como el Explorador de Windows). */
export function formatBytes(bytes: number): string {
  let v = bytes;
  let i = 0;
  while (v >= 1024 && i < UNITS.length - 1) {
    v /= 1024;
    i++;
  }
  return i === 0 ? `${bytes} B` : `${v.toFixed(v >= 100 ? 0 : 1)} ${UNITS[i]}`;
}

/** Fecha corta local a partir de segundos Unix. */
export function formatDate(unixSecs: number): string {
  return new Date(unixSecs * 1000).toLocaleString("es", { dateStyle: "medium", timeStyle: "short" });
}

/** "hoy", "hace 3 días", "hace 5 meses"… */
export function formatAge(days: number): string {
  if (days <= 0) return "hoy";
  if (days === 1) return "ayer";
  if (days < 60) return `hace ${days} días`;
  if (days < 730) return `hace ${Math.round(days / 30)} meses`;
  return `hace ${Math.round(days / 365)} años`;
}

export const RISK_LABEL: Record<Risk, string> = {
  safe: "Seguro",
  moderate: "Moderado",
  advanced: "Avanzado",
};

export const CATEGORY_LABEL: Record<Category, string> = {
  system: "Sistema",
  browser: "Navegadores",
  dev: "Desarrollo",
  ai: "Modelos de IA",
  apps: "Aplicaciones",
  registry: "Registro",
};

export const CATEGORY_ORDER: Category[] = ["system", "browser", "dev", "ai", "apps", "registry"];

export const MODE_ICONS = ["zap", "code", "brain", "gamepad", "layers", "star", "rocket", "shield"];

/** Convierte un nombre en id válido para un modo: "Mi Modo!" → "mi-modo". */
export function slugify(name: string): string {
  return name
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 40);
}
