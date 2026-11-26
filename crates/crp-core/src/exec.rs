//! Ejecución de las acciones de un hallazgo.

use crate::context::ScanContext;
use crate::env::Env;
use crate::error::{Error, Result};
use crate::fsutil::{self, DeleteStats};
use crate::model::{Action, CleanOutcome, Finding};
use crate::module::CleanerModule;
use crate::safety;
use std::path::PathBuf;
use std::process::Command;

/// Limpia un hallazgo. Con `dry_run` sólo informa lo que se liberaría.
pub fn execute(
    finding: &Finding,
    module: &dyn CleanerModule,
    ctx: &ScanContext,
    dry_run: bool,
) -> CleanOutcome {
    let mut out = CleanOutcome {
        finding_id: finding.id.clone(),
        ..Default::default()
    };

    if finding.requires_admin && !ctx.is_admin {
        out.skipped_items = finding.items;
        out.errors.push(Error::RequiresAdmin.to_string());
        return out;
    }
    if dry_run {
        out.freed_bytes = finding.bytes;
        out.removed_items = finding.items;
        return out;
    }

    let mut stats = DeleteStats::default();
    match &finding.action {
        Action::DeleteContents { roots, filter } => {
            for root in roots {
                match safety::check_target(root, &ctx.env) {
                    Ok(()) => stats.merge(fsutil::delete_contents(root, filter, &ctx.cancel)),
                    Err(e) => out.errors.push(e.to_string()),
                }
            }
        }
        Action::DeletePaths { paths } => {
            for path in paths {
                match safety::check_target(path, &ctx.env) {
                    Ok(()) => stats.merge(fsutil::delete_path(path, &ctx.cancel)),
                    Err(e) => out.errors.push(e.to_string()),
                }
            }
        }
        Action::RunCommand { program, args } => match run_command(&ctx.env, program, args) {
            Ok(()) => {
                stats.freed = finding.bytes;
                stats.removed = finding.items;
            }
            Err(e) => out.errors.push(e.to_string()),
        },
        Action::Custom { .. } => match module.clean_custom(finding, ctx) {
            Ok(custom) => {
                return CleanOutcome {
                    finding_id: finding.id.clone(),
                    ..custom
                };
            }
            Err(e) => out.errors.push(e.to_string()),
        },
    }

    out.freed_bytes = stats.freed;
    out.removed_items = stats.removed;
    out.skipped_items = stats.skipped;
    out.errors.extend(stats.errors);
    out
}

/// Busca un ejecutable en `PATH` respetando `PATHEXT` (`npm` → `npm.cmd`).
pub fn which(env: &Env, program: &str) -> Option<PathBuf> {
    let exts: Vec<String> = env
        .var("PATHEXT")
        .unwrap_or(".COM;.EXE;.BAT;.CMD")
        .split(';')
        .filter(|e| !e.is_empty())
        .map(str::to_owned)
        .collect();
    let has_ext = std::path::Path::new(program).extension().is_some();
    let path = env.var("PATH")?;
    for dir in std::env::split_paths(path) {
        if has_ext {
            let p = dir.join(program);
            if p.is_file() {
                return Some(p);
            }
        }
        for ext in &exts {
            let p = dir.join(format!("{program}{ext}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Ejecuta un programa sin ventana y devuelve error con su stderr si falla.
pub fn run_command(env: &Env, program: &str, args: &[String]) -> Result<()> {
    let exe = which(env, program).ok_or_else(|| Error::Command {
        program: program.to_owned(),
        message: "no se encontró en PATH".into(),
    })?;
    let mut cmd = Command::new(&exe);
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd.output().map_err(|e| Error::Command {
        program: program.to_owned(),
        message: e.to_string(),
    })?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(Error::Command {
            program: program.to_owned(),
            message: format!("{} {}", output.status, stderr.trim()),
        })
    }
}
