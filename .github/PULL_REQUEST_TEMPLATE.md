## What and why / Qué y por qué

<!-- Link the issue if there is one: "Closes #123" / Enlaza el issue si existe -->

## How it was tested / Cómo se probó

- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace --all-targets` (0 warnings)
- [ ] `cargo test --workspace --exclude rcleaner`
- [ ] `npx svelte-check` in `app/` (if the UI changed / si cambió la UI)
- [ ] Tried it in the app (`cargo tauri dev`) / Probado en la app

## Checklist

- [ ] User-facing text is in `locales/en.json` and `locales/es.json`, not hard-coded. / Los textos están en `locales/`, no fijos en el código.
- [ ] New cleaners: honest risk level, a test and an entry in `docs/MODULES.md` + `docs/MODULES.es.md`. / Limpiadores nuevos: riesgo honesto, test y entrada en `docs/MODULES.es.md` + `docs/MODULES.md`.
- [ ] Screenshots for UI changes. / Capturas si cambia la UI.
- [ ] User-facing changes noted under `## [Unreleased]` in `CHANGELOG.md`. / Cambios visibles anotados en `CHANGELOG.md`.
