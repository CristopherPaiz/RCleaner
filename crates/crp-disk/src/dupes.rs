//! Buscador de archivos duplicados por contenido.
//!
//! 1. Lista archivos ≥ `min_size` (sin seguir enlaces, saltando carpetas de
//!    sistema/desarrollo y archivos de la nube sin copia local).
//! 2. Agrupa por tamaño exacto.
//! 3. Hash BLAKE3 de los primeros 16 KiB.
//! 4. Hash BLAKE3 completo de los que siguen coincidiendo.
//! 5. Descarta enlaces duros (misma identidad en disco: borrar uno no libera nada).

use crate::protect::{self, SYSTEM_ATTRIBUTE};
use crate::{attributes, unix_secs};
use crp_core::{CancelToken, Env, path_within};
use crp_platform::files::{identity, is_cloud_placeholder};
use rayon::prelude::*;
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::Instant;

/// Carpetas que por defecto no se recorren: duplicados ahí son normales
/// (dependencias, sistema) y borrarlos rompería cosas.
pub const DEFAULT_SKIP_DIRS: &[&str] = &[
    "$recycle.bin",
    "system volume information",
    "windows",
    "program files",
    "program files (x86)",
    "programdata",
    "appdata",
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "target",
    ".venv",
    "venv",
    "__pycache__",
    "site-packages",
    ".gradle",
    ".m2",
    ".cargo",
    ".rustup",
    ".nuget",
    ".npm",
    ".cache",
    "packagecache",
    "intermediates",
    "cache_data",
    "code cache",
    "gpucache",
];

const PARTIAL: u64 = 16 * 1024;

#[derive(Debug, Clone)]
pub struct DupeOptions {
    pub roots: Vec<PathBuf>,
    pub min_size: u64,
    /// Nombres de carpeta (minúsculas) que no se recorren.
    pub skip_dirs: Vec<String>,
    /// Rutas excluidas por el usuario.
    pub exclusions: Vec<PathBuf>,
}

