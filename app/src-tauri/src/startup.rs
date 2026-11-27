//! Arranque: programas al iniciar sesión, tareas programadas y servicios de
//! terceros. Sólo se activan/desactivan (o se cambia el tipo de inicio de un
//! servicio); nunca se borra nada, así que todo se puede deshacer.
//!
//! Como en el resto de comandos, la UI manda identificadores y aquí se
//! comprueba que existan en el último listado antes de tocar nada.

use crp_core::{Env, exec, t};
use crp_platform::startup::{self, Scope, Source, StartupEntry};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

type CmdResult<T> = Result<T, String>;

#[derive(Default)]
struct Last {
    apps: Vec<StartupEntry>,
    tasks: Vec<String>,
    services: Vec<String>,
}

fn last() -> MutexGuard<'static, Last> {
    static LAST: OnceLock<Mutex<Last>> = OnceLock::new();
    LAST.get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Ejecutable de una entrada del último listado (para su icono).
pub fn file_of(id: &str) -> Option<PathBuf> {
    last()
        .apps
        .iter()
        .find(|e| e.id() == id)
        .and_then(|e| e.file.clone())
}

/// ¿Es el ejecutable de algo del último listado? (para "Abrir ubicación").
pub fn known_file(path: &Path) -> bool {
    last().apps.iter().any(|e| e.file.as_deref() == Some(path))
}

/// Programas habilitados al iniciar sesión (para el Resumen).
pub fn enabled_apps() -> usize {
    startup::list().iter().filter(|e| e.enabled).count()
}

// ── Programas ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct AppItem {
    id: String,
    name: String,
    command: String,
    file: Option<PathBuf>,
    /// El archivo ya no existe: la entrada no hace nada.
    missing: bool,
    scope: Scope,
    source: Source,
    enabled: bool,
}

#[tauri::command]
pub async fn startup_apps() -> CmdResult<Vec<AppItem>> {
    tauri::async_runtime::spawn_blocking(|| {
        let list = startup::list();
        let items = list
            .iter()
            .map(|e| AppItem {
                id: e.id(),
                name: display_name(e),
                command: e.command.clone(),
                missing: e
                    .file
                    .as_ref()
                    .is_some_and(|f| f.is_absolute() && !f.exists()),
                file: e.file.clone(),
                scope: e.scope,
                source: e.source,
                enabled: e.enabled,
            })
            .collect();
        last().apps = list;
        items
    })
    .await
    .map_err(|e| e.to_string())
}

/// En la carpeta Inicio el nombre es el del archivo: sin `.lnk`.
fn display_name(e: &StartupEntry) -> String {
    match e.source {
        Source::Folder => Path::new(&e.name)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| e.name.clone()),
        _ => e.name.clone(),
    }
}

#[tauri::command]
pub fn set_startup_app(id: String, enabled: bool) -> CmdResult<()> {
    let entry = last()
        .apps
        .iter()
        .find(|e| e.id() == id)
        .cloned()
        .ok_or_else(|| t("app.unknown_path"))?;
    startup::set_enabled(entry.scope, entry.source, &entry.name, enabled).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            t("ui.startup.needs_admin")
        } else {
            e.to_string()
        }
    })
}

// ── PowerShell ──────────────────────────────────────────────────────────

/// Ejecuta un script de PowerShell y lee su salida JSON (siempre una lista).
fn powershell<T: for<'de> Deserialize<'de>>(script: &str) -> CmdResult<Vec<T>> {
    let full = format!(
        "$ErrorActionPreference='SilentlyContinue';\
         [Console]::OutputEncoding=[Text.Encoding]::UTF8;\
         $r = @({script});\
         ConvertTo-Json -InputObject $r -Compress -Depth 3"
    );
    let args = ["-NoProfile", "-NonInteractive", "-Command", &full].map(String::from);
    let out = exec::command_output(&Env::from_system(), "powershell", &args)
        .map_err(|e| e.to_string())?;
    let out = out.trim();
    if out.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(out).map_err(|e| e.to_string())
}

fn run(program: &str, args: &[&str]) -> CmdResult<()> {
    let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    exec::run_command(&Env::from_system(), program, &args).map_err(|e| e.to_string())
}

// ── Tareas programadas ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct RawTask {
    name: String,
    path: String,
    state: Option<String>,
    author: Option<String>,
    exe: Option<String>,
    args: Option<String>,
    #[serde(default)]
    triggers: Vec<String>,
}

#[derive(Serialize)]
pub struct TaskItem {
    /// Ruta completa (`\Carpeta\Nombre`).
    id: String,
    name: String,
    folder: String,
    command: String,
    author: Option<String>,
    enabled: bool,
    running: bool,
    /// `logon`, `boot`, `schedule`, `idle`, `event`, `other`.
    triggers: Vec<String>,
}

/// Las de Windows (`\Microsoft\…`) no se listan: no son de terceros.
const TASKS_PS: &str = "Get-ScheduledTask | Where-Object { $_.TaskPath -notlike '\\Microsoft\\*' } | ForEach-Object { \
    $a = @($_.Actions)[0]; \
    [pscustomobject]@{ name = $_.TaskName; path = $_.TaskPath; state = [string]$_.State; author = $_.Author; \
      exe = $a.Execute; args = $a.Arguments; \
      triggers = @($_.Triggers | ForEach-Object { $_.CimClass.CimClassName }) } }";

