# RCleaner — Phased plan

English · [Español](ROADMAP.es.md)

Cleanup and maintenance tool for Windows written in Rust. It brings together
what CCleaner / PC Manager (cleanup), WinDirStat (disk map) and
Bulk Crap Uninstaller (bulk uninstaller) do, with **modes** built around user
profiles (quick, developer, AI, gamer, deep).

## Stack decisions

| Topic | Choice | Why |
|---|---|---|
| Language | Rust 2024 (stable) | Fast, single binary, no GC, memory safe. |
| GUI | **Tauri 2** + Svelte 5 + Vite | ~10 MB binary (uses the system's WebView2), modern UI, builds **MSI (WiX)** and **EXE (NSIS)** with one command. Tauri 3 is still in alpha: not used. |
| Parallelism | `rayon` | Module scans and folder sizing in parallel. |
| FS walking | `walkdir` (doesn't follow links) | Never enters junctions/symlinks → never deletes outside the path. |
| Win32 | `windows-sys` | Minimal FFI: admin, Recycle Bin, (later) registry, restore points. |
| Registry | `winreg` (phases 4-5) | Reading uninstallers and cleaning keys with a `.reg` backup. |
| Duplicate hashing | `blake3` | Size → 16 KiB hash → full hash. BLAKE3 with SIMD is already faster than the disk; a second algorithm adds nothing. |
| CLI | `clap` | Automatable, scriptable and a base for tests. |

## Principles (non-negotiable)

1. **Preview first.** Scanning never deletes. Cleaning requires confirmation (`--yes` in the CLI, a dialog in the GUI).
2. **Never follow links.** Junctions/symlinks are treated as leaves.
3. **Protected path list.** Drive roots, `C:\Windows`, `System32`, `WinSxS`, the user profile, `Program Files`, etc. are never a target.
4. **Explicit risk.** Every finding is `Safe`, `Moderate` or `Advanced`. Only safe ones are preselected.
5. **Admin on demand.** The app runs as the user; what needs admin is marked and skipped without elevation.
6. **Backups** before touching the registry (`.reg` export) and an optional restore point.
7. **Declarative modules.** Adding a cleaner = declaring paths; the delete/measure logic is shared.

## Phases

### Phase 0 — Foundations ✅
- Cargo workspace: `crp-core`, `crp-platform`, `crp-modules`, `crp-cli`.
- Engine: module registry, parallel scan, execution, progress, cancellation, safety guards.
- Embedded modes (TOML), `dev.*` selectors.
- Docs: running the project, architecture, builds.

### Phase 1 — Base cleanup + modes ✅
- **System:** user/Windows temp files, thumbnails, WER, dumps, Windows logs, shader caches (DirectX/NVIDIA/AMD/Intel), Delivery Optimization, Windows Update downloads, Recycle Bin.
- **Browsers (cache only):** Chrome, Edge, Brave, Vivaldi, Opera/GX, Chromium, Firefox.
- **Developer:** pip, uv, poetry, conda (tarballs), npm, pnpm, yarn, bun, cargo, gradle, maven, NuGet, Go, VS Code/Cursor, JetBrains; project artifacts (`node_modules`, `target`, `__pycache__`, `.venv`, `.next`…).
- **AI:** Ollama models (deleting layers no other model uses), Hugging Face hub, LM Studio, Torch/Whisper/Keras.
- `rcl` CLI: `modes`, `modules`, `scan`, `clean`, `info`.

### Phase 2 — Desktop app ✅
- Tauri 2 + Svelte 5. **Home** view (drives, mode shortcuts, history and total freed), **Clean** (live progress, filter, sort by size/age, quick selection Safe/All/None, warnings, confirmation, summary with errors) and **Settings**.
- Persistent settings (`%APPDATA%\RCleaner\settings.json`): project folders (auto or manual), **exclusions** (respected when measuring and deleting), mode on startup.
- **Custom modes** with a visual editor (or by duplicating a built-in one) → `%APPDATA%\RCleaner\modes\*.toml`. The CLI sees them too.
- **Open-program warning** (browsers, editors, JetBrains IDEs, Ollama, LM Studio) via Toolhelp32.
- **Restart as administrator** (UAC) from the app.
- Cleanup history (`history.json`, last 500).
- **MSI** + **NSIS (.exe)** builds, icon, single version in `Cargo.toml`. Optional signing documented.
- Minor pending items: real code signing (needs a certificate), single instance, system tray.

### Phase 3 — Space + duplicates + new UI ✅
- **New interface** (see [DESIGN.md](DESIGN.md)): icon rail, one screen per tool with its own glyph, round button and color; results as rows per category.
- `crp-disk` crate:
  - **Space analyzer:** parallel walk (rayon, never follows links/junctions), compact in-memory tree, level-trimmed views, top large files, types by extension. Cloud files with no local copy count as 0 bytes. Full C:\ (≈3 M files) in ~35 s without admin.
  - Navigable **radial map** (sunburst, DaisyDisk style) + folder list + a "tray" of items to delete.
  - **Duplicates:** size → BLAKE3 of 16 KiB → full BLAKE3; rules out hard links; never reads OneDrive/Drive on-demand files (doesn't download them); skips AppData, `node_modules`, `.git`, Windows, Program Files… by default; auto-selection (keep the oldest/newest/shortest path). Never allows deleting every copy.
- Everything deleted from Space/Duplicates goes to the **Recycle Bin** (`SHFileOperationW` + a warning if something doesn't fit) and goes through `safety::check_target` + the Windows/Program Files/ProgramData block.
- CLI: `rcl space <path>` and `rcl dupes <paths…>`.
- Pending: reading the MFT (faster with admin), alternative treemap, replacing duplicates with hard links.

### Phase 3.5 — Open project ✅
- **Languages:** Spanish and English (Windows language by default) in the app, engine and CLI, from `locales/*.json`; adding one means adding a file.
- Light / dark / system **theme**. Frameless window with custom buttons.
- Documentation for contributors: README (en/es), CONTRIBUTING, code of conduct, SECURITY, translation guide, issue/PR templates and CI on GitHub Actions.

### Phase 4 — Uninstaller (BCUninstaller-style) + dev tools
- Inventory: `HKLM/HKCU\...\Uninstall` (+ WOW6432Node), MSIX/Store (`Get-AppxPackage`), winget.
- Bulk uninstall (silent when there's a `QuietUninstallString`/MSI `/qn`), sequential queue.
- Leftovers: folders in Program Files/AppData, orphaned keys.
- Dev: unused global npm/pip packages, `wsl --list` / `wsl --unregister` (+ compact VHDX), `docker system prune`, AI models by last use.

### Phase 5 — Registry (with backup)
- Orphaned uninstall entries, MUICache/missing paths, extensions with no program, startup entries pointing to deleted files, missing shared DLLs.
- `.reg` export beforehand and restore from the app. Restore point (`SystemRestore.CreateRestorePoint`).

### Phase 6 — Maintenance
- Startup programs, scheduled tasks, services.
- Scheduled cleanup (Task Scheduler), `DISM /StartComponentCleanup`, `Windows.old`.
- Disk health, code signing, zero telemetry.

## What we will NOT do (on purpose)
- Delete **Prefetch**: it slows down startup; Windows manages it.
- Placebo RAM or registry "optimizers".
- Delete cookies/history without an explicit option (privacy = separate, opt-in module).
