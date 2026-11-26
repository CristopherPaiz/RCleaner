//! Medición y borrado de archivos. Nunca sigue symlinks ni junctions.

use crate::context::{CancelToken, path_within};
use crate::model::EntryFilter;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

/// Máximo de mensajes de error guardados por operación.
const MAX_ERRORS: usize = 20;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub bytes: u64,
    pub items: u64,
}

impl std::ops::AddAssign for Stats {
    fn add_assign(&mut self, rhs: Self) {
        self.bytes += rhs.bytes;
        self.items += rhs.items;
    }
}

#[derive(Debug, Default, Clone)]
pub struct DeleteStats {
    pub freed: u64,
    pub removed: u64,
    pub skipped: u64,
    pub errors: Vec<String>,
}

impl DeleteStats {
    fn error(&mut self, path: &Path, err: impl std::fmt::Display) {
        self.skipped += 1;
        if self.errors.len() < MAX_ERRORS {
            self.errors.push(format!("{}: {err}", path.display()));
        }
    }

    pub fn merge(&mut self, other: DeleteStats) {
        self.freed += other.freed;
        self.removed += other.removed;
        self.skipped += other.skipped;
        let room = MAX_ERRORS.saturating_sub(self.errors.len());
        self.errors.extend(other.errors.into_iter().take(room));
    }
}

/// Filtro compilado.
struct Matcher {
    patterns: Vec<glob::Pattern>,
    min_age: Option<Duration>,
    now: SystemTime,
}

impl Matcher {
    fn new(filter: &EntryFilter) -> Self {
        Self {
            patterns: filter
                .names
                .iter()
                .filter_map(|p| glob::Pattern::new(p).ok())
                .collect(),
            min_age: filter.min_age_secs.map(Duration::from_secs),
            now: SystemTime::now(),
        }
    }

    fn old_enough(&self, meta: &fs::Metadata) -> bool {
        let Some(min) = self.min_age else { return true };
        // Sin fecha fiable → se considera reciente (conservador).
        meta.modified()
            .ok()
            .and_then(|m| self.now.duration_since(m).ok())
            .is_some_and(|age| age >= min)
    }

    fn name_matches(&self, name: &OsStr) -> bool {
        if self.patterns.is_empty() {
            return true;
        }
        let opts = glob::MatchOptions {
            case_sensitive: false,
            ..Default::default()
        };
        let name = name.to_string_lossy();
        self.patterns.iter().any(|p| p.matches_with(&name, opts))
    }

    fn file_matches(&self, name: &OsStr, meta: &fs::Metadata) -> bool {
        self.name_matches(name) && self.old_enough(meta)
    }
}

fn walker(root: &Path, filter: &EntryFilter) -> WalkDir {
    let w = WalkDir::new(root).follow_links(false).min_depth(1);
    match filter.max_depth {
        Some(d) => w.max_depth(d),
        None => w,
    }
}

/// Recorrido en pre-orden que no entra en las rutas excluidas.
///
/// Ojo: no combinar `filter_entry` con `contents_first(true)`: en walkdir eso
/// salta los hermanos que siguen a una carpeta filtrada.
fn entries(
    root: &Path,
    filter: &EntryFilter,
) -> impl Iterator<Item = walkdir::Result<walkdir::DirEntry>> {
    let exclude = filter.exclude.clone();
    walker(root, filter)
        .into_iter()
        .filter_entry(move |e| !exclude.iter().any(|x| path_within(e.path(), x)))
}

