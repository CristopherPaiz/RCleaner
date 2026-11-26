//! Analizador de uso de disco.
//!
//! [`scan`] recorre una carpeta en paralelo (rayon) sin seguir enlaces
//! simbólicos ni uniones, y construye un [`DiskTree`]: un vector plano de
//! nodos con los hijos ordenados por tamaño. La UI no recibe el árbol entero
//! (pueden ser millones de nodos), sino vistas recortadas con [`DiskTree::view`]
//! y listas con [`DiskTree::children`].
//!
//! Los archivos de la nube sin copia local (OneDrive "a petición") cuentan 0
//! bytes: no ocupan disco.

use crate::protect::{self, SYSTEM_ATTRIBUTE};
use crate::{attributes, unix_secs};
use crp_core::{CancelToken, Env, path_within};
use crp_platform::files::is_cloud_placeholder;
use rayon::prelude::*;
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::Instant;

pub type NodeId = u32;
const NO_PARENT: NodeId = NodeId::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Dir,
    File,
    /// Enlace simbólico o unión: se muestra pero no se recorre (0 bytes).
    Link,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: Box<str>,
    pub size: u64,
    /// Archivos contenidos (1 para un archivo).
    pub items: u64,
    /// Última modificación (segundos Unix, 0 si se desconoce).
    pub modified: u64,
    pub parent: NodeId,
    pub kind: Kind,
    /// Tiene el atributo de sistema de Windows.
    pub system: bool,
    /// Ordenados por tamaño, de mayor a menor.
    pub children: Vec<NodeId>,
}

/// Contadores en vivo del escaneo (se leen desde otro hilo).
#[derive(Debug, Default)]
pub struct ScanProgress {
    pub files: AtomicU64,
    pub dirs: AtomicU64,
    pub bytes: AtomicU64,
    current: Mutex<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressSnapshot {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub current: String,
}

impl ScanProgress {
    pub fn snapshot(&self) -> ProgressSnapshot {
        ProgressSnapshot {
            files: self.files.load(Relaxed),
            dirs: self.dirs.load(Relaxed),
            bytes: self.bytes.load(Relaxed),
            current: self.current.lock().map(|c| c.clone()).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Carpetas que no se recorren (exclusiones del usuario).
    pub exclusions: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtStat {
    pub ext: String,
    pub bytes: u64,
    pub files: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub root: NodeId,
    pub path: PathBuf,
    pub bytes: u64,
    pub files: u64,
    pub dirs: u64,
    /// Carpetas que no se pudieron leer (permisos).
    pub unreadable: u64,
    /// Archivos de la nube sin copia local (cuentan 0 bytes).
    pub cloud_files: u64,
    pub elapsed_ms: u64,
    /// Tipos de archivo por espacio ocupado (los 40 mayores).
    pub extensions: Vec<ExtStat>,
}

/// Nodo recortado para dibujar el mapa radial.
#[derive(Debug, Clone, Serialize)]
pub struct ViewNode {
    /// `None` para el grupo "elementos más pequeños".
    pub id: Option<NodeId>,
    pub name: String,
    pub size: u64,
    pub items: u64,
    /// `dir`, `file`, `link` o `rest`.
    pub kind: &'static str,
    pub children: Vec<ViewNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub id: NodeId,
    pub name: String,
    pub size: u64,
    pub items: u64,
    pub kind: Kind,
    pub modified: u64,
    /// Por qué no se puede eliminar (archivo de Windows, carpeta base…).
    pub protected: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub id: NodeId,
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: u64,
    pub protected: Option<String>,
}

/// Sin extensión conocida (carpetas y piezas "resto").
pub const NO_EXT: u16 = u16::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TreemapKind {
    Dir,
    File,
    /// Elementos demasiado pequeños para verse, agrupados.
    Rest,
}

/// Árbol podado para el mapa de rectángulos, en columnas (JSON compacto).
/// La pieza 0 es la raíz; `parent[i]` apunta a una pieza anterior.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Treemap {
    pub parent: Vec<u32>,
    /// Id del nodo (`u32::MAX` en las piezas "resto").
    pub id: Vec<NodeId>,
    pub size: Vec<u64>,
    pub name: Vec<String>,
    pub kind: Vec<TreemapKind>,
    /// Índice en `exts`, o [`NO_EXT`].
    pub ext: Vec<u16>,
    pub exts: Vec<String>,
}

impl Treemap {
    fn push(
        &mut self,
        parent: u32,
        id: NodeId,
        size: u64,
        name: &str,
        kind: TreemapKind,
        ext: u16,
    ) -> u32 {
        self.parent.push(parent);
        self.id.push(id);
        self.size.push(size);
        self.name.push(name.to_owned());
        self.kind.push(kind);
        self.ext.push(ext);
        (self.parent.len() - 1) as u32
    }

    pub fn len(&self) -> usize {
        self.parent.len()
    }

    pub fn is_empty(&self) -> bool {
        self.parent.is_empty()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Crumb {
    pub id: NodeId,
    pub name: String,
}

#[derive(Debug)]
pub struct DiskTree {
    nodes: Vec<Node>,
    summary: Summary,
    env: Env,
}

// ── Escaneo ────────────────────────────────────────────────────────────────

struct Ctx<'a> {
    opts: &'a ScanOptions,
    cancel: &'a CancelToken,
    progress: &'a ScanProgress,
    unreadable: AtomicU64,
    cloud: AtomicU64,
}

/// Nodo provisional del recorrido recursivo (luego se aplana).
struct Tmp {
    name: Box<str>,
    size: u64,
    items: u64,
    modified: u64,
    kind: Kind,
    system: bool,
    children: Vec<Tmp>,
}

impl Tmp {
    fn leaf(name: Box<str>, size: u64, items: u64, modified: u64, kind: Kind) -> Self {
        Self {
            name,
            size,
            items,
            modified,
            kind,
            system: false,
            children: Vec::new(),
        }
    }

    fn system(mut self, attrs: u32) -> Self {
        self.system = attrs & SYSTEM_ATTRIBUTE != 0;
        self
    }
}

/// Analiza `root` (unidad o carpeta). Falla si no es una carpeta legible o se cancela.
pub fn scan(
    root: &Path,
    opts: &ScanOptions,
    cancel: &CancelToken,
    progress: &ScanProgress,
) -> io::Result<DiskTree> {
    let start = Instant::now();
    let md = std::fs::metadata(root)?;
    if !md.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            crp_core::t("error.not_a_folder"),
        ));
    }
    std::fs::read_dir(root)?; // sin permiso en la raíz → error claro en vez de árbol vacío

