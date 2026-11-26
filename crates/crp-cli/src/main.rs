//! `crp` — Clean-CRP desde la terminal.
//!
//! ```text
//! crp modes                       # lista los modos
//! crp modules --mode programador  # módulos de un modo
//! crp scan --mode programador     # vista previa (no borra nada)
//! crp clean --mode rapido         # simulación: muestra qué borraría
//! crp clean --mode rapido --yes   # limpia de verdad
//! ```

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use crp_core::{
    CleanOptions, Engine, Finding, ProgressEvent, Risk, ScanContext, ScanReport, modes,
};
use humansize::{WINDOWS, format_size};
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "crp",
    version,
    about = "Clean-CRP: limpiador modular para Windows"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Información del entorno (rutas, permisos, carpetas de proyectos).
    Info,
    /// Lista los modos disponibles.
    Modes,
    /// Lista los módulos (opcionalmente sólo los de un modo).
    Modules {
        #[arg(long)]
        mode: Option<String>,
    },
    /// Escanea y muestra lo que se puede limpiar. No borra nada.
    Scan(ScanArgs),
    /// Limpia. Sin `--yes` sólo simula.
    Clean {
        #[command(flatten)]
        scan: ScanArgs,
        /// Riesgo máximo a incluir (por defecto sólo lo preseleccionado: seguro).
        #[arg(long, value_enum)]
        risk: Option<RiskArg>,
        /// Limpiar sólo hallazgos cuyo id contenga alguno de estos textos.
        #[arg(long = "only", num_args = 1..)]
        only: Vec<String>,
        /// Ejecutar de verdad (sin esto es una simulación).
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Args)]
struct ScanArgs {
    /// Modo a usar (ver `crp modes`). Por defecto: rapido.
    #[arg(long, short)]
    mode: Option<String>,
    /// Selectores de módulo en lugar de un modo (`dev.*`, `system.user_temp`, `!ai.*`).
    #[arg(long = "module", num_args = 1..)]
    modules: Vec<String>,
    /// Carpetas de proyectos para `dev.projects` (sustituye a las detectadas).
    #[arg(long = "root", num_args = 1..)]
    roots: Vec<PathBuf>,
    /// Salida JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum RiskArg {
    Safe,
    Moderate,
    Advanced,
}

impl From<RiskArg> for Risk {
    fn from(r: RiskArg) -> Self {
        match r {
            RiskArg::Safe => Risk::Safe,
            RiskArg::Moderate => Risk::Moderate,
            RiskArg::Advanced => Risk::Advanced,
        }
    }
}

fn size(bytes: u64) -> String {
    format_size(bytes, WINDOWS)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let engine = crp_modules::engine();
    match cli.command {
        Command::Info => info(),
        Command::Modes => {
            let (all, errors) = modes::all(&crp_modules::app_paths().modes_dir());
            for m in all {
                let tag = if m.custom { " (personalizado)" } else { "" };
                println!(
                    "{:<12} {}{tag}\n{:<12} {}\n",
                    m.id, m.name, "", m.description
                );
            }
            for e in errors {
                eprintln!("! {e}");
            }
            Ok(())
        }
        Command::Modules { mode } => {
            let selectors = match mode {
                Some(id) => mode_selectors(&id)?,
                None => vec!["*".into()],
            };
            for m in engine.resolve(&selectors) {
                let meta = m.meta();
                let admin = if meta.requires_admin { " [admin]" } else { "" };
                println!(
                    "{:<30} {:<9} {}{admin}",
                    meta.id,
                    meta.risk.label(),
                    meta.name
                );
            }
            Ok(())
        }
        Command::Scan(args) => {
            let ctx = context(&args);
            let report = scan(&engine, &args, &ctx)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print_report(&report, &ctx);
            }
            Ok(())
        }
        Command::Clean {
            scan: args,
            risk,
            only,
            yes,
        } => {
            let ctx = context(&args);
            let report = scan(&engine, &args, &ctx)?;
            let chosen: Vec<Finding> = report
                .findings
                .into_iter()
                .filter(|f| match (&only[..], risk) {
                    ([], None) => f.selected,
                    ([], Some(max)) => f.risk <= max.into(),
                    (ids, _) => ids.iter().any(|id| f.id.contains(id.as_str())),
                })
                .collect();
            clean(&engine, &chosen, &ctx, !yes, args.json)
        }
    }
}

