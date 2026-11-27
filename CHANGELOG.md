# Changelog

All notable changes to RCleaner are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

Add every user-facing change to **Unreleased** as you make it. When a version
is released, `scripts/release.ps1` moves those notes under the new version and
GitHub publishes them as the release notes (see
[docs/BUILD.md](docs/BUILD.md#releases)).

Sections: `Added`, `Changed`, `Fixed`, `Removed`, `Security`.

## [Unreleased]

### Added

- **Startup** tab: programs that open when you sign in, scheduled tasks and
  third-party services. Turn them off or on (or set a service to manual)
  without deleting anything, the same way Task Manager does.
- Cleanup: **Previous Windows** (`Windows.old` and update leftovers, usually
  10–30 GB) and **Superseded Windows updates** (WinSxS, with Windows' DISM).
  Both need administrator and are never preselected.
- README: an **"Is it safe?"** section explaining what RCleaner will and will
  never touch.

### Changed

- New **Overview**, in the style of Windows 11's Storage settings: the system
  drive with what can be freed safely marked on its bar, a quick clean button,
  plain suggestion rows only when there's something to do (low disk space,
  previous Windows, a full Recycle Bin, many startup programs, admin-only
  space, updates) and your other drives. No score, activity feed or chart.

## [0.1.3] - 2026-11-27

### Added

- **Exclusions** tab in the sidebar: every excluded folder with its size,
  plus add, remove and show in Explorer.
- Space: **Analyze again** button that rescans and returns to the folder you
  were in, and a visible **Change drive** menu.
- Space › Types: clicking a file type lists the largest files of that type.
- Deletion log: choose how many entries to keep (or none) and clear it; both
  ask for confirmation before deleting entries.
- **Automatic updates**: RCleaner checks GitHub for a new version on start
  (can be turned off in Settings › About) and installs it with one click;
  the portable version links to the download instead.
- The installer is in **English** too: a single setup that asks which
  language to use.
- Optional **browser cookies and history** (Settings › Security). Off by
  default, never marked automatically; passwords, bookmarks and forms are
  never touched.
- The window remembers its size, position and maximized state, also when it
  comes back from the tray.

### Changed

- Hovering the radial chart or the treemap highlights that item in the list
  and dims the rest.
- Cleanup shows which drives it looks at and how much is selected on each.
- Cleanup groups browsers: each browser is one entry with its own list of
  options, which can be marked or collapsed together.
- "All" and group checkboxes never mark user data (cookies, history, models).

### Removed

- The MSI installer: the setup (`…-setup.exe`) covers every language in one
  file. If you installed an MSI version, uninstall it before installing this
  one (Settings › Apps), or you'll have two copies.

### Fixed

- Menus opened near the right edge of the window were cut off.
- Home: the total freed showed the plural text for a single cleanup's key.

## [0.1.2] - 2026-11-27

### Added

- Portable `RCleaner_<version>_x64-portable.exe` in every release: runs
  without installing (uses the WebView2 that ships with Windows 10/11).

## [0.1.1] - 2026-11-26

### Changed

- Installers are now a few MB instead of ~285 MB: they no longer carry the
  full WebView2 installer. WebView2 already ships with Windows 10/11; if it is
  missing, the installer downloads it.

## [0.1.0] - 2026-11-26

### Added

- Cleaning engine with parallel scanning, safe deletion (Recycle Bin or
  permanent), TOML cleaning modes, settings and history.
- 37 cleaners: Windows system, browsers, developer tools and AI tools.
- `rcl` command-line interface.
- Desktop app (Tauri 2 + Svelte 5) with a frameless window, light/dark/system
  theme and English/Spanish translations.
- **Space**: disk analyzer with a radial chart and a WinDirStat-style treemap.
- **Duplicates**: duplicate file finder (BLAKE3).
- Windows file, drive and app icons, plus Material Icon Theme icons for
  well-known development folders.
- MSI and EXE installers that bundle WebView2 and the C runtime, so they work
  on a clean Windows install.
- New **Home**: health score with checks, what can be cleaned by category,
  drives, recent activity, space freed per week and a quick clean button.
- First-run wizard (language, theme, behaviour) and a loading screen.
- System tray: minimizing closes the window and leaves RCleaner in the tray
  using a few MB of memory; reopening restores it.
- Background mode: analyzes periodically (read-only) and shows a Windows
  notification when the safe junk passes a threshold. Optional start with
  Windows.
- Single instance: opening RCleaner again brings up the running window.
- Option to always run as administrator.
- Right-click menus everywhere (rows, charts, drives, text fields) instead of
  the WebView menu: open, show in Explorer, copy path, add to or remove from
  exclusions, mark for deletion.
- Excluded folders are shown in their own colour in Space and the results.
- Deleting shows the path being removed and its progress; after a cleanup the
  list of what was removed can be expanded.
- Space and Duplicates ask on **Continue** whether to use the Recycle Bin or
  delete permanently.
- Deletion log (`deleted.log`) with everything RCleaner removed, viewable from
  Settings and Home.
- Mouse back/forward buttons and Alt+Left/Right navigate between views.
- **About** page with project links and contributors.

### Changed

- "N more" in confirmation dialogs is clickable and shows the full list.
- The radial chart and long folder paths adapt to small windows without
  overlapping other elements.

### Security

- Files Windows needs (hiberfil.sys, pagefile.sys, swapfile.sys, the registry,
  the Recycle Bin, Windows.old, base folders and system files) can never be
  deleted from Space or Duplicates.
- System files are shown but locked even as administrator. They can only be
  unlocked with the "Allow system files" setting, and every such selection
  asks the user to accept responsibility first.
- `#![forbid(unsafe_code)]` in every crate except `crp-platform`, the only one
  that calls the Windows API.

[Unreleased]: https://github.com/CristopherPaiz/RCleaner/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/CristopherPaiz/RCleaner/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/CristopherPaiz/RCleaner/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/CristopherPaiz/RCleaner/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/CristopherPaiz/RCleaner/releases/tag/v0.1.0
