//! Traducciones.
//!
//! Los textos viven en `locales/<código>.json` en la raíz del repositorio y
//! los comparten el motor, la CLI y la interfaz. Las claves se anidan por
//! secciones (`module.system.user_temp.name`); `en.json` es la referencia y
//! cualquier clave que falte en otro idioma cae a inglés.
//!
//! - `{nombre}` dentro de un texto se sustituye con [`tf`].
//! - Plurales: [`tn`] busca `clave_one` / `clave_other` según `n`.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{OnceLock, RwLock};

include!(concat!(env!("OUT_DIR"), "/locales.rs"));

/// Idioma de referencia y de respaldo.
pub const FALLBACK: &str = "en";

static CURRENT: RwLock<String> = RwLock::new(String::new());

type Catalog = HashMap<String, String>;

fn catalogs() -> &'static HashMap<&'static str, Catalog> {
    static C: OnceLock<HashMap<&'static str, Catalog>> = OnceLock::new();
    C.get_or_init(|| {
        LOCALES
            .iter()
            .map(|(code, src)| {
                let value: serde_json::Value = serde_json::from_str(src)
                    .unwrap_or_else(|e| panic!("locales/{code}.json no es JSON válido: {e}"));
                let mut flat = Catalog::new();
                flatten("", &value, &mut flat);
                (*code, flat)
            })
            .collect()
    })
}

fn flatten(prefix: &str, v: &serde_json::Value, out: &mut Catalog) {
    match v {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(&key, v, out);
            }
        }
        serde_json::Value::String(s) => {
            out.insert(prefix.to_owned(), s.clone());
        }
        _ => {}
    }
}

/// Un idioma disponible.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Language {
    pub code: String,
    /// Nombre en su propio idioma ("Español", "English").
    pub name: String,
}

pub fn languages() -> Vec<Language> {
    let mut v: Vec<Language> = catalogs()
        .iter()
        .map(|(code, c)| Language {
            code: (*code).to_owned(),
            name: c
                .get("_meta.name")
                .cloned()
                .unwrap_or_else(|| (*code).to_owned()),
        })
        .collect();
    v.sort_by(|a, b| a.code.cmp(&b.code));
    v
}

/// Idioma disponible que mejor encaja con `wanted` (`es-MX` → `es`).
pub fn resolve(wanted: &str) -> &'static str {
    let c = catalogs();
    let wanted = wanted.trim().replace('_', "-").to_ascii_lowercase();
    if let Some((code, _)) = c.get_key_value(wanted.as_str()) {
        return code;
    }
    let base = wanted.split('-').next().unwrap_or_default();
    c.get_key_value(base).map_or(FALLBACK, |(code, _)| code)
}

/// Cambia el idioma de los textos que se generen a partir de ahora.
/// Devuelve el código efectivo.
pub fn set_language(wanted: &str) -> &'static str {
    let code = resolve(wanted);
    *CURRENT
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = code.to_owned();
    code
}

pub fn language() -> String {
    let cur = CURRENT
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if cur.is_empty() {
        FALLBACK.to_owned()
    } else {
        cur.clone()
    }
}

fn lookup(key: &str) -> Option<&'static str> {
    let c = catalogs();
    c.get(language().as_str())
        .and_then(|m| m.get(key))
        .or_else(|| c.get(FALLBACK).and_then(|m| m.get(key)))
        .map(String::as_str)
}

/// Texto de `key` si existe en el idioma actual o en inglés.
pub fn get(key: &str) -> Option<String> {
    lookup(key).map(str::to_owned)
}

/// Texto de `key` en el idioma actual (o la propia clave si no existe).
pub fn t(key: &str) -> String {
    lookup(key).map_or_else(|| key.to_owned(), str::to_owned)
}

/// Como [`t`] sustituyendo `{nombre}` por cada argumento.
pub fn tf(key: &str, args: &[(&str, &dyn Display)]) -> String {
    let mut s = t(key);
    for (name, value) in args {
        s = s.replace(&format!("{{{name}}}"), &value.to_string());
    }
    s
}

/// Plural: usa `key_one` si `n == 1` y `key_other` si no; `{n}` es el número.
pub fn tn(key: &str, n: u64, args: &[(&str, &dyn Display)]) -> String {
    let k = if n == 1 {
        format!("{key}_one")
    } else {
        format!("{key}_other")
    };
    let mut all: Vec<(&str, &dyn Display)> = vec![("n", &n)];
    all.extend_from_slice(args);
    tf(&k, &all)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(s: &str) -> Vec<String> {
        let mut v: Vec<String> = s
            .split('{')
            .skip(1)
            .filter_map(|p| p.split_once('}').map(|(a, _)| a.to_owned()))
            .collect();
        v.sort();
        v
    }

    #[test]
    fn every_locale_matches_the_reference() {
        let c = catalogs();
        let reference = &c[FALLBACK];
        for (code, cat) in c {
            assert!(cat.contains_key("_meta.name"), "{code}: falta _meta.name");
            for (k, v) in cat {
                let Some(r) = reference.get(k) else {
                    panic!("{code}: la clave '{k}' no existe en en.json");
                };
                assert_eq!(vars(v), vars(r), "{code}: variables distintas en '{k}'");
            }
        }
    }

    #[test]
    fn resolves_regional_codes() {
        assert_eq!(resolve("es-MX"), "es");
        assert_eq!(resolve("en_US"), "en");
        assert_eq!(resolve("xx"), FALLBACK);
    }
}
