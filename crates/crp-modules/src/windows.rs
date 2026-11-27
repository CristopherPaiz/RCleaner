//! Restos grandes de Windows: la instalación anterior (`Windows.old`) y las
//! actualizaciones sustituidas del almacén de componentes (`WinSxS`).
//!
//! Ambos necesitan administrador y no se premarcan nunca: después ya no se
//! puede volver a la versión anterior de Windows ni desinstalar las
//! actualizaciones ya instaladas.

use crp_core::Category;
use crp_core::context::CancelToken;
use crp_core::{
    Action, CleanOutcome, CleanerModule, Engine, Env, Error, Finding, ModuleMeta, Result, Risk,
    ScanContext, exec, fsutil, t,
};
use std::path::{Path, PathBuf};

/// Carpetas de la raíz del disco del sistema que deja una actualización.
const PREVIOUS: &[(&str, &str)] = &[
    ("Windows.old", "windows_old"),
    ("$Windows.~BT", "upgrade"),
    ("$Windows.~WS", "upgrade_media"),
];

/// SID de Administradores (no depende del idioma de Windows).
const ADMINISTRATORS: &str = "*S-1-5-32-544";

pub fn register(e: &mut Engine) {
    e.register(PreviousWindows {
        meta: ModuleMeta::new("system.windows_old", Category::System)
            .risk(Risk::Moderate)
            .admin(),
    });
    e.register(ComponentStore {
        meta: ModuleMeta::new("system.component_store", Category::System)
            .risk(Risk::Moderate)
            .admin(),
    });
}

/// Raíz del disco donde está Windows (`C:\`).
fn system_root(env: &Env) -> Option<PathBuf> {
    env.windows_dir.parent().map(PathBuf::from)
}

/// Instalaciones anteriores presentes: (nombre de carpeta, clave, ruta).
pub fn previous_installs(env: &Env) -> Vec<(&'static str, &'static str, PathBuf)> {
    let Some(root) = system_root(env) else {
        return Vec::new();
    };
    PREVIOUS
        .iter()
        .map(|(dir, key)| (*dir, *key, root.join(dir)))
        .filter(|(_, _, p)| p.is_dir())
        .collect()
}

/// Bytes que ocupan las instalaciones anteriores (sin administrador, al
/// menos lo que se puede leer).
pub fn previous_installs_bytes(env: &Env) -> u64 {
    let cancel = CancelToken::default();
    previous_installs(env)
        .iter()
        .map(|(_, _, p)| fsutil::measure_path(p, &cancel).bytes)
        .sum()
}

/// Carpetas personales que pueden quedar en `Windows.old\Users\<nombre>`
/// tras una instalación limpia o si algo no se migró.
const USER_FOLDERS: &[&str] = &[
    "Desktop",
    "Documents",
    "Downloads",
    "Pictures",
    "Music",
    "Videos",
];

/// ¿Quedan archivos personales de la instalación anterior?
fn has_user_files(windows_old: &Path) -> bool {
    let Ok(users) = std::fs::read_dir(windows_old.join("Users")) else {
        return false;
    };
    users.flatten().any(|u| {
        USER_FOLDERS.iter().any(|f| {
            std::fs::read_dir(u.path().join(f)).is_ok_and(|mut d| {
                d.any(|e| e.is_ok_and(|e| !e.file_name().eq_ignore_ascii_case("desktop.ini")))
            })
        })
    })
}

struct PreviousWindows {
    meta: ModuleMeta,
}