    let ctx = Ctx {
        opts,
        cancel,
        progress,
        unreadable: AtomicU64::new(0),
        cloud: AtomicU64::new(0),
    };
    // Pila grande: la recursión sigue la profundidad real de carpetas.
    let pool = rayon::ThreadPoolBuilder::new()
        .stack_size(16 << 20)
        .build()
        .map_err(io::Error::other)?;

    let name: Box<str> = root.display().to_string().into();
    let (nodes, exts) = pool.install(|| {
        let tmp = scan_dir(root, name, unix_secs(md.modified()), attributes(&md), &ctx);
        let mut nodes = Vec::new();
        let mut exts = HashMap::new();
        flatten(tmp, NO_PARENT, &mut nodes, &mut exts);
        (nodes, exts)
    });
    if cancel.is_cancelled() {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            crp_core::t("error.cancelled"),
        ));
    }

    let mut extensions: Vec<ExtStat> = exts
        .into_iter()
        .map(|(ext, (bytes, files))| ExtStat { ext, bytes, files })
        .collect();
    extensions.sort_unstable_by_key(|e| Reverse(e.bytes));
    extensions.truncate(40);

    let dirs = nodes.iter().filter(|n| n.kind == Kind::Dir).count() as u64;
    let summary = Summary {
        root: 0,
        path: root.to_path_buf(),
        bytes: nodes[0].size,
        files: nodes[0].items,
        dirs,
        unreadable: ctx.unreadable.load(Relaxed),
        cloud_files: ctx.cloud.load(Relaxed),
        elapsed_ms: start.elapsed().as_millis() as u64,
        extensions,
    };
    Ok(DiskTree {
        nodes,
        summary,
        env: Env::from_system(),
    })
}

