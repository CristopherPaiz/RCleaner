# Contributing to RCleaner

English · [Español](CONTRIBUTING.es.md)

Thanks for helping! RCleaner deletes files on people's computers, so the bar
for **safety** is high, but the process is simple. By taking part you agree to
follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Ways to help

| You want to… | Do this |
|---|---|
| Report a bug | Open an issue with the **Bug report** template. Include your Windows version, what you clicked and what happened. |
| Suggest a cleaner or a feature | Open an issue with the **Feature request** template. For cleaners, say which folder it would clean and how it is rebuilt. |
| Translate | Read [docs/TRANSLATING.md](docs/TRANSLATING.md). No Rust or Svelte knowledge needed. |
| Fix or build something | Comment on the issue first so work isn't duplicated, then open a pull request. |
| Report a security problem | Don't open a public issue: follow [SECURITY.md](SECURITY.md). |

## Development setup

Requirements: Windows 10/11, Rust stable (MSVC), Node 20+, WebView2 and
`cargo install tauri-cli --version "^2" --locked`.

```powershell
cargo test --workspace --exclude rcleaner   # engine, modules, disk, i18n
cargo clippy --workspace --all-targets       # must have 0 warnings
cargo fmt --all

cd app
npm install
npx svelte-check                             # must have 0 errors and 0 warnings
cargo tauri dev                              # run the app
```

Architecture overview: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). UI rules:
[docs/DESIGN.md](docs/DESIGN.md).

## Pull requests

1. Branch from `main` (`fix/…`, `feat/…`, `i18n/<code>`).
2. Keep each PR about one thing. Small PRs get reviewed faster.
3. Before pushing: `cargo fmt`, `cargo clippy` and `cargo test` clean, and
   `npx svelte-check` clean if you touched the UI.
4. Use [Conventional Commits](https://www.conventionalcommits.org/) for
   messages: `feat: add Unity cache cleaner`, `fix(space): …`, `i18n(fr): …`.
5. Fill in the PR template. Screenshots help a lot for UI changes.

CI runs the same checks on every pull request.

## Adding a cleaner

Most cleaners are a few declarative lines; see `crates/crp-modules/src/dev/caches.rs`.

```rust
e.register(PathRuleModule::new(
    ModuleMeta::new("dev.pip", Category::Dev),
    |ctx| vec![Target::contents("cache", [ctx.env.local_app_data.join(r"pip\Cache")])],
));
```

Then add its texts to **every** file in `locales/` (at least `en.json`):

```json
"module": { "dev": { "pip": {
  "name": "pip cache",
  "desc": "Packages downloaded by pip…",
  "target": { "cache": "pip cache" }
} } }
```

Safety rules for cleaners:

- **Scanning must never modify the disk.** Only `clean` deletes.
- Pick the honest risk: *Safe* only if it rebuilds itself with no cost to the
  user; *Moderate* if it costs downloads or rebuilds; *Advanced* if it is data
  the user put there on purpose.
- Prefer cleaning a folder's contents over deleting the folder itself, and
  filter by age when the app might be using recent files.
- If an app locks its files, declare its executable with `.closes(&["app.exe"])`
  so the user is warned.
- Never touch credentials, cookies, history, documents or settings.
- Add a test with a sandboxed `Env` (see the existing tests) and add the cleaner
  to [docs/MODULES.md](docs/MODULES.md) and
  [docs/MODULES.es.md](docs/MODULES.es.md).

## Code style

- Rust 2024, `rustfmt.toml` in the repo, no `unsafe` outside `crp-platform`.
- Svelte 5 runes and TypeScript; no new UI dependencies without discussing it.
- User-facing text **always** goes through `locales/` (`t("…")` in Rust and
  Svelte). Never hard-code a sentence in the code.
- Code comments may be in English or Spanish; new ones are best in English.
