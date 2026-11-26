//! Incrusta todos los `locales/*.json` de la raíz del repositorio. Añadir un
//! idioma es sólo añadir su archivo: no hay que tocar código.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../locales");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("falta la carpeta locales/ en la raíz del repositorio")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let mut out = String::from("pub(crate) static LOCALES: &[(&str, &str)] = &[\n");
    for f in &files {
        let code = f.file_stem().unwrap().to_string_lossy();
        let abs = f.canonicalize().unwrap();
        let abs = abs.to_string_lossy();
        let abs = abs.strip_prefix(r"\\?\").unwrap_or(&abs);
        writeln!(out, "    (\"{code}\", include_str!(r#\"{abs}\"#)),").unwrap();
    }
    out.push_str("];\n");
    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("locales.rs");
    std::fs::write(dest, out).unwrap();
}
