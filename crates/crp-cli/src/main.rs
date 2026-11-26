//! `rcl` — RCleaner desde la terminal.
//!
//! ```text
//! rcl modes                       # lista los modos
//! rcl modules --mode programador  # módulos de un modo
//! rcl scan --mode programador     # vista previa (no borra nada)
//! rcl clean --mode rapido         # simulación: muestra qué borraría
//! rcl clean --mode rapido --yes   # limpia de verdad
//! rcl space C:\ --top 20          # qué ocupa espacio
//! rcl dupes D:\Fotos              # archivos duplicados (no borra)
//! ```
// Todo el código de sistema (`unsafe`/FFI) vive sólo en crp-platform.
#![forbid(unsafe_code)]

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use crp_core::{
    CleanOptions, Engine, Finding, ProgressEvent, Risk, ScanContext, ScanReport, modes, t, tf, tn,
};
use humansize::{WINDOWS, format_size};
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rcl", version, about = "RCleaner: modular cleaner for Windows")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Environment info (paths, permissions, project folders).
    Info,
    /// List available modes.
    Modes,
    /// List modules (optionally only those of a mode).
    Modules {
        #[arg(long)]
        mode: Option<String>,
    },
    /// Scan and show what can be cleaned. Deletes nothing.
    Scan(ScanArgs),
    /// Clean. Without `--yes` it is a dry run.
    Clean {
        #[command(flatten)]
        scan: ScanArgs,
        /// Highest risk to include (default: only the preselected, safe items).
        #[arg(long, value_enum)]
        risk: Option<RiskArg>,
        /// Only clean findings whose id contains one of these texts.
        #[arg(long = "only", num_args = 1..)]
        only: Vec<String>,
        /// Really delete (otherwise it is a dry run).
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// What takes up space in a drive or folder. Deletes nothing.
    Space {
        path: PathBuf,
        /// How many folders/files to show.
        #[arg(long, default_value_t = 15)]
        top: usize,
    },
    /// Find duplicate files by content. Deletes nothing.
    Dupes {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Minimum size in KB.
        #[arg(long, default_value_t = 1024)]
        min_kb: u64,
        /// Also walk AppData, node_modules, .git, Windows…
        #[arg(long)]
        all_dirs: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Args)]
