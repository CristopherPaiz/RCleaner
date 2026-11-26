import type { Category, Risk } from "./api";
import { i18n, t, tn } from "./i18n.svelte";

const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** Tamaño legible en unidades binarias (como el Explorador de Windows), con coma decimal. */
export function formatBytes(bytes: number): string {
  const [v, u] = splitBytes(bytes);
  return `${v} ${u}`;
}

/** Valor y unidad por separado (para cifras grandes con la unidad en pequeño). */
export function splitBytes(bytes: number): [string, string] {
  let v = bytes;
  let i = 0;
  while (v >= 1024 && i < UNITS.length - 1) {
    v /= 1024;
    i++;
  }
  if (i === 0) return [String(bytes), "B"];
  return [v.toLocaleString(i18n.lang, { maximumFractionDigits: v >= 100 ? 0 : 1 }), UNITS[i]];
}

/** Número con separador de miles. */
export function formatCount(n: number): string {
  return n.toLocaleString(i18n.lang);
}

/** Icono para un nombre de archivo según su extensión. */
export function fileIcon(name: string): string {
  const ext = name.includes(".") ? name.split(".").pop()!.toLowerCase() : "";
  if (["jpg", "jpeg", "png", "gif", "webp", "heic", "bmp", "tif", "tiff", "raw", "cr2", "nef", "psd", "svg"].includes(ext)) return "image";
  if (["mp4", "mkv", "mov", "avi", "wmv", "webm", "m4v", "flv"].includes(ext)) return "film";
  if (["mp3", "flac", "wav", "ogg", "m4a", "aac", "wma"].includes(ext)) return "music";
  if (["zip", "rar", "7z", "tar", "gz", "xz", "iso", "img", "vhd", "vhdx", "cab", "msi"].includes(ext)) return "archive";
  return "file";
}

/** Carpeta y nombre de una ruta de Windows. */
export function splitPath(path: string): [string, string] {
  const i = path.lastIndexOf("\\");
  return i < 0 ? ["", path] : [path.slice(0, i), path.slice(i + 1)];
}

/** Fecha corta local a partir de segundos Unix. */
export function formatDate(unixSecs: number): string {
  return new Date(unixSecs * 1000).toLocaleString(i18n.lang, { dateStyle: "medium", timeStyle: "short" });
}

/** "hoy", "hace 3 días", "hace 5 meses"… (en el idioma actual) */
export function formatAge(days: number): string {
  if (days <= 0) return t("ui.age.today");
  if (days === 1) return t("ui.age.yesterday");
  if (days < 60) return tn("ui.age.days", days);
  if (days < 730) return tn("ui.age.months", Math.round(days / 30));
  return tn("ui.age.years", Math.round(days / 365));
}

export const riskLabel = (r: Risk) => t(`risk.${r}`);
export const categoryLabel = (c: Category) => t(`category.${c}`);

export const CATEGORY_ICON: Record<Category, string> = {
  system: "monitor",
  browser: "globe",
  dev: "code",
  ai: "cpu",
  apps: "box",
  registry: "key",
};

export const CATEGORY_ORDER: Category[] = ["system", "browser", "dev", "ai", "apps", "registry"];

export const MODE_ICONS = ["zap", "code", "brain", "gamepad", "layers", "star", "rocket", "shield"];

/** Par de colores de cada modo según su icono (también vale para los modos propios). */
export const MODE_COLORS: Record<string, [string, string]> = {
  zap: ["#10b98a", "#9be15d"],
  code: ["#3d7bff", "#3fd0ff"],
  brain: ["#9b5cff", "#f062c0"],
  gamepad: ["#ff4d5e", "#ff9f43"],
  layers: ["#0f9fb0", "#4f6dff"],
  star: ["#f59e0b", "#fcd34d"],
  rocket: ["#ff5f7e", "#a855f7"],
  shield: ["#14b8a6", "#22c55e"],
};

export function modeColors(icon: string | undefined): [string, string] {
  return MODE_COLORS[icon ?? ""] ?? MODE_COLORS.zap;
}

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
