# Levantar el proyecto y generar builds

[English](BUILD.md) · Español

## 1. Requisitos

| Herramienta | Versión | Instalar |
|---|---|---|
| Rust (toolchain MSVC) | stable ≥ 1.85 | `winget install Rustlang.Rustup` y luego `rustup default stable-msvc` |
| Build Tools de Visual Studio | 2022+ con "Desarrollo para el escritorio con C++" | `winget install Microsoft.VisualStudio.2022.BuildTools` |
| Node.js | 20 LTS o superior | `winget install OpenJS.NodeJS.LTS` |
| Tauri CLI | 2.x | `cargo install tauri-cli --version "^2" --locked` |
| WebView2 | runtime | Ya viene en Windows 10 21H2+/11 |

WiX (para MSI) y NSIS (para EXE) **no hace falta instalarlos**: la CLI de Tauri
los descarga sola la primera vez que generas un bundle (necesita internet).

> Si `cargo test` falla con *"rustdoc … is not applicable"*, tu toolchain está
> incompleto: `rustup component add rustc rust-std` o reinstala con
> `rustup toolchain install stable --profile default`. Los doctests están
> desactivados, así que no afecta a los tests normales.

## 2. Desarrollo

```powershell
# Motor + módulos + CLI (no necesita Node)
cargo build
cargo test
cargo clippy --all-targets
cargo run -p crp-cli -- scan --mode rapido

# App de escritorio (recarga en caliente de la UI; reinicia al cambiar Rust)
cd app
npm install
cargo tauri dev
```

`cargo tauri dev` lanza `npm run dev` (Vite en `http://localhost:1420`) y abre
la ventana nativa. La consola muestra los `println!`/errores de Rust.

Comprobación de tipos del frontend: `npm run check` (dentro de `app/`).

### Probar como administrador
Algunos módulos (`system.windows_temp`, Windows Update, logs…) requieren
elevación. Abre la terminal como administrador y ejecuta `cargo tauri dev`
o `rcl` desde ahí.

## 3. Builds de producción

```powershell
.\scripts\build.ps1                 # npm ci + MSI + EXE
.\scripts\build.ps1 -SkipInstall    # sin reinstalar node_modules
.\scripts\build.ps1 -Target msi     # sólo MSI
.\scripts\build.ps1 -Target nsis    # sólo EXE
```

Equivale a `cd app; cargo tauri build`. Resultados:

| Tipo | Ruta | Uso |
|---|---|---|
| Ejecutable portable | `target\release\rcleaner.exe` | Sin instalar (requiere WebView2) |
| MSI (WiX) | `target\release\bundle\msi\RCleaner_<ver>_x64_es-ES.msi` | Empresas / GPO / `msiexec /i … /qn` |
| EXE (NSIS) | `target\release\bundle\nsis\RCleaner_<ver>_x64-setup.exe` | Instalador normal para usuarios |
| CLI | `target\release\rcl.exe` (`cargo build -r -p crp-cli`) | Scripts / tareas programadas |

El perfil `release` usa LTO completo, `codegen-units = 1` y `strip` para un
binario pequeño.

### Versión
Se toma de `[workspace.package] version` en el `Cargo.toml` raíz. Cambia sólo ahí.

### Idioma del instalador
`app/src-tauri/tauri.conf.json` → `bundle.windows.wix.language` (`es-ES`) y
`bundle.windows.nsis.languages` (`Spanish`).

### Icono
Edita `app/app-icon.png` y regenera: `cd app; cargo tauri icon app-icon.png -o src-tauri/icons`.

### Firma de código (opcional, recomendado para distribuir)
Sin firma, SmartScreen mostrará "Editor desconocido". Con un certificado:

```jsonc
// tauri.conf.json → bundle.windows
"certificateThumbprint": "<huella SHA1>",
"digestAlgorithm": "sha256",
"timestampUrl": "http://timestamp.digicert.com"
```

## 4. Solución de problemas

| Síntoma | Causa / solución |
|---|---|
| `frontendDist ... doesn't exist` al compilar la app | Ejecuta `npm run build` en `app/` o usa `cargo tauri build/dev` (lo hace solo). `cargo build` a secas sólo compila las crates. |
| Falla la descarga de WiX/NSIS | Sin internet o proxy. Descarga manual: ver docs de Tauri "Windows Installer". |
| `link.exe not found` | Faltan las Build Tools de VS con C++. |
| `Found version mismatched Tauri packages` | El crate `tauri` y el paquete npm `@tauri-apps/api` deben compartir *major.minor*. Mira `grep -A1 'name = "tauri"$' Cargo.lock` y fija `@tauri-apps/api` a esa minor en `app/package.json` (p. ej. `~2.11.1`). Al subir Tauri, sube ambos a la vez. |
| Muchos "omitidos" al limpiar | Archivos en uso (navegador/editor abiertos) o sin permisos: cierra apps o ejecuta como admin. |
