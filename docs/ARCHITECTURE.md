# Architecture

English · [Español](ARCHITECTURE.es.md)

```
          ┌──────────────┐     ┌──────────────────────┐
          │   crp-cli    │     │  app (Tauri + Svelte) │
          └──────┬───────┘     └──────────┬───────────┘
                 │  Engine::scan / clean   │ commands + events
                 ▼                         ▼
          ┌─────────────────────────────┐  ┌──────────────────┐
          │         crp-modules          │  │     crp-disk     │
          │ system · browsers · dev · ai │  │  space · dupes   │
          └──────────────┬──────────────┘  └────────┬─────────┘
                         │ implement CleanerModule / PathRuleModule
          ┌──────────────▼──────────────┐   ┌────────────────┐
          │           crp-core          │   │  crp-platform  │
          │ model · engine · fsutil ·   │   │ Win32 (unsafe) │
          │ safety · modes · progress   │   └────────────────┘
          └─────────────────────────────┘
```

Dependency rule: `core` depends on no other crate in the project, and neither
does `platform`; `modules` and `disk` use both; the CLI and the app use
`modules`, `disk` and `core`.

## Concepts

| Type | Where | What it is |
|---|---|---|
| `ModuleMeta` | `core::module` | id (`category.name`), name, description, category, default risk, whether it needs admin, processes that block it (`.closes(&["chrome.exe"])`) |
| `CleanerModule` | `core::module` | `scan(ctx) -> Vec<Finding>` (no side effects) and optional `clean_custom` |
| `Finding` | `core::model` | something that can be cleaned: label, bytes, items, risk, `selected`, `age_days`, `warning`, `Action` |
| `Action` | `core::model` | `DeleteContents{roots, filter}`, `DeletePaths`, `RunCommand`, `Custom` |
| `Engine` | `core::engine` | module registry, parallel `scan` (rayon), `clean`; adds warnings when one of a module's processes is running |
| `Mode` | `core::modes` + `modes/builtin.toml` | list of selectors (`dev.*`, `!ai.*`, `*`); `modes::all/save_user/delete_user` for user modes |
| `ScanContext` | `core::context` | `Env` (paths), cancellation, admin, project folders, **exclusions**, running processes |
| `Settings`, `AppPaths`, history | `core::settings` | `%APPDATA%\RCleaner\{settings.json, history.json, modes\}` with atomic writes |

## Deletion safety

1. **Scanning never writes.** Only `Engine::clean` deletes.
2. `safety::check_target` rejects drive roots, the user's base folders,
   `Program Files`, `ProgramData`, `C:\Windows`, and anything inside
   `System32`/`SysWOW64`/`WinSxS`/`servicing`/`Boot`. It runs on **every**
   root before deleting, even if the module gets it wrong.
3. `walkdir` with `follow_links(false)`: symlinks and junctions are removed as
   links; their target is never walked (test `never_follows_symlinked_dirs`).
4. Files in use → skipped and counted (`skipped_items`); they don't abort.
5. Read-only → the attribute is cleared and the delete retried (e.g. Go modules).
6. The GUI only sends finding **ids** to the backend; Rust decides the paths
   from its last scan.
7. `requires_admin` findings are skipped when the process isn't elevated.
8. A `panic` in a module is caught and reported as that module's error.
9. User **exclusions**: `Target::into_finding` drops excluded roots and full
   paths that contain an exclusion; exclusions inside a folder being emptied
   travel in `EntryFilter.exclude`, and the walk never enters them, either to
   measure or to delete. (Note: the walk is pre-order; walkdir with
   `contents_first` + `filter_entry` skips siblings.)
10. Space/Duplicates go through `deletable()`: `check_target` always applies;
    `protect::reason` (system files) and Program Files/Windows only lift when
    the user turns on `allow_system`, and the UI asks for confirmation on every
    such pick. Exclusions are never deletable.
11. Every deletion is appended to `deleted.log` (JSON lines, trimmed past
    4 MB) with source, path, size and whether it was permanent.
12. Startup (programs, tasks, services) only toggles: `StartupApproved` like
    Task Manager, `schtasks /Change`, `sc config start=`. Nothing is deleted,
    and only items from the last listing are accepted from the UI.
13. `unsafe` is forbidden (`#![forbid(unsafe_code)]`) everywhere except
    `crp-platform`, which wraps the Windows API.

## Space and duplicates (`crp-disk`)

- `tree::scan` walks with `std::fs::read_dir` + rayon (16 MB stack because of
  recursion). On Windows the metadata comes from the directory listing itself:
  no extra call per file. Links/junctions = 0-byte leaves. Result: a flat
  `Vec<Node>` with children sorted by size. The UI asks for views (`view`,
  `children`, `largest_files`) by id; never the whole tree.
- `dupes::find`: size → BLAKE3 of 16 KiB → full BLAKE3, with
  `crp_platform::files::identity` to rule out hard links and
  `is_cloud_placeholder` to avoid reading on-demand files (it would download them).
- Deleting: `crp_platform::recycle_bin::send` (Recycle Bin, `FOF_WANTNUKEWARNING`).
  The app runs `safety::check_target` and blocks Windows / Program Files /
  ProgramData; for duplicates it also refuses to delete every copy in a group.
- `protect::reason` lists what Windows needs and can never go to the Recycle
  Bin from Space or Duplicates: `hiberfil.sys`, `pagefile.sys`, `swapfile.sys`,
  registry hives (`NTUSER.DAT`, `UsrClass.dat`), boot files, `$Recycle.Bin`,
  `System Volume Information`, `Windows.old`, profile and app base folders,
  top-level items of a drive and anything with the *system* attribute. Space
  shows them with a lock and the reason; Duplicates doesn't offer them; the
  backend checks again before deleting.