impl DupeOptions {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self {
            roots,
            min_size: 1024 * 1024,
            skip_dirs: DEFAULT_SKIP_DIRS.iter().map(|s| (*s).to_owned()).collect(),
            exclusions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum DupeProgress {
    Listing { files: u64 },
    Hashing { done: u64, total: u64 },
}

#[derive(Debug, Clone, Serialize)]
pub struct DupeFile {
    pub id: u32,
    pub path: PathBuf,
    pub modified: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DupeGroup {
    pub size: u64,
    /// De más antiguo a más reciente.
    pub files: Vec<DupeFile>,
}

impl DupeGroup {
    /// Espacio que se recupera dejando una sola copia.
    pub fn wasted(&self) -> u64 {
        self.size * (self.files.len() as u64).saturating_sub(1)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DupeReport {
    /// Ordenados por espacio recuperable.
    pub groups: Vec<DupeGroup>,
    pub files_scanned: u64,
    pub wasted_bytes: u64,
    /// Carpetas o archivos que no se pudieron leer.
    pub unreadable: u64,
    /// Archivos de la nube omitidos (leerlos los descargaría).
    pub cloud_skipped: u64,
    pub elapsed_ms: u64,
}

impl DupeReport {
    /// Grupo y archivo con ese id.
    pub fn find(&self, id: u32) -> Option<(usize, &DupeFile)> {
        self.groups
            .iter()
            .enumerate()
            .find_map(|(g, grp)| grp.files.iter().find(|f| f.id == id).map(|f| (g, f)))
    }

    /// Quita archivos ya borrados y los grupos que se quedan con una copia.
    pub fn forget(&mut self, ids: &HashSet<u32>) {
        for g in &mut self.groups {
            g.files.retain(|f| !ids.contains(&f.id));
        }
        self.groups.retain(|g| g.files.len() > 1);
        self.wasted_bytes = self.groups.iter().map(DupeGroup::wasted).sum();
    }
}

struct Candidate {
    path: PathBuf,
    size: u64,
    modified: u64,
}

struct Walk<'a> {
    opts: &'a DupeOptions,
    cancel: &'a CancelToken,
    progress: &'a (dyn Fn(DupeProgress) + Sync),
    files: AtomicU64,
    unreadable: AtomicU64,
    cloud: AtomicU64,
    env: Env,
}

/// Busca duplicados. Devuelve un informe vacío si se cancela.
pub fn find(
    opts: &DupeOptions,
    cancel: &CancelToken,
    progress: &(dyn Fn(DupeProgress) + Sync),
) -> DupeReport {
    let start = Instant::now();
    let walk = Walk {
        opts,
        cancel,
        progress,
        files: AtomicU64::new(0),
        unreadable: AtomicU64::new(0),
        cloud: AtomicU64::new(0),
        env: Env::from_system(),
    };

    // Raíces anidadas: quedarse con las de más arriba para no contar dos veces.
    let mut roots: Vec<&PathBuf> = opts.roots.iter().collect();
    roots.sort_by_key(|r| r.as_os_str().len());
    let mut kept: Vec<&PathBuf> = Vec::new();
    for r in roots {
        if !kept.iter().any(|k| path_within(r, k)) {
            kept.push(r);
        }
    }
    let candidates: Vec<Candidate> = kept
        .par_iter()
        .flat_map(|r| {
            let mut out = Vec::new();
            walk_dir(r, &walk, &mut out);
            out
        })
        .collect();
    let files_scanned = walk.files.load(Relaxed);
    progress(DupeProgress::Listing {
        files: files_scanned,
    });

    // Por tamaño.
    let mut by_size: HashMap<u64, Vec<usize>> = HashMap::new();
    for (i, c) in candidates.iter().enumerate() {
        by_size.entry(c.size).or_default().push(i);
    }
    let sized: Vec<usize> = by_size
        .into_values()
        .filter(|v| v.len() > 1)
        .flatten()
        .collect();
    let full_needed = sized
        .iter()
        .filter(|&&i| candidates[i].size > PARTIAL)
        .count() as u64;
    let total = sized.len() as u64 + full_needed;
    let done = AtomicU64::new(0);
    let tick = || {
        let d = done.fetch_add(1, Relaxed) + 1;
        if d % 32 == 0 || d == total {
            progress(DupeProgress::Hashing { done: d, total });
        }
    };

    // Hash parcial → hash completo.
    let partial = hash_and_group(&candidates, &sized, Some(PARTIAL), cancel, &walk, &tick);
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut need_full: Vec<usize> = Vec::new();
    for g in partial {
        if candidates[g[0]].size <= PARTIAL {
            groups.push(g); // el hash parcial ya cubrió el archivo entero
        } else {
            need_full.extend(g);
        }
    }
    groups.extend(hash_and_group(
        &candidates,
        &need_full,
        None,
        cancel,
        &walk,
        &tick,
    ));
    if cancel.is_cancelled() {
        return DupeReport::default();
    }

    // Enlaces duros fuera, ids estables y orden por espacio recuperable.
    let mut next_id = 0u32;
    let mut out: Vec<DupeGroup> = groups
        .into_iter()
        .filter_map(|g| {
            let mut seen = HashSet::new();
            let mut files: Vec<&Candidate> = g
                .into_iter()
                .map(|i| &candidates[i])
                .filter(|c| identity(&c.path).is_none_or(|id| seen.insert((id.volume, id.index))))
                .collect();
            if files.len() < 2 {
                return None;
            }
            files.sort_by_key(|c| c.modified);
            Some(DupeGroup {
                size: files[0].size,
                files: files
                    .into_iter()
                    .map(|c| {
                        next_id += 1;
                        DupeFile {
                            id: next_id,
                            path: c.path.clone(),
                            modified: c.modified,
                        }
                    })
                    .collect(),
            })
        })
        .collect();
    out.sort_by_key(|g| Reverse(g.wasted()));

    DupeReport {
        wasted_bytes: out.iter().map(DupeGroup::wasted).sum(),
        groups: out,
        files_scanned,
        unreadable: walk.unreadable.load(Relaxed),
        cloud_skipped: walk.cloud.load(Relaxed),
        elapsed_ms: start.elapsed().as_millis() as u64,
    }
}

fn walk_dir(dir: &Path, w: &Walk, out: &mut Vec<Candidate>) {
    if w.cancel.is_cancelled() {
        return;
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        w.unreadable.fetch_add(1, Relaxed);
        return;
    };
    let mut subdirs = Vec::new();
    for entry in read.flatten() {
        let Ok(md) = entry.metadata() else { continue };
        let ft = md.file_type();
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ft.is_dir() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if !w.opts.skip_dirs.contains(&name)
                && !w.opts.exclusions.iter().any(|ex| path_within(&path, ex))
            {
                subdirs.push(path);
            }
            continue;
        }
        let n = w.files.fetch_add(1, Relaxed) + 1;
        if n % 2000 == 0 {
            (w.progress)(DupeProgress::Listing { files: n });
        }
        let attrs = attributes(&md);
        if is_cloud_placeholder(attrs) {
            w.cloud.fetch_add(1, Relaxed);
            continue;
        }
        // Archivos de Windows (pagefile, registro, sistema…): ni se ofrecen.
        if md.len() >= w.opts.min_size.max(1)
            && protect::reason(&path, attrs & SYSTEM_ATTRIBUTE != 0, &w.env).is_none()
        {
            out.push(Candidate {
                path,
                size: md.len(),
                modified: unix_secs(md.modified()),
            });
        }
    }
    let nested: Vec<Candidate> = subdirs
        .into_par_iter()
        .flat_map(|d| {
            let mut v = Vec::new();
            walk_dir(&d, w, &mut v);
            v
        })
        .collect();
    out.extend(nested);
}

/// Hashea `indices` y devuelve los grupos con el mismo (tamaño, hash).
fn hash_and_group(
    candidates: &[Candidate],
    indices: &[usize],
    limit: Option<u64>,
    cancel: &CancelToken,
    walk: &Walk,
    tick: &(dyn Fn() + Sync),
) -> Vec<Vec<usize>> {
    let hashed: Vec<(usize, [u8; 32])> = indices
        .par_iter()
        .filter_map(|&i| {
            if cancel.is_cancelled() {
                return None;
            }
            let h = hash_file(&candidates[i].path, limit);
            tick();
            match h {
                Ok(h) => Some((i, h)),
                Err(_) => {
                    walk.unreadable.fetch_add(1, Relaxed);
                    None
                }
            }
        })
        .collect();
    let mut map: HashMap<(u64, [u8; 32]), Vec<usize>> = HashMap::new();
    for (i, h) in hashed {
        map.entry((candidates[i].size, h)).or_default().push(i);
    }
    map.into_values().filter(|g| g.len() > 1).collect()
}

fn hash_file(path: &Path, limit: Option<u64>) -> io::Result<[u8; 32]> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut remaining = limit.unwrap_or(u64::MAX);
    while remaining > 0 {
        let want = (buf.len() as u64).min(remaining) as usize;
        let n = file.read(&mut buf[..want])?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        remaining -= n as u64;
    }
    Ok(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn opts(root: &Path) -> DupeOptions {
        DupeOptions {
            min_size: 1,
            ..DupeOptions::new(vec![root.to_path_buf()])
        }
    }

    #[test]
    fn finds_groups_by_content() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        fs::create_dir_all(r.join("a/b")).unwrap();
        let big: Vec<u8> = (0..40_000u32).map(|i| (i % 251) as u8).collect();
        let mut big2 = big.clone();
        *big2.last_mut().unwrap() ^= 1; // mismo tamaño y prefijo, distinto final
        fs::write(r.join("x.bin"), &big).unwrap();
        fs::write(r.join("a/x copy.bin"), &big).unwrap();
        fs::write(r.join("a/b/x2.bin"), &big).unwrap();
        fs::write(r.join("a/near.bin"), &big2).unwrap();
        fs::write(r.join("s1.txt"), b"hola mundo").unwrap();
        fs::write(r.join("a/s2.txt"), b"hola mundo").unwrap();
        fs::write(r.join("a/s3.txt"), b"hola mundx").unwrap();

        let rep = find(&opts(r), &CancelToken::new(), &|_| {});
        assert_eq!(rep.groups.len(), 2);
        assert_eq!(rep.groups[0].files.len(), 3);
        assert_eq!(rep.groups[0].size, 40_000);
        assert_eq!(rep.groups[1].files.len(), 2);
        assert_eq!(rep.wasted_bytes, 80_000 + 10);
        assert_eq!(rep.files_scanned, 7);
    }