fn scan_dir(path: &Path, name: Box<str>, modified: u64, attrs: u32, ctx: &Ctx) -> Tmp {
    let mut node = Tmp::leaf(name, 0, 0, modified, Kind::Dir).system(attrs);
    ctx.progress.dirs.fetch_add(1, Relaxed);
    let Ok(read) = std::fs::read_dir(path) else {
        ctx.unreadable.fetch_add(1, Relaxed);
        return node;
    };

    let mut subdirs = Vec::new();
    let (mut files, mut bytes) = (0u64, 0u64);
    for entry in read.flatten() {
        if ctx.cancel.is_cancelled() {
            break;
        }
        // En Windows estos metadatos vienen del propio listado (sin llamadas
        // extra) y no siguen enlaces.
        let Ok(md) = entry.metadata() else { continue };
        let name: Box<str> = entry.file_name().to_string_lossy().into();
        let modified = unix_secs(md.modified());
        let attrs = attributes(&md);
        let ft = md.file_type();
        if ft.is_symlink() {
            node.children
                .push(Tmp::leaf(name, 0, 0, modified, Kind::Link).system(attrs));
        } else if ft.is_dir() {
            let p = entry.path();
            if !ctx.opts.exclusions.iter().any(|ex| path_within(&p, ex)) {
                subdirs.push((p, name, modified, attrs));
            }
        } else {
            let size = if is_cloud_placeholder(attrs) {
                ctx.cloud.fetch_add(1, Relaxed);
                0
            } else {
                md.len()
            };
            files += 1;
            bytes += size;
            node.children
                .push(Tmp::leaf(name, size, 1, modified, Kind::File).system(attrs));
        }
    }
    ctx.progress.files.fetch_add(files, Relaxed);
    ctx.progress.bytes.fetch_add(bytes, Relaxed);
    if let Ok(mut current) = ctx.progress.current.try_lock() {
        *current = path.display().to_string();
    }

    let dirs: Vec<Tmp> = subdirs
        .into_par_iter()
        .map(|(p, n, m, a)| scan_dir(&p, n, m, a, ctx))
        .collect();
    node.children.extend(dirs);
    node.size = node.children.iter().map(|c| c.size).sum();
    node.items = node.children.iter().map(|c| c.items).sum();
    node
}

fn extension(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() && ext.len() <= 12 => {
            ext.to_lowercase()
        }
        _ => String::new(),
    }
}

fn flatten(
    tmp: Tmp,
    parent: NodeId,
    nodes: &mut Vec<Node>,
    exts: &mut HashMap<String, (u64, u64)>,
) -> NodeId {
    let id = nodes.len() as NodeId;
    if tmp.kind == Kind::File {
        let e = exts.entry(extension(&tmp.name)).or_default();
        e.0 += tmp.size;
        e.1 += 1;
    }
    nodes.push(Node {
        name: tmp.name,
        size: tmp.size,
        items: tmp.items,
        modified: tmp.modified,
        parent,
        kind: tmp.kind,
        system: tmp.system,
        children: Vec::new(),
    });
    let mut kids = tmp.children;
    kids.sort_unstable_by_key(|k| Reverse(k.size));
    let ids: Vec<NodeId> = kids
        .into_iter()
        .map(|k| flatten(k, id, nodes, exts))
        .collect();
    nodes[id as usize].children = ids;
    id
}

// ── Consultas ──────────────────────────────────────────────────────────────

