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

### Security

- Files Windows needs (hiberfil.sys, pagefile.sys, swapfile.sys, the registry,
  the Recycle Bin, Windows.old, base folders and system files) can never be
  deleted from Space or Duplicates.

[Unreleased]: https://github.com/CristopherPaiz/RCleaner/commits/main