/// Suma los archivos bajo `root` que pasan el filtro.
pub fn measure_contents(root: &Path, filter: &EntryFilter, cancel: &CancelToken) -> Stats {
    let matcher = Matcher::new(filter);
    let mut stats = Stats::default();
    for entry in entries(root, filter).filter_map(Result::ok) {
        if cancel.is_cancelled() {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if matcher.file_matches(entry.file_name(), &meta) {
            stats.bytes += meta.len();
            stats.items += 1;
        }
    }
    stats
}

/// Tamaño de un archivo o árbol completo. Un enlace cuenta como 1 elemento de 0 bytes.
pub fn measure_path(path: &Path, cancel: &CancelToken) -> Stats {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_file() => Stats {
            bytes: m.len(),
            items: 1,
        },
        Ok(m) if m.is_dir() => {
            let mut s = measure_contents(path, &EntryFilter::default(), cancel);
            s.items += 1;
            s
        }
        Ok(_) => Stats { bytes: 0, items: 1 },
        Err(_) => Stats::default(),
    }
}

/// Borra un archivo; si es de sólo lectura, quita el atributo y reintenta.
#[allow(clippy::permissions_set_readonly_false)]
fn remove_file_force(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            let meta = fs::symlink_metadata(path)?;
            let mut perms = meta.permissions();
            if !perms.readonly() {
                return Err(e);
            }
            perms.set_readonly(false);
            fs::set_permissions(path, perms)?;
            fs::remove_file(path)
        }
        other => other,
    }
}

/// Elimina un symlink/junction sin tocar su destino.
fn remove_link(path: &Path) -> io::Result<()> {
    fs::remove_file(path).or_else(|_| fs::remove_dir(path))
}

/// Borra el contenido de `root` que pasa el filtro. `root` se conserva.
///
/// - Archivos en uso o sin permisos se omiten (cuentan en `skipped`).
/// - Si el filtro no restringe por nombre, también se eliminan las subcarpetas
///   que queden vacías (y que cumplan la antigüedad mínima).
/// - Los enlaces se eliminan como enlace, jamás se recorre su destino.
pub fn delete_contents(root: &Path, filter: &EntryFilter, cancel: &CancelToken) -> DeleteStats {
    let matcher = Matcher::new(filter);
    let prune_dirs = filter.names.is_empty();
    let mut stats = DeleteStats::default();
    // Carpetas candidatas a quitar si quedan vacías (en orden de visita).
    let mut dirs = Vec::new();

    for entry in entries(root, filter) {
        if cancel.is_cancelled() {
            break;
        }
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                let p = e
                    .path()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| root.to_path_buf());
                stats.error(&p, e);
                continue;
            }
        };
        let path = entry.path();
        let ft = entry.file_type();

        if ft.is_symlink() {
            if prune_dirs && remove_link(path).is_ok() {
                stats.removed += 1;
            }
            continue;
        }

        let Ok(meta) = entry.metadata() else { continue };
        if ft.is_file() {
            if !matcher.file_matches(entry.file_name(), &meta) {
                continue;
            }
            cancel.set_current(path);
            match remove_file_force(path) {
                Ok(()) => {
                    stats.freed += meta.len();
                    stats.removed += 1;
                }
                Err(e) => stats.error(path, e),
            }
        } else if ft.is_dir() && prune_dirs && matcher.old_enough(&meta) {
            dirs.push(path.to_path_buf());
        }
    }
    // De la más profunda a la menos; falla si no está vacía: correcto, se ignora.
    for dir in dirs.iter().rev() {
        let _ = fs::remove_dir(dir);
    }
    stats
}