impl DiskTree {
    pub fn summary(&self) -> &Summary {
        &self.summary
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id as usize)
    }

    /// Motivo por el que el nodo no se puede eliminar, si lo hay (ver [`protect`]).
    pub fn protection(&self, id: NodeId) -> Option<String> {
        let node = self.node(id)?;
        let path = self.path(id)?;
        protect::reason(&path, node.system, &self.env)
    }

    /// `true` si el nodo sigue colgando de la raíz (no se eliminó).
    pub fn is_attached(&self, id: NodeId) -> bool {
        let mut cur = id;
        loop {
            let Some(n) = self.node(cur) else {
                return false;
            };
            if cur == self.summary.root {
                return true;
            }
            let p = n.parent;
            if p == NO_PARENT || !self.nodes[p as usize].children.contains(&cur) {
                return false;
            }
            cur = p;
        }
    }

    /// Ruta completa del nodo.
    pub fn path(&self, id: NodeId) -> Option<PathBuf> {
        let mut names = Vec::new();
        let mut cur = id;
        while cur != self.summary.root {
            let n = self.node(cur)?;
            names.push(&*n.name);
            cur = n.parent;
        }
        let mut p = self.summary.path.clone();
        p.extend(names.into_iter().rev());
        Some(p)
    }

    /// Migas de pan desde la raíz hasta `id` (incluido).
    pub fn ancestors(&self, id: NodeId) -> Vec<Crumb> {
        let mut out = Vec::new();
        let mut cur = id;
        while let Some(n) = self.node(cur) {
            out.push(Crumb {
                id: cur,
                name: n.name.to_string(),
            });
            if cur == self.summary.root {
                break;
            }
            cur = n.parent;
        }
        out.reverse();
        out
    }

    /// Vista para el mapa radial: `depth` anillos bajo `id`. Los hijos que
    /// ocupan menos de `min_fraction` del nodo raíz de la vista se agrupan.
    pub fn view(&self, id: NodeId, depth: u32, min_fraction: f64) -> Option<ViewNode> {
        let n = self.node(id)?;
        let threshold = ((n.size as f64) * min_fraction).max(1.0) as u64;
        Some(self.view_rec(id, depth, threshold))
    }

    fn view_rec(&self, id: NodeId, depth: u32, threshold: u64) -> ViewNode {
        let n = &self.nodes[id as usize];
        let mut children = Vec::new();
        if depth > 0 {
            let (mut rest_size, mut rest_items, mut rest_count) = (0u64, 0u64, 0u32);
            for &c in &n.children {
                let cn = &self.nodes[c as usize];
                if cn.size >= threshold {
                    children.push(self.view_rec(c, depth - 1, threshold));
                } else {
                    rest_size += cn.size;
                    rest_items += cn.items;
                    rest_count += 1;
                }
            }
            if rest_size > 0 {
                children.push(ViewNode {
                    id: None,
                    name: crp_core::tn("disk.small_items", u64::from(rest_count), &[]),
                    size: rest_size,
                    items: rest_items,
                    kind: "rest",
                    children: Vec::new(),
                });
            }
        }
        ViewNode {
            id: Some(id),
            name: n.name.to_string(),
            size: n.size,
            items: n.items,
            kind: match n.kind {
                Kind::Dir => "dir",
                Kind::File => "file",
                Kind::Link => "link",
            },
            children,
        }
    }

    /// Datos para el mapa de rectángulos (estilo WinDirStat) bajo `id`.
    ///
    /// Recorre por niveles y baja hasta los archivos mientras quepan
    /// `max_items` piezas. Lo que ocupa menos de `min_fraction` del total se
    /// junta en una pieza "resto" por carpeta: no llegaría a verse.
    pub fn treemap(&self, id: NodeId, max_items: usize, min_fraction: f64) -> Option<Treemap> {
        let root = self.node(id)?;
        let threshold = ((root.size as f64) * min_fraction).max(1.0) as u64;
        let mut out = Treemap::default();
        let mut exts: HashMap<String, u16> = HashMap::new();
        out.push(
            NO_PARENT,
            id,
            root.size,
            &root.name,
            TreemapKind::Dir,
            NO_EXT,
        );
        let mut queue = std::collections::VecDeque::from([(0u32, id)]);
        while let Some((at, nid)) = queue.pop_front() {
            let n = &self.nodes[nid as usize];
            let mut rest = 0u64;
            for &c in &n.children {
                let cn = &self.nodes[c as usize];
                if cn.size == 0 {
                    continue;
                }
                if cn.size < threshold || out.len() >= max_items {
                    rest += cn.size;
                    continue;
                }
                let (kind, ext) = match cn.kind {
                    Kind::Dir => (TreemapKind::Dir, NO_EXT),
                    _ => {
                        let e = extension(&cn.name);
                        let next = exts.len() as u16;
                        let code = *exts.entry(e.clone()).or_insert_with(|| {
                            out.exts.push(e);
                            next
                        });
                        (TreemapKind::File, code)
                    }
                };
                let me = out.push(at, c, cn.size, &cn.name, kind, ext);
                if cn.kind == Kind::Dir && !cn.children.is_empty() {
                    queue.push_back((me, c));
                }
            }
            if rest > 0 {
                out.push(at, NO_PARENT, rest, "", TreemapKind::Rest, NO_EXT);
            }
        }
        Some(out)
    }

    /// Hijos directos (ya ordenados por tamaño), hasta `limit`.
    pub fn children(&self, id: NodeId, limit: usize) -> Vec<Entry> {
        let Some(n) = self.node(id) else {
            return Vec::new();
        };
        n.children
            .iter()
            .take(limit)
            .map(|&c| {
                let cn = &self.nodes[c as usize];
                Entry {
                    id: c,
                    name: cn.name.to_string(),
                    size: cn.size,
                    items: cn.items,
                    kind: cn.kind,
                    modified: cn.modified,
                    protected: self.protection(c),
                }
            })
            .collect()
    }

    /// Los `limit` archivos más grandes bajo `id`.
    pub fn largest_files(&self, id: NodeId, limit: usize) -> Vec<FileEntry> {
        let mut heap: BinaryHeap<Reverse<(u64, NodeId)>> = BinaryHeap::new();
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            let Some(n) = self.node(cur) else { continue };
            match n.kind {
                Kind::File => {
                    heap.push(Reverse((n.size, cur)));
                    if heap.len() > limit {
                        heap.pop();
                    }
                }
                Kind::Dir => {
                    // Si el directorio entero no supera al menor del top, se descarta.
                    let floor = if heap.len() == limit {
                        heap.peek().map_or(0, |r| r.0.0)
                    } else {
                        0
                    };
                    if n.size > floor || cur == id {
                        stack.extend(&n.children);
                    }
                }
                Kind::Link => {}
            }
        }
        let mut out: Vec<FileEntry> = heap
            .into_iter()
            .filter_map(|Reverse((size, fid))| {
                let path = self.path(fid)?;
                let node = &self.nodes[fid as usize];
                Some(FileEntry {
                    id: fid,
                    name: node.name.to_string(),
                    protected: protect::reason(&path, node.system, &self.env),
                    path,
                    size,
                    modified: node.modified,
                })
            })
            .collect();
        out.sort_unstable_by_key(|f| Reverse(f.size));
        out
    }

    /// Quita un nodo ya borrado del disco y descuenta su tamaño de los
    /// ancestros. Devuelve los bytes descontados.
    pub fn remove(&mut self, id: NodeId) -> Option<u64> {
        if id == self.summary.root || !self.is_attached(id) {
            return None;
        }
        let (size, items, parent) = {
            let n = &self.nodes[id as usize];
            (n.size, n.items, n.parent)
        };
        self.nodes[parent as usize].children.retain(|&c| c != id);
        let mut cur = parent;
        while cur != NO_PARENT {
            let n = &mut self.nodes[cur as usize];
            n.size = n.size.saturating_sub(size);
            n.items = n.items.saturating_sub(items);
            let p = n.parent;
            if p != NO_PARENT {
                self.resort(p); // mantener el orden por tamaño
            }
            cur = p;
        }
        self.summary.bytes = self.nodes[self.summary.root as usize].size;
        self.summary.files = self.nodes[self.summary.root as usize].items;
        Some(size)
    }

    fn resort(&mut self, id: NodeId) {
        let mut kids = std::mem::take(&mut self.nodes[id as usize].children);
        kids.sort_unstable_by_key(|&c| Reverse(self.nodes[c as usize].size));
        self.nodes[id as usize].children = kids;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sample() -> (tempfile::TempDir, DiskTree) {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        fs::create_dir_all(r.join("big/inner")).unwrap();
        fs::create_dir_all(r.join("small")).unwrap();
        fs::create_dir_all(r.join("skip")).unwrap();
        fs::write(r.join("big/a.bin"), vec![0u8; 5000]).unwrap();
        fs::write(r.join("big/inner/b.MP4"), vec![0u8; 3000]).unwrap();
        fs::write(r.join("small/c.txt"), vec![0u8; 10]).unwrap();
        fs::write(r.join("skip/huge.bin"), vec![0u8; 9000]).unwrap();
        fs::write(r.join("root.log"), vec![0u8; 100]).unwrap();
        let opts = ScanOptions {
            exclusions: vec![r.join("skip")],
        };
        let tree = scan(r, &opts, &CancelToken::new(), &ScanProgress::default()).unwrap();
        (dir, tree)
    }

    #[test]
    fn sizes_order_and_exclusions() {
        let (_d, t) = sample();
        let s = t.summary();
        assert_eq!(s.bytes, 8110);
        assert_eq!(s.files, 4);
        let kids = t.children(0, 10);
        let names: Vec<_> = kids.iter().map(|k| k.name.as_str()).collect();
        assert_eq!(names, ["big", "root.log", "small"]); // "skip" excluida
        assert_eq!(kids[0].size, 8000);
        assert_eq!(s.extensions[0].ext, "bin");
        assert!(s.extensions.iter().any(|e| e.ext == "mp4"));
    }

    #[test]
    fn paths_and_crumbs() {
        let (d, t) = sample();
        let big = t.children(0, 10)[0].id;
        let inner = t
            .children(big, 10)
            .into_iter()
            .find(|e| e.name == "inner")
            .unwrap()
            .id;
        assert_eq!(t.path(inner).unwrap(), d.path().join("big").join("inner"));
        let crumbs: Vec<_> = t.ancestors(inner).into_iter().map(|c| c.name).collect();
        assert_eq!(crumbs[1..], ["big".to_string(), "inner".to_string()]);
        let top = t.largest_files(0, 2);
        assert_eq!(top.iter().map(|f| f.size).collect::<Vec<_>>(), [5000, 3000]);
    }

    #[test]
    fn treemap_reaches_files_and_groups_tiny_ones() {
        let (_dir, tree) = sample();
        let tm = tree.treemap(0, 10_000, 0.0).unwrap();
        assert_eq!(tm.parent[0], NO_PARENT);
        assert_eq!(tm.size[0], tree.summary().bytes);
        assert!(tm.kind.contains(&TreemapKind::File));
        // Cada padre suma lo mismo que sus hijos.
        for i in 0..tm.len() {
            if tm.kind[i] == TreemapKind::Dir && tm.parent.contains(&(i as u32)) {
                let kids: u64 = (0..tm.len())
                    .filter(|&j| tm.parent[j] == i as u32)
                    .map(|j| tm.size[j])
                    .sum();
                assert_eq!(kids, tm.size[i]);
            }
        }
        let small = tree.treemap(0, 10_000, 0.9).unwrap();
        assert!(small.kind.contains(&TreemapKind::Rest));
    }

    #[test]
    fn view_groups_small_items() {
        let (_d, t) = sample();
        let v = t.view(0, 3, 0.05).unwrap();
        // root.log (100) y small (10) < 5 % de 8110 → un grupo "rest".
        assert_eq!(v.children.len(), 2);
        assert_eq!(v.children[1].kind, "rest");
        assert_eq!(v.children[1].size, 110);
    }

    #[test]
    fn remove_updates_ancestors() {
        let (_d, mut t) = sample();
        let big = t.children(0, 10)[0].id;
        let a = t
            .children(big, 10)
            .into_iter()
            .find(|e| e.name == "a.bin")
            .unwrap()
            .id;
        assert_eq!(t.remove(a), Some(5000));
        assert_eq!(t.node(big).unwrap().size, 3000);
        assert_eq!(t.summary().bytes, 3110);
        assert_eq!(t.summary().files, 3);
        assert!(!t.is_attached(a));
        assert_eq!(t.remove(a), None);
        assert_eq!(t.remove(0), None);
    }
}