fn trigger_kind(class: &str) -> &'static str {
    let c = class.to_ascii_lowercase();
    if c.contains("logon") {
        "logon"
    } else if c.contains("boot") {
        "boot"
    } else if c.contains("daily")
        || c.contains("weekly")
        || c.contains("monthly")
        || c.contains("time")
    {
        "schedule"
    } else if c.contains("idle") {
        "idle"
    } else if c.contains("event") {
        "event"
    } else {
        "other"
    }
}

#[tauri::command]
pub async fn startup_tasks() -> CmdResult<Vec<TaskItem>> {
    let raw: Vec<RawTask> = tauri::async_runtime::spawn_blocking(|| powershell(TASKS_PS))
        .await
        .map_err(|e| e.to_string())??;
    let items: Vec<TaskItem> = raw
        .into_iter()
        .map(|r| {
            let state = r.state.unwrap_or_default().to_ascii_lowercase();
            let mut triggers: Vec<String> = r
                .triggers
                .iter()
                .map(|c| trigger_kind(c).to_owned())
                .collect();
            triggers.dedup();
            TaskItem {
                id: format!("{}{}", r.path, r.name),
                folder: r.path.trim_matches('\\').to_owned(),
                command: [r.exe.unwrap_or_default(), r.args.unwrap_or_default()]
                    .join(" ")
                    .trim()
                    .to_owned(),
                name: r.name,
                author: r.author.filter(|a| !a.trim().is_empty()),
                enabled: state != "disabled",
                running: state == "running",
                triggers,
            }
        })
        .collect();
    last().tasks = items.iter().map(|t| t.id.clone()).collect();
    Ok(items)
}

#[tauri::command]
pub async fn set_startup_task(id: String, enabled: bool) -> CmdResult<()> {
    if !last().tasks.contains(&id) {
        return Err(t("app.unknown_path"));
    }
    let flag = if enabled { "/ENABLE" } else { "/DISABLE" };
    tauri::async_runtime::spawn_blocking(move || run("schtasks", &["/Change", "/TN", &id, flag]))
        .await
        .map_err(|e| e.to_string())?
}

// ── Servicios ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct RawService {
    name: String,
    display: Option<String>,
    desc: Option<String>,
    start: Option<String>,
    state: Option<String>,
    path: Option<String>,
    delayed: Option<bool>,
}

#[derive(Serialize)]
pub struct ServiceItem {
    name: String,
    display: String,
    description: Option<String>,
    /// `auto`, `manual`, `disabled` (u otro que no se puede cambiar aquí).
    start: String,
    delayed: bool,
    running: bool,
    path: String,
}

const SERVICES_PS: &str = "Get-CimInstance Win32_Service | ForEach-Object { \
    [pscustomobject]@{ name = $_.Name; display = $_.DisplayName; desc = $_.Description; start = $_.StartMode; \
      state = $_.State; path = $_.PathName; delayed = $_.DelayedAutoStart } }";

/// Servicios de Windows: su ejecutable vive en la carpeta de Windows o es
/// Windows Defender. No se listan para no tocarlos por error.
fn is_windows_service(path: &str, windows_dir: &str) -> bool {
    let p = path.trim_start_matches('"').to_ascii_lowercase();
    p.starts_with(windows_dir) || p.contains(r"\windows defender\")
}

#[tauri::command]
pub async fn startup_services() -> CmdResult<Vec<ServiceItem>> {
    let raw: Vec<RawService> = tauri::async_runtime::spawn_blocking(|| powershell(SERVICES_PS))
        .await
        .map_err(|e| e.to_string())??;
    let windows = Env::from_system()
        .windows_dir
        .to_string_lossy()
        .to_ascii_lowercase();
    let mut items: Vec<ServiceItem> = raw
        .into_iter()
        .filter_map(|r| {
            let path = r.path.filter(|p| !p.trim().is_empty())?;
            if is_windows_service(&path, &windows) {
                return None;
            }
            Some(ServiceItem {
                display: r.display.unwrap_or_else(|| r.name.clone()),
                description: r.desc.filter(|d| !d.trim().is_empty()),
                start: r.start.unwrap_or_default().to_ascii_lowercase(),
                delayed: r.delayed.unwrap_or(false),
                running: r.state.is_some_and(|s| s.eq_ignore_ascii_case("running")),
                path,
                name: r.name,
            })
        })
        .collect();
    items.sort_by_key(|s| s.display.to_lowercase());
    last().services = items.iter().map(|s| s.name.clone()).collect();
    Ok(items)
}

/// `auto` / `manual` / `disabled` (necesita administrador).
#[tauri::command]
pub async fn set_service_start(name: String, start: String) -> CmdResult<()> {
    if !last().services.contains(&name) {
        return Err(t("app.unknown_path"));
    }
    let mode = match start.as_str() {
        "auto" => "auto",
        "manual" => "demand",
        "disabled" => "disabled",
        _ => return Err(t("app.unknown_path")),
    };
    tauri::async_runtime::spawn_blocking(move || run("sc", &["config", &name, "start=", mode]))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_triggers_and_windows_services() {
        assert_eq!(trigger_kind("MSFT_TaskLogonTrigger"), "logon");
        assert_eq!(trigger_kind("MSFT_TaskDailyTrigger"), "schedule");
        assert_eq!(trigger_kind("MSFT_TaskBootTrigger"), "boot");
        let w = r"c:\windows";
        assert!(is_windows_service(
            r"C:\WINDOWS\system32\svchost.exe -k netsvcs",
            w
        ));
        assert!(is_windows_service(
            r#""C:\ProgramData\Microsoft\Windows Defender\platform\4.18\MsMpEng.exe""#,
            w
        ));
        assert!(!is_windows_service(
            r#""C:\Program Files\Steam\steamservice.exe""#,
            w
        ));
    }
}