impl CleanerModule for PreviousWindows {
    fn meta(&self) -> &ModuleMeta {
        &self.meta
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<Finding>> {
        let mut out = Vec::new();
        for (dir, key, path) in previous_installs(&ctx.env) {
            // Sin administrador parte del árbol no se puede leer: el tamaño
            // sale por debajo del real, pero sirve para avisar.
            let stats = fsutil::measure_path(&path, &ctx.cancel);
            if stats.items == 0 {
                continue;
            }
            let warning = (key == "windows_old" && has_user_files(&path))
                .then(|| t("module.system.windows_old.user_files"));
            out.push(Finding {
                id: format!("{}:{key}", self.meta.id),
                module_id: self.meta.id.clone(),
                category: self.meta.category,
                label: t(&format!("module.system.windows_old.target.{key}")),
                detail: Some(t("module.system.windows_old.detail")),
                path: Some(path),
                bytes: stats.bytes,
                items: stats.items,
                risk: self.meta.risk,
                requires_admin: true,
                selected: false,
                age_days: None,
                warning,
                action: Action::Custom { key: dir.into() },
            });
        }
        Ok(out)
    }

    /// Windows protege estos archivos con permisos de TrustedInstaller: se le
    /// da la propiedad a Administradores y control total, y luego se borra.
    fn clean_custom(&self, finding: &Finding, ctx: &ScanContext) -> Result<CleanOutcome> {
        let Action::Custom { key } = &finding.action else {
            return Err(Error::Other("bad action".into()));
        };
        // Sólo las carpetas conocidas de la raíz del disco del sistema.
        let path = previous_installs(&ctx.env)
            .into_iter()
            .find(|(dir, _, _)| dir == key)
            .map(|(_, _, p)| p)
            .ok_or_else(|| Error::Other(format!("{key}: not found")))?;
        let p = path.to_string_lossy().into_owned();
        let icacls = |args: &[&str]| {
            let mut all = vec![p.clone()];
            all.extend(args.iter().map(|a| (*a).to_owned()));
            // `/C` sigue aunque algún archivo falle; lo que quede lo cuenta
            // el borrado como omitido.
            let _ = exec::run_command(&ctx.env, "icacls", &all);
        };
        icacls(&["/setowner", ADMINISTRATORS, "/T", "/C", "/Q"]);
        icacls(&["/grant", &format!("{ADMINISTRATORS}:F"), "/T", "/C", "/Q"]);
        let stats = fsutil::delete_path(&path, &ctx.cancel);
        Ok(CleanOutcome {
            freed_bytes: stats.freed,
            removed_items: stats.removed,
            skipped_items: stats.skipped,
            errors: stats.errors,
            ..Default::default()
        })
    }
}

/// Almacén de componentes (`WinSxS`): DISM quita las versiones sustituidas
/// por actualizaciones posteriores. Es lo mismo que hace Windows por su
/// cuenta en una tarea programada, sin esperar a que toque.
struct ComponentStore {
    meta: ModuleMeta,
}

/// Lo que dice `DISM /AnalyzeComponentStore` (con `/English`).
#[derive(Debug, Default, PartialEq)]
pub struct StoreAnalysis {
    /// Copias de seguridad y características deshabilitadas + caché.
    pub reclaimable_bytes: u64,
    pub packages: u64,
    pub recommended: bool,
}

/// `1.86 GB` → bytes.
fn parse_size(value: &str) -> Option<u64> {
    let mut parts = value.split_whitespace();
    let n: f64 = parts.next()?.replace(',', ".").parse().ok()?;
    let mult = match parts.next()?.to_ascii_uppercase().as_str() {
        "BYTES" | "B" => 1.0,
        "KB" => 1024.0,
        "MB" => 1024.0_f64.powi(2),
        "GB" => 1024.0_f64.powi(3),
        "TB" => 1024.0_f64.powi(4),
        _ => return None,
    };
    Some((n * mult) as u64)
}

pub fn parse_analysis(text: &str) -> StoreAnalysis {
    let mut a = StoreAnalysis::default();
    for line in text.lines() {
        let Some((label, value)) = line.split_once(':') else {
            continue;
        };
        let (label, value) = (label.trim().to_ascii_lowercase(), value.trim());
        if label.starts_with("backups and disabled features")
            || label.starts_with("cache and temporary data")
        {
            a.reclaimable_bytes += parse_size(value).unwrap_or(0);
        } else if label.starts_with("number of reclaimable packages") {
            a.packages = value.parse().unwrap_or(0);
        } else if label.starts_with("component store cleanup recommended") {
            a.recommended = value.eq_ignore_ascii_case("yes");
        }
    }
    a
}

const DISM_ARGS: [&str; 3] = ["/Online", "/Cleanup-Image", "/English"];

impl CleanerModule for ComponentStore {
    fn meta(&self) -> &ModuleMeta {
        &self.meta
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<Finding>> {
        // DISM sólo funciona como administrador, y el análisis tarda varios
        // minutos: sólo si el usuario lo activó en Ajustes.
        if !ctx.is_admin || !ctx.component_store {
            return Ok(Vec::new());
        }
        let mut args: Vec<String> = DISM_ARGS.iter().map(|s| (*s).to_owned()).collect();
        args.push("/AnalyzeComponentStore".into());
        let a = parse_analysis(&exec::command_output(&ctx.env, "dism", &args)?);
        if !(a.recommended || a.packages > 0) || a.reclaimable_bytes == 0 {
            return Ok(Vec::new());
        }
        let mut clean: Vec<String> = DISM_ARGS.iter().map(|s| (*s).to_owned()).collect();
        clean.push("/StartComponentCleanup".into());
        Ok(vec![Finding {
            id: format!("{}:cleanup", self.meta.id),
            module_id: self.meta.id.clone(),
            category: self.meta.category,
            label: t("module.system.component_store.label"),
            detail: Some(t("module.system.component_store.detail")),
            path: Some(ctx.env.windows_dir.join("WinSxS")),
            bytes: a.reclaimable_bytes,
            items: a.packages.max(1),
            risk: self.meta.risk,
            requires_admin: true,
            selected: false,
            age_days: None,
            warning: None,
            action: Action::RunCommand {
                program: "dism".into(),
                args: clean,
            },
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parses_dism_analysis() {
        let out = "\
Deployment Image Servicing and Management tool
Version: 10.0.26100.1

Image Version: 10.0.26100.2033

[===========================100.0%==========================]

Component Store (WinSxS) information:

Windows Explorer Reported Size of Component Store : 7.68 GB

Actual Size of Component Store : 7.47 GB

    Shared with Windows : 5.28 GB
    Backups and Disabled Features : 1.86 GB
    Cache and Temporary Data :  332.46 MB

Date of Last Cleanup : 2026-10-02 10:12:31

Number of Reclaimable Packages : 2
Component Store Cleanup Recommended : Yes

The operation completed successfully.";
        let a = parse_analysis(out);
        assert_eq!(a.packages, 2);
        assert!(a.recommended);
        let gb = 1024u64.pow(3) as f64;
        let expected = 1.86 * gb + 332.46 * 1024.0 * 1024.0;
        assert!((a.reclaimable_bytes as f64 - expected).abs() < 1024.0 * 1024.0);
        assert_eq!(parse_analysis("nothing"), StoreAnalysis::default());
    }

    #[test]
    fn finds_previous_installs_only_at_the_system_root() {
        let d = tempfile::tempdir().unwrap();
        let env = Env::sandbox(d.path());
        fs::create_dir_all(env.windows_dir.clone()).unwrap();
        assert!(previous_installs(&env).is_empty());
        fs::create_dir_all(d.path().join(r"Windows.old\Windows")).unwrap();
        fs::write(d.path().join(r"Windows.old\Windows\a.dll"), b"x").unwrap();
        let found = previous_installs(&env);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "Windows.old");
        assert_eq!(previous_installs_bytes(&env), 1);
    }

    #[test]
    fn component_store_is_only_analyzed_when_turned_on() {
        let d = tempfile::tempdir().unwrap();
        let m = ComponentStore {
            meta: ModuleMeta::new("system.component_store", Category::System),
        };
        // Administrador pero sin activarlo en Ajustes: no se ejecuta DISM.
        let ctx = ScanContext::new(Env::sandbox(d.path()), true);
        assert!(!ctx.component_store);
        assert!(m.scan(&ctx).unwrap().is_empty());
    }

    #[test]
    fn warns_about_personal_files_left_in_windows_old() {
        let d = tempfile::tempdir().unwrap();
        let old = d.path().join("Windows.old");
        let docs = old.join("Users").join("ana").join("Documents");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("desktop.ini"), b"x").unwrap();
        assert!(!has_user_files(&old));
        fs::write(docs.join("tesis.docx"), b"x").unwrap();
        assert!(has_user_files(&old));
        assert!(!has_user_files(d.path()));
    }
}
