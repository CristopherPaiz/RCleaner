/// <reference types="vite/client" />
// Traducciones de la interfaz. Los textos viven en `locales/<código>.json`
// (raíz del repositorio) y los comparte el backend en Rust: añadir un idioma
// es añadir su archivo. `en.json` es la referencia; lo que falte cae a inglés.

type Dict = Record<string, string>;

const raw = import.meta.glob<Record<string, unknown>>("../../../locales/*.json", { eager: true, import: "default" });

function flatten(obj: Record<string, unknown>, prefix = "", out: Dict = {}): Dict {
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (typeof v === "string") out[key] = v;
    else if (v && typeof v === "object") flatten(v as Record<string, unknown>, key, out);
  }
  return out;
}

const catalogs: Record<string, Dict> = {};
for (const [path, json] of Object.entries(raw) as [string, Record<string, unknown>][]) {
  const code = path.match(/([\w-]+)\.json$/)![1];
  catalogs[code] = flatten(json);
}

export const FALLBACK = "en";

/** Idiomas disponibles con su nombre en su propio idioma. */
export const LANGUAGES = Object.keys(catalogs)
  .sort()
  .map((code) => ({ code, name: catalogs[code]["_meta.name"] ?? code }));

/** Mejor idioma disponible para `wanted` (`es-MX` → `es`). */
export function resolveLang(wanted: string | null | undefined): string {
  const w = (wanted ?? "").trim().replace("_", "-").toLowerCase();
  if (catalogs[w]) return w;
  const base = w.split("-")[0];
  return catalogs[base] ? base : FALLBACK;
}

class I18n {
  lang = $state(resolveLang(typeof navigator !== "undefined" ? navigator.language : FALLBACK));
}

export const i18n = new I18n();

type Params = Record<string, string | number>;

/** Texto de `key` en el idioma actual; `{nombre}` se sustituye con `params`. */
export function t(key: string, params?: Params): string {
  let s = catalogs[i18n.lang]?.[key] ?? catalogs[FALLBACK]?.[key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      s = s.replaceAll(`{${k}}`, typeof v === "number" ? v.toLocaleString(i18n.lang) : v);
    }
  }
  return s;
}

/** Plural: `key_one` si `n === 1`, si no `key_other`. `{n}` es el número. */
export function tn(key: string, n: number, params?: Params): string {
  return t(`${key}_${n === 1 ? "one" : "other"}`, { n, ...params });
}
