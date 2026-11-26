# Running the project and building

English · [Español](BUILD.es.md)

## 1. Requirements

| Tool | Version | Install |
|---|---|---|
| Rust (MSVC toolchain) | stable ≥ 1.85 | `winget install Rustlang.Rustup`, then `rustup default stable-msvc` |
| Visual Studio Build Tools | 2022+ with "Desktop development with C++" | `winget install Microsoft.VisualStudio.2022.BuildTools` |
| Node.js | 20 LTS or newer | `winget install OpenJS.NodeJS.LTS` |
| Tauri CLI | 2.x | `cargo install tauri-cli --version "^2" --locked` |
| WebView2 | runtime | Already on Windows 10/11; installers add it if missing (below) |

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
| Portable executable | `target\release\rcleaner.exe` | No install (needs WebView2); published in each release as `RCleaner_<version>_x64-portable.exe` |
| MSI (WiX) | `target\release\bundle\msi\RCleaner_<ver>_x64_es-ES.msi` | Companies / GPO / `msiexec /i … /qn` |
| EXE (NSIS) | `target\release\bundle\nsis\RCleaner_<ver>_x64-setup.exe` | Regular installer for users |
| CLI | `target\release\rcl.exe` (`cargo build -r -p crp-cli`) | Scripts / scheduled tasks |

The `release` profile uses full LTO, `codegen-units = 1` and `strip` for a
small binary.

### Self-contained installers
RCleaner must work on any Windows 10/11 PC, even freshly installed. Two things
it uses are not guaranteed to be there, so the installers take care of them:

- **WebView2** (the Edge engine Tauri draws the UI with): it ships with
  Windows 11 and up-to-date Windows 10. `webviewInstallMode` is
  `embedBootstrapper`: the MSI/EXE carry only Microsoft's small bootstrapper
  (~2 MB) and, if the PC lacks WebView2, download and install it silently
  (that rare case needs internet). This keeps the installers at a few MB;
  `offlineInstaller` would make them ~285 MB.
- **Visual C++ runtime**: `.cargo/config.toml` links it statically
  (`+crt-static`), so no "Visual C++ Redistributable" is needed.

The portable `rcleaner.exe` alone still needs WebView2 (present on every
Windows 11 and up-to-date Windows 10); use the installers to be safe.

### Version
Taken from `[workspace.package] version` in the root `Cargo.toml`.
`scripts/release.ps1` changes it for you (see [Releases](#releases)).

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

## Releases

GitHub builds and publishes releases; the notes come from
[CHANGELOG.md](../CHANGELOG.md).

1. While you work, add each user-facing change under `## [Unreleased]` in
   `CHANGELOG.md` (`### Added`, `### Changed`, `### Fixed`, `### Removed`,
   `### Security`) and commit it with the change.
2. To release, on an up-to-date, clean `main`:

   ```powershell
   .\scripts\release.ps1 0.2.0
   ```

   It moves the Unreleased notes to `## [0.2.0] - <date>`, sets the version
   in `Cargo.toml`, `Cargo.lock` and `app/package*.json`, commits
   `chore(release): v0.2.0`, tags `v0.2.0` and pushes. Use `-NoPush` to
   review first, then `git push origin main v0.2.0`.
3. The tag starts [`.github/workflows/release.yml`](../.github/workflows/release.yml):
   it checks that the tag matches `Cargo.toml` and that the changelog has the
   section, builds the MSI and EXE installers and creates the GitHub release
   with both files and that section as notes (~15 min).

Versions must be plain `X.Y.Z` (MSI does not accept `-beta` suffixes).

## 4. Troubleshooting

| Symptom | Cause / fix |
|---|---|
| `frontendDist ... doesn't exist` when building the app | Run `npm run build` in `app/` or use `cargo tauri build/dev` (it does that for you). Plain `cargo build` only compiles the crates. |
| WiX/NSIS download fails | No internet, or a proxy. Manual download: see the Tauri docs, "Windows Installer". |
| `link.exe not found` | The VS Build Tools with C++ are missing. |
| `Found version mismatched Tauri packages` | The `tauri` crate and the npm package `@tauri-apps/api` must share *major.minor*. Check `grep -A1 'name = "tauri"$' Cargo.lock` and pin `@tauri-apps/api` to that minor in `app/package.json` (e.g. `~2.11.1`). When upgrading Tauri, upgrade both together. |
| Many "skipped" items when cleaning | Files in use (browser/editor open) or missing permissions: close apps or run as admin. |
