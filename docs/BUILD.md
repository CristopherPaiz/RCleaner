# Running the project and building

English · [Español](BUILD.es.md)

## 1. Requirements

| Tool | Version | Install |
|---|---|---|
| Rust (MSVC toolchain) | stable ≥ 1.85 | `winget install Rustlang.Rustup`, then `rustup default stable-msvc` |
| Visual Studio Build Tools | 2022+ with "Desktop development with C++" | `winget install Microsoft.VisualStudio.2022.BuildTools` |
| Node.js | 20 LTS or newer | `winget install OpenJS.NodeJS.LTS` |
| Tauri CLI | 2.x | `cargo install tauri-cli --version "^2" --locked` |
| WebView2 | runtime | Already included in Windows 10 21H2+/11 |

You **don't need to install** WiX (for MSI) or NSIS (for EXE): the Tauri CLI
downloads them the first time you build a bundle (needs internet).

> If `cargo test` fails with *"rustdoc … is not applicable"*, your toolchain is
> incomplete: `rustup component add rustc rust-std`, or reinstall with
> `rustup toolchain install stable --profile default`. Doctests are disabled,
> so this doesn't affect the regular tests.

## 2. Development

```powershell
# Engine + modules + CLI (no Node needed)
cargo build
cargo test
cargo clippy --all-targets
cargo run -p crp-cli -- scan --mode rapido

# Desktop app (hot reload for the UI; restarts when Rust changes)
cd app
npm install
cargo tauri dev
```

`cargo tauri dev` starts `npm run dev` (Vite on `http://localhost:1420`) and
opens the native window. The console shows Rust's `println!` output and errors.

Frontend type checking: `npm run check` (inside `app/`).

### Testing as administrator
Some modules (`system.windows_temp`, Windows Update, logs…) need elevation.
Open the terminal as administrator and run `cargo tauri dev` or `rcl` from
there.

## 3. Production builds

```powershell
.\scripts\build.ps1                 # npm ci + MSI + EXE
.\scripts\build.ps1 -SkipInstall    # without reinstalling node_modules
.\scripts\build.ps1 -Target msi     # MSI only
.\scripts\build.ps1 -Target nsis    # EXE only
```

Same as `cd app; cargo tauri build`. Output:

| Type | Path | Use |
|---|---|---|
| Portable executable | `target\release\rcleaner.exe` | No install (needs WebView2) |
| MSI (WiX) | `target\release\bundle\msi\RCleaner_<ver>_x64_es-ES.msi` | Companies / GPO / `msiexec /i … /qn` |
| EXE (NSIS) | `target\release\bundle\nsis\RCleaner_<ver>_x64-setup.exe` | Regular installer for users |
| CLI | `target\release\rcl.exe` (`cargo build -r -p crp-cli`) | Scripts / scheduled tasks |

The `release` profile uses full LTO, `codegen-units = 1` and `strip` for a
small binary.

### Version
Taken from `[workspace.package] version` in the root `Cargo.toml`. Change it
only there.

### Installer language
`app/src-tauri/tauri.conf.json` → `bundle.windows.wix.language` (`es-ES`) and
`bundle.windows.nsis.languages` (`Spanish`).

### Icon
Edit `app/app-icon.png` and regenerate: `cd app; cargo tauri icon app-icon.png -o src-tauri/icons`.

### Code signing (optional, recommended for distribution)
Without a signature, SmartScreen shows "Unknown publisher". With a certificate:

```jsonc
// tauri.conf.json → bundle.windows
"certificateThumbprint": "<SHA1 thumbprint>",
"digestAlgorithm": "sha256",
"timestampUrl": "http://timestamp.digicert.com"
```

## 4. Troubleshooting

| Symptom | Cause / fix |
|---|---|
| `frontendDist ... doesn't exist` when building the app | Run `npm run build` in `app/` or use `cargo tauri build/dev` (it does that for you). Plain `cargo build` only compiles the crates. |
| WiX/NSIS download fails | No internet, or a proxy. Manual download: see the Tauri docs, "Windows Installer". |
| `link.exe not found` | The VS Build Tools with C++ are missing. |
| `Found version mismatched Tauri packages` | The `tauri` crate and the npm package `@tauri-apps/api` must share *major.minor*. Check `grep -A1 'name = "tauri"$' Cargo.lock` and pin `@tauri-apps/api` to that minor in `app/package.json` (e.g. `~2.11.1`). When upgrading Tauri, upgrade both together. |
| Many "skipped" items when cleaning | Files in use (browser/editor open) or missing permissions: close apps or run as admin. |