/// Borra un archivo o una carpeta completa (incluida la propia carpeta).
pub fn delete_path(path: &Path, cancel: &CancelToken) -> DeleteStats {
    let mut stats = DeleteStats::default();
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return stats,
        Err(e) => {
            stats.error(path, e);
            return stats;
        }
    };
    cancel.set_current(path);
    if meta.is_file() {
        match remove_file_force(path) {
            Ok(()) => {
                stats.freed = meta.len();
                stats.removed = 1;
            }
            Err(e) => stats.error(path, e),
        }
    } else if meta.is_dir() {
        stats = delete_contents(path, &EntryFilter::default(), cancel);
        if !cancel.is_cancelled() {
            match fs::remove_dir(path) {
                Ok(()) => stats.removed += 1,
                Err(e) if stats.skipped == 0 => stats.error(path, e),
                Err(_) => {}
            }
        }
    } else if remove_link(path).is_ok() {
        stats.removed = 1;
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    fn write(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        File::create(path)
            .unwrap()
            .write_all(&vec![0u8; bytes])
            .unwrap();
    }

    #[test]
    fn measure_and_delete_everything() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("cache");
        write(&root.join("a.bin"), 100);
        write(&root.join(r"sub\b.bin"), 50);
        let cancel = CancelToken::new();

        let s = measure_contents(&root, &EntryFilter::default(), &cancel);
        assert_eq!(
            s,
            Stats {
                bytes: 150,
                items: 2
            }
        );

        let d = delete_contents(&root, &EntryFilter::default(), &cancel);
        assert_eq!((d.freed, d.removed, d.skipped), (150, 2, 0));
        assert!(root.exists(), "la raíz se conserva");
        assert!(!root.join("sub").exists(), "subcarpeta vacía eliminada");
    }

    #[test]
    fn name_filter_only_touches_matches() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join("thumbcache_256.db"), 10);
        write(&root.join("THUMBCACHE_idx.DB"), 5);
        write(&root.join("keep.txt"), 7);
        let f = EntryFilter {
            names: vec!["thumbcache_*.db".into()],
            ..Default::default()
        };
        let cancel = CancelToken::new();

        assert_eq!(measure_contents(root, &f, &cancel).bytes, 15);
        delete_contents(root, &f, &cancel);
        assert!(root.join("keep.txt").exists());
        assert!(!root.join("thumbcache_256.db").exists());
    }

    #[test]
    fn min_age_skips_fresh_files() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("fresh.tmp"), 10);
        let f = EntryFilter {
            min_age_secs: Some(3600),
            ..Default::default()
        };
        let cancel = CancelToken::new();
        assert_eq!(measure_contents(dir.path(), &f, &cancel).items, 0);
        delete_contents(dir.path(), &f, &cancel);
        assert!(dir.path().join("fresh.tmp").exists());
    }

    #[test]
    fn exclusions_are_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join(r"keep\a.bin"), 10);
        write(&root.join("b.bin"), 5);
        let f = EntryFilter {
            exclude: vec![root.join("KEEP")],
            ..Default::default()
        };
        let cancel = CancelToken::new();
        assert_eq!(measure_contents(root, &f, &cancel).bytes, 5);
        delete_contents(root, &f, &cancel);
        assert!(root.join(r"keep\a.bin").exists());
        assert!(!root.join("b.bin").exists());
    }

    #[test]
    fn readonly_files_are_removed() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("ro.bin");
        write(&p, 3);
        let mut perms = fs::metadata(&p).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&p, perms).unwrap();
        let d = delete_path(&p, &CancelToken::new());
        assert_eq!((d.freed, d.removed), (3, 1));
        assert!(!p.exists());
    }

    #[test]
    fn delete_path_removes_tree() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("models--org--name");
        write(&model.join(r"blobs\x"), 42);
        let d = delete_path(&model, &CancelToken::new());
        assert_eq!(d.freed, 42);
        assert!(!model.exists());
    }

    #[cfg(windows)]
    #[test]
    fn never_follows_symlinked_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside");
        write(&outside.join("precious.txt"), 9);
        let root = dir.path().join("cache");
        fs::create_dir_all(&root).unwrap();
        // Crear symlinks puede requerir modo desarrollador; si no se puede, se omite.
        if std::os::windows::fs::symlink_dir(&outside, root.join("link")).is_err() {
            return;
        }
        let cancel = CancelToken::new();
        assert_eq!(
            measure_contents(&root, &EntryFilter::default(), &cancel).bytes,
            0
        );
        delete_contents(&root, &EntryFilter::default(), &cancel);
        assert!(
            outside.join("precious.txt").exists(),
            "el destino del enlace sigue intacto"
        );
    }
}