struct ScanArgs {
    /// Mode to use (see `rcl modes`). Default: rapido.
    #[arg(long, short)]
    mode: Option<String>,
    /// Module selectors instead of a mode (`dev.*`, `system.user_temp`, `!ai.*`).
    #[arg(long = "module", num_args = 1..)]
    modules: Vec<String>,
    /// Project folders for `dev.projects` (replaces the detected ones).
    #[arg(long = "root", num_args = 1..)]
    roots: Vec<PathBuf>,
    /// JSON output.
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
    let settings =
        crp_core::Settings::load(&crp_modules::app_paths().settings_file()).unwrap_or_default();
    let lang = settings
        .language
        .or_else(crp_platform::locale::ui_language)
        .unwrap_or_default();
    crp_core::i18n::set_language(&lang);
    let engine = crp_modules::engine();
    match cli.command {
        Command::Info => info(),
        Command::Modes => {
            let (all, errors) = modes::all(&crp_modules::app_paths().modes_dir());
            for m in all {
                let tag = if m.custom {
                    format!(" ({})", t("cli.custom"))
                } else {
                    String::new()
                };
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
        Command::Space { path, top } => space(&path, top),
        Command::Dupes {
            paths,
            min_kb,
            all_dirs,
            json,
        } => dupes(paths, min_kb, all_dirs, json),
    }
}

fn space(path: &std::path::Path, top: usize) -> Result<()> {
    use crp_disk::tree::{Kind, ScanOptions, ScanProgress, scan};
    let opts = ScanOptions {
        exclusions: crp_modules::user_context().exclusions,
    };
    eprintln!("{}", tf("cli.analyzing", &[("path", &path.display())]));
    let tree = scan(
        path,
        &opts,
        &crp_core::CancelToken::new(),
        &ScanProgress::default(),
    )?;
    let s = tree.summary();
    println!(
        "{}  ·  {}  ·  {}  ·  {:.1} s",
        size(s.bytes),
        tn("cli.files", s.files, &[]),
        tn("cli.folders", s.dirs, &[]),
        s.elapsed_ms as f64 / 1000.0
    );
    if s.unreadable > 0 {
        println!("! {}", tn("cli.unreadable", s.unreadable, &[]));
    }
    println!("\n{}:", t("cli.entries"));
    for e in tree.children(s.root, top) {
        let tag = if e.kind == Kind::Dir { "\\" } else { "" };
        println!("  {:>10}  {}{tag}", size(e.size), e.name);
    }
    println!("\n{}:", t("cli.largest"));
    for f in tree.largest_files(s.root, top) {
        println!("  {:>10}  {}", size(f.size), f.path.display());
    }
    println!("\n{}:", t("cli.types"));
    for e in s.extensions.iter().take(top) {
        let ext = if e.ext.is_empty() {
            format!("({})", t("cli.no_ext"))
        } else {
            e.ext.to_string()
        };
        println!(
            "  {:>10}  {ext:<16} {}",
            size(e.bytes),
            tn("cli.files", e.files, &[])
        );
    }
    Ok(())
}

fn dupes(paths: Vec<PathBuf>, min_kb: u64, all_dirs: bool, json: bool) -> Result<()> {
    use crp_disk::dupes::{DupeOptions, DupeProgress, find};
    let mut opts = DupeOptions::new(paths);
    opts.min_size = min_kb * 1024;
    opts.exclusions = crp_modules::user_context().exclusions;
    if all_dirs {
        opts.skip_dirs.clear();
    }
    let progress = |p: DupeProgress| {
        if let DupeProgress::Hashing { done, total } = p {
            eprint!("\r{} {done}/{total}", t("cli.comparing"));
        }
    };
    let rep = find(&opts, &crp_core::CancelToken::new(), &progress);
    eprintln!();
    if json {
        println!("{}", serde_json::to_string_pretty(&rep)?);
        return Ok(());
    }
    for g in &rep.groups {
        println!("{} × {}", g.files.len(), size(g.size));
        for f in &g.files {
            println!("    {}", f.path.display());
        }
    }
    println!(
        "\n{} · {} · {} · {:.1} s",
        tn("cli.groups", rep.groups.len() as u64, &[]),
        tf("cli.recoverable", &[("size", &size(rep.wasted_bytes))]),
        tn("cli.files_checked", rep.files_scanned, &[]),
        rep.elapsed_ms as f64 / 1000.0
    );
    Ok(())
}

fn info() -> Result<()> {
    let ctx = crp_modules::user_context();
    let row = |k: &str, v: &dyn std::fmt::Display| println!("{:<14}: {v}", t(k));
    row(
        "cli.info.settings",
        &crp_modules::app_paths().config_dir.display(),
    );
    row(
        "cli.info.admin",
        &t(if ctx.is_admin { "cli.yes" } else { "cli.no" }),
    );
    row("cli.info.profile", &ctx.env.home.display());
    row("cli.info.language", &crp_core::i18n::language());
    println!(
        "{:<14}: {}",
        "LocalAppData",
        ctx.env.local_app_data.display()
    );
    println!("{:<14}: {}", "Temp", ctx.env.temp.display());
    println!("{:<14}: {}", "Windows", ctx.env.windows_dir.display());
    println!("{:<14}:", t("cli.info.projects"));
    for r in &ctx.dev_roots {
        println!("  {}", r.display());
    }
    if !ctx.exclusions.is_empty() {
        println!("{:<14}:", t("cli.info.exclusions"));
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
        None => bail!("{}", tf("cli.unknown_mode", &[("id", &id)])),
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
            format!(" ({})", t("cli.needs_admin"))
        } else {
            String::new()
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
        "\n{}",
        tf(
            "cli.scan_total",
            &[
                ("total", &size(report.total_bytes())),
                ("safe", &size(report.selected_bytes())),
                ("ms", &report.elapsed_ms),
            ],
        )
    );
    if findings.iter().any(|f| f.requires_admin) && !ctx.is_admin {
        println!("{}", t("cli.some_need_admin"));
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
        println!("{}", t("cli.nothing"));
        return Ok(());
    }
    let progress = |e: ProgressEvent| {
        if let ProgressEvent::FindingCleaned { done, total, .. } = e {
            eprint!("\r[{done}/{total}] {}", t("cli.cleaning"));
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
            "\n{}",
            tf("cli.dry_run", &[("size", &size(report.freed_bytes()))])
        );
    } else {
        println!(
            "\n{}",
            tf(
                "cli.freed",
                &[
                    ("size", &size(report.freed_bytes())),
                    ("skipped", &report.skipped_items()),
                    ("ms", &report.elapsed_ms),
                ],
            )
        );
    }
    Ok(())
}