## Adding a cleaner (typical case: paths)

```rust
// crates/crp-modules/src/dev/caches.rs
e.register(PathRuleModule::new(
    ModuleMeta::new("dev.deno", Category::Dev),
    |ctx| {
        let def = ctx.env.local_app_data.join("deno");
        vec![Target::contents("cache", [ctx.env.var_path("DENO_DIR").unwrap_or(def)])]
    },
));
```

Texts go in `locales/*.json` (see below): `module.dev.deno.name`,
`module.dev.deno.desc`, `module.dev.deno.target.cache` and, optionally,
`module.dev.deno.detail.cache`.

`Target` options: `.names(&["*.log"])`, `.older_than_hours(24)`,
`.max_depth(1)`, `.risk(Risk::Moderate)`, `.admin(true)`, `.label(…)` and
`.detail(…)` for dynamic texts (model names, paths).
`Target::paths(...)` deletes whole paths (e.g. a model).

Custom logic (Recycle Bin, Ollama, projects): implement `CleanerModule`
directly and, if needed, `Action::Custom` + `clean_custom`.

Then add the id to the matching modes in
`crates/crp-core/modes/builtin.toml`, document it in `docs/MODULES.md` and
`docs/MODULES.es.md`, and write a test with `Env::sandbox(tempdir)`.

## Adding a mode

- Built into the binary: a `[[mode]]` block in `crates/crp-core/modes/builtin.toml`.
- User mode: the editor in *Settings → Custom modes*, or a `.toml` with the
  same format in `%APPDATA%\RCleaner\modes\`. Id: lowercase letters, digits,
  `-`, `_`. It can't reuse the id of a built-in mode.

## Desktop app

```
app/src/
  app.css              tokens and shared pieces (see docs/DESIGN.md)
  lib/api.ts           only layer that calls invoke; types mirror Rust
  lib/app.svelte.ts    global state: info, modes, settings, cleanup session
  lib/space.svelte.ts  analyzer: current node, view, delete tray
  lib/dupes.svelte.ts  duplicates: folders, options, auto-selection
  lib/sunburst.ts      geometry and colors of the radial map
  lib/selectors.ts     same selector logic as Rust (mode editor)
  lib/nav.svelte.ts    view history (mouse back/forward, Alt+arrows)
  lib/menu.svelte.ts   global right-click menu
  lib/activity.svelte.ts  path being deleted and delete progress
  views/               Home, Clean, Space, Dupes, Settings, About
  components/          Glyph, ActionButton, Dock, Modal, Sunburst, Treemap,
                       ResultList, ConfirmDialog, DeleteFlow, ContextMenu,
                       Wizard, DeletionLog, ModeEditor, Icon (inline SVG)
app/src-tauri/src/
  state.rs             engine, settings, last scan, disk tree, duplicates,
                       one cancellation token per task
  commands.rs          info, modes, settings, scan/clean, cancel(task), reveal, admin,
                       deletion log, links
  space.rs             disk_* (scan, view, children, crumbs, largest, reveal, delete)
                       and dupes_* (scan, report, reveal, delete)
  tray.rs              tray icon and menu; the window is destroyed on minimize
  background.rs        periodic read-only scan + notification
```

- **Tray and memory**: with "minimize to tray" on, minimizing *destroys* the
  window (WebView2 and its processes exit), drops the disk/duplicate trees and
  trims the working set. If a task is running (`AppState::busy()` guard) the
  window is only hidden. `tray::show` recreates it from `tauri.conf.json`
  (`"create": false`, so `main()` decides whether to show it).
- `--background` (autostart) starts without a window; `--wait-pid` lets a
  relaunch (e.g. as administrator) wait for the old instance to exit.
  `tauri-plugin-single-instance` forwards a second launch to `tray::show`.
- Deleting emits `crp://delete` (done/total/current) and `crp://now` (path
  being removed, every 100 ms, from `CancelToken::current`).

- Heavy work runs in `spawn_blocking`; progress through the events
  `crp://progress`, `crp://disk` (every 150 ms) and `crp://dupes`.
- The folder picker uses `tauri-plugin-dialog` **from Rust** (command
  `pick_folder`): the frontend gets no dialog or file-system permissions.
- The UI only sends **ids** (findings, nodes, duplicate files). `reveal`
  only opens paths the backend knows.

## Languages (`locales/`)

- One JSON per language in `locales/` (repo root). `en.json` is the reference;
  keys missing from another language fall back to English.
- **Rust:** `crp-core/build.rs` embeds every JSON; `crp_core::{t, tf, tn}`
  translate into the active language (`i18n::set_language`). The app sets it at
  startup (saved setting, or the Windows language via `crp_platform::locale`)
  and, when it changes, rebuilds the engine and drops the last scan.
- **UI:** `app/src/lib/i18n.svelte.ts` reads the same files with
  `import.meta.glob`; `t()` reacts to the language.
- `{variable}` gets substituted; `_one`/`_other` for plurals. The test
  `i18n::every_locale_matches_the_reference` checks keys and variables.
- Translator guide: [TRANSLATING.md](TRANSLATING.md).

## Theme

`settings.theme` = `light` / `dark` / `null` (system). The UI sets
`data-theme` on `<html>`; `app.css` defines the light colors both for a light
system theme and for `data-theme="light"`.

## Tests

- Unit tests in every module with `Env::sandbox(tempdir)`: they never touch real paths.
- `cargo test` = core + platform + modules + disk + cli.
- Frontend: `cd app; npm run check` (svelte-check, strict TypeScript) and
  `npm test` (Vitest + jsdom: helpers, complete translations in every
  language, updates and the results list; `invoke` mocked in
  `src/test/tauri.ts`).