fn info() -> Result<()> {
    let ctx = crp_modules::user_context();
    println!(
        "Ajustes       : {}",
        crp_modules::app_paths().config_dir.display()
    );
    println!("Administrador : {}", if ctx.is_admin { "sí" } else { "no" });
    println!("Perfil        : {}", ctx.env.home.display());
    println!("LocalAppData  : {}", ctx.env.local_app_data.display());
    println!("Temp          : {}", ctx.env.temp.display());
    println!("Windows       : {}", ctx.env.windows_dir.display());
    println!("Proyectos     :");
    for r in &ctx.dev_roots {
        println!("  {}", r.display());
    }
    if !ctx.exclusions.is_empty() {
        println!("Exclusiones   :");
        for r in &ctx.exclusions {
            println!("  {}", r.display());
        }
    }
    Ok(())
}

fn mode_selectors(id: &str) -> Result<Vec<String>> {
    let (all, _) = modes::all(&crp_modules::app_paths().modes_dir());
    match all.into_iter().find(|m| m.id == id) {
        Some(m) => Ok(m.modules),
        None => bail!("modo desconocido '{id}'. Usa `crp modes` para verlos."),
    }
}

fn context(args: &ScanArgs) -> ScanContext {
    let ctx = crp_modules::user_context();
    if args.roots.is_empty() {
        ctx
    } else {
        ctx.with_dev_roots(args.roots.clone())
    }
}

fn scan(engine: &Engine, args: &ScanArgs, ctx: &ScanContext) -> Result<ScanReport> {
    let selectors = if !args.modules.is_empty() {
        args.modules.clone()
    } else {
        mode_selectors(args.mode.as_deref().unwrap_or("rapido"))?
    };
    // Progreso en stderr con formato "N/M" (lo entienden herramientas externas).
    let progress = |e: ProgressEvent| {
        if let ProgressEvent::ModuleFinished {
            module_id,
            done,
            total,
            ..
        } = e
        {
            eprint!("\r[{done}/{total}] {module_id:<40}");
            let _ = std::io::stderr().flush();
        }
    };
    let report = engine.scan(&selectors, ctx, &progress);
    eprintln!("\r{:<60}", "");
    Ok(report)
}

fn print_report(report: &ScanReport, ctx: &ScanContext) {
    let mut findings: Vec<&Finding> = report.findings.iter().collect();
    findings.sort_by(|a, b| a.category.cmp(&b.category).then(b.bytes.cmp(&a.bytes)));

    let mut current = None;
    for f in &findings {
        if current != Some(f.category) {
            current = Some(f.category);
            println!("\n== {} ==", f.category.label());
        }
        let mark = if f.selected { "[x]" } else { "[ ]" };
        let admin = if f.requires_admin && !ctx.is_admin {
            " (requiere admin)"
        } else {
            ""
        };
        println!(
            "{mark} {:>10}  {:<9} {}{admin}\n               {}",
            size(f.bytes),
            f.risk.label(),
            f.label,
            f.id
        );
        if let Some(d) = &f.detail {
            println!("               {d}");
        }
        if let Some(w) = &f.warning {
            println!("               ! {w}");
        }
    }
    for e in &report.errors {
        println!("\n! {}: {}", e.module_id, e.message);
    }
    println!(
        "\nTotal encontrado: {}  ·  preseleccionado (seguro): {}  ·  {} ms",
        size(report.total_bytes()),
        size(report.selected_bytes()),
        report.elapsed_ms
    );
    if findings.iter().any(|f| f.requires_admin) && !ctx.is_admin {
        println!("Algunos elementos requieren ejecutar como administrador.");
    }
}

fn clean(
    engine: &Engine,
    chosen: &[Finding],
    ctx: &ScanContext,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    if chosen.is_empty() {
        println!("Nada que limpiar con esos criterios.");
        return Ok(());
    }
    let progress = |e: ProgressEvent| {
        if let ProgressEvent::FindingCleaned { done, total, .. } = e {
            eprint!("\r[{done}/{total}] limpiando…");
            let _ = std::io::stderr().flush();
        }
    };
    let report = engine.clean(chosen, CleanOptions { dry_run }, ctx, &progress);
    eprintln!("\r{:<40}", "");

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    for (f, o) in chosen.iter().zip(&report.outcomes) {
        println!("{:>10}  {}", size(o.freed_bytes), f.label);
        for e in o.errors.iter().take(3) {
            println!("            ! {e}");
        }
    }
    if dry_run {
        println!(
            "\nSIMULACIÓN: se liberarían {}. Repite con --yes para limpiar.",
            size(report.freed_bytes())
        );
    } else {
        println!(
            "\nLiberado: {}  ·  omitidos (en uso/sin permiso): {}  ·  {} ms",
            size(report.freed_bytes()),
            report.skipped_items(),
            report.elapsed_ms
        );
    }
    Ok(())
}
