//! Iconos de Windows para la UI (archivos, carpetas, unidades y apps).
//!
//! La UI pide claves; las rutas se resuelven aquí a partir del último análisis
//! (igual que en el resto de comandos, no se aceptan rutas de la UI):
//!
//! | Clave | Icono |
//! |---|---|
//! | `ext:zip` | tipo de archivo (sin tocar el disco) |
//! | `folder` | carpeta |
//! | `drive:C:\` | unidad |
//! | `node:123` | archivo del análisis de Espacio (`.exe`, `.lnk`…) |
//! | `dupe:45` | archivo de Duplicados |
//! | `app:chrome.exe` | app instalada (según el registro de Windows) |

use crate::state::{AppState, lock};
use base64::Engine;
use crp_platform::drives;
use crp_platform::icons::{IconQuery, find_app, png};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tauri::State;

/// Máximo de iconos por llamada.
const BATCH: usize = 300;

/// Iconos ya convertidos (`None` = Windows no dio ninguno).
fn cache() -> &'static Mutex<HashMap<String, Option<String>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Traduce la clave de la UI a una consulta y a su clave de caché.
fn resolve(key: &str, state: &AppState) -> Option<(String, IconQuery)> {
    let (kind, arg) = key.split_once(':').unwrap_or((key, ""));
    match kind {
        "folder" => Some(("folder".into(), IconQuery::Folder)),
        "ext" => {
            let ext = arg.to_ascii_lowercase();
            let valid = ext.len() <= 16
                && ext
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            valid.then(|| (format!("ext:{ext}"), IconQuery::Extension(ext)))
        }
        "drive" => {
            let root = drives::fixed_drives()
                .into_iter()
                .find(|d| d.root.eq_ignore_ascii_case(arg))?;
            Some((
                format!("drive:{}", root.root),
                IconQuery::Drive(PathBuf::from(root.root)),
            ))
        }
        "node" => {
            let id = arg.parse().ok()?;
            let path = lock(&state.disk).as_ref()?.path(id)?;
            Some((format!("path:{}", path.display()), IconQuery::File(path)))
        }
        "dupe" => {
            let id = arg.parse().ok()?;
            let path = lock(&state.dupes).as_ref()?.find(id)?.1.path.clone();
            Some((format!("path:{}", path.display()), IconQuery::File(path)))
        }
        "app" => {
            let exe = find_app(arg)?;
            Some((format!("path:{}", exe.display()), IconQuery::File(exe)))
        }
        _ => None,
    }
}

/// Iconos como `data:image/png;base64,…` por clave. Las claves sin icono no
/// aparecen en la respuesta: la UI se queda con su icono propio.
#[tauri::command]
pub async fn icons(
    state: State<'_, AppState>,
    keys: Vec<String>,
) -> Result<HashMap<String, String>, String> {
    let wanted: Vec<(String, String, IconQuery)> = keys
        .into_iter()
        .take(BATCH)
        .filter_map(|k| resolve(&k, &state).map(|(ck, q)| (k, ck, q)))
        .collect();
    tauri::async_runtime::spawn_blocking(move || {
        let mut out = HashMap::new();
        for (key, cache_key, query) in wanted {
            let hit = lock(cache()).get(&cache_key).cloned();
            let url = match hit {
                Some(url) => url,
                None => {
                    let url = png(&query).map(|bytes| {
                        format!(
                            "data:image/png;base64,{}",
                            base64::engine::general_purpose::STANDARD.encode(bytes)
                        )
                    });
                    lock(cache()).insert(cache_key, url.clone());
                    url
                }
            };
            if let Some(url) = url {
                out.insert(key, url);
            }
        }
        out
    })
    .await
    .map_err(|e| e.to_string())
}
