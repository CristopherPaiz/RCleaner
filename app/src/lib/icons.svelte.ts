// Iconos de archivos, carpetas y apps.
//
// 1. Carpetas de desarrollo conocidas (node_modules, .git, .venv…): iconos de
//    Material Icon Theme (MIT) incluidos en la app (assets/folders).
// 2. Todo lo demás: el icono que muestra el Explorador de Windows, pedido al
//    backend por lotes y guardado en memoria (como WinDirStat).
// Mientras llega, o si Windows no da ninguno, se usa el icono propio de la UI.

import { SvelteMap } from "svelte/reactivity";
import { api } from "./api";

const svgs = import.meta.glob("../assets/folders/*.svg", {
  eager: true,
  query: "?url",
  import: "default",
}) as Record<string, string>;

function svg(name: string): string | undefined {
  return svgs[`../assets/folders/folder-${name}.svg`];
}

/** Nombre de carpeta (minúsculas) → icono de Material Icon Theme. */
const FOLDERS: Record<string, string> = {
  node_modules: "node",
  ".npm": "node",
  "npm-cache": "node",
  ".git": "git",
  ".github": "github",
  ".vscode": "vscode",
  ".idea": "intellij",
  __pycache__: "python",
  ".pytest_cache": "python",
  ".mypy_cache": "python",
  ".ruff_cache": "python",
  ".venv": "python",
  venv: "python",
  ".venvs": "python",
  envs: "python",
  ".conda": "python",
  "site-packages": "python",
  anaconda3: "python",
  miniconda3: "python",
  miniforge3: "python",
  pypoetry: "python",
  pip: "python",
  ".ipynb_checkpoints": "jupyter",
  target: "target",
  ".cargo": "rust",
  ".rustup": "rust",
  go: "go",
  ".gradle": "gradle",
  gradle: "gradle",
  ".android": "android",
  android: "android",
  ".m2": "java",
  ".next": "next",
  ".nuxt": "nuxt",
  ".svelte-kit": "svelte",
  ".angular": "angular",
  ".turbo": "turborepo",
  ".vercel": "vercel",
  ".firebase": "firebase",
  ".aws": "aws",
  ".kube": "kubernetes",
  ".terraform": "terraform",
  ".docker": "docker",
  ".yarn": "yarn",
  yarn: "yarn",
  ".dart_tool": "dart",
  flutter: "flutter",
  ".claude": "claude",
  ".cursor": "cursor",
  dist: "dist",
  build: "dist",
  out: "dist",
  src: "src",
  docs: "docs",
  doc: "docs",
  test: "test",
  tests: "test",
  __tests__: "test",
  coverage: "coverage",
  __mocks__: "mock",
  public: "public",
  temp: "temp",
  tmp: "temp",
  ".tmp": "temp",
  cache: "temp",
  ".cache": "temp",
  ".parcel-cache": "temp",
  logs: "log",
  log: "log",
  config: "config",
  ".config": "config",
  scripts: "scripts",
  tools: "tools",
  lib: "lib",
  libs: "lib",
  include: "include",
  packages: "packages",
  backup: "backup",
  backups: "backup",
  components: "react-components",
  ".circleci": "ci",
  fonts: "font",
  images: "images",
  img: "images",
  audio: "audio",
  database: "database",
  db: "database",
  archive: "archive",
  archives: "archive",
  icons: "svg",
};

/** Icono incluido para una carpeta conocida, por su nombre. */
export function folderSvg(name: string): string | undefined {
  const key = FOLDERS[name.toLowerCase()];
  return key ? svg(key) : undefined;
}

/** Tipos cuyo icono es propio de cada archivo (no del tipo). */
const OWN_ICON = new Set(["exe", "lnk", "ico", "url", "cur", "ani", "scr", "msc", "appref-ms"]);

export function extOf(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
}

/**
 * Clave de icono de Windows para un archivo: por tipo (`ext:zip`), salvo
 * los que traen icono propio, que se piden por id (`node:12`, `dupe:4`).
 */
export function fileKey(name: string, idKey: string): string {
  const ext = extOf(name);
  return OWN_ICON.has(ext) ? idKey : `ext:${ext}`;
}

class IconStore {
  /** Clave → `data:` URL (o "" si Windows no dio ninguno). */
  readonly urls = new SvelteMap<string, string>();
  private queued = new Set<string>();
  private timer: ReturnType<typeof setTimeout> | null = null;

  /** URL del icono si ya está; si no, lo pide (en lote) y devuelve undefined. */
  get(key: string): string | undefined {
    const url = this.urls.get(key);
    if (url === undefined && !this.queued.has(key)) {
      this.queued.add(key);
      this.timer ??= setTimeout(() => void this.flush(), 16);
    }
    return url || undefined;
  }

  private async flush() {
    this.timer = null;
    const keys = [...this.queued].slice(0, 300);
    try {
      const got = await api.icons(keys);
      for (const k of keys) this.urls.set(k, got[k] ?? "");
    } catch {
      for (const k of keys) this.urls.set(k, "");
    }
    for (const k of keys) this.queued.delete(k);
    if (this.queued.size > 0) this.timer ??= setTimeout(() => void this.flush(), 16);
  }

  /** Los `node:` cambian de significado con cada análisis. */
  forgetNodes() {
    for (const k of [...this.urls.keys()]) if (k.startsWith("node:") || k.startsWith("dupe:")) this.urls.delete(k);
  }
}

export const icons = new IconStore();