    #[test]
    fn skips_dirs_min_size_and_nested_roots() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        fs::create_dir_all(r.join("node_modules")).unwrap();
        fs::create_dir_all(r.join("sub")).unwrap();
        fs::write(r.join("a.txt"), b"igual").unwrap();
        fs::write(r.join("node_modules/a.txt"), b"igual").unwrap();
        fs::write(r.join("sub/a.txt"), b"igual").unwrap();
        let mut o = opts(r);
        o.roots.push(r.join("sub")); // anidada: no debe contar doble
        let rep = find(&o, &CancelToken::new(), &|_| {});
        assert_eq!(rep.groups.len(), 1);
        assert_eq!(rep.groups[0].files.len(), 2);

        o.min_size = 100;
        assert!(find(&o, &CancelToken::new(), &|_| {}).groups.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn hard_links_are_not_duplicates() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        fs::write(r.join("a.bin"), b"contenido").unwrap();
        fs::hard_link(r.join("a.bin"), r.join("b.bin")).unwrap();
        assert!(
            find(&opts(r), &CancelToken::new(), &|_| {})
                .groups
                .is_empty()
        );
    }

    #[test]
    fn forget_drops_singletons() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        fs::write(r.join("a"), b"zz").unwrap();
        fs::write(r.join("b"), b"zz").unwrap();
        let mut rep = find(&opts(r), &CancelToken::new(), &|_| {});
        let id = rep.groups[0].files[0].id;
        assert!(rep.find(id).is_some());
        rep.forget(&HashSet::from([id]));
        assert!(rep.groups.is_empty());
        assert_eq!(rep.wasted_bytes, 0);
    }
}
