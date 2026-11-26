# Colaborar con RCleaner

[English](CONTRIBUTING.md) · Español

¡Gracias por ayudar! RCleaner borra archivos en computadoras de otras personas,
así que la **seguridad** es lo primero, pero el proceso es sencillo. Al
participar aceptas el [Código de conducta](CODE_OF_CONDUCT.es.md).

## Formas de ayudar

| Quieres… | Haz esto |
|---|---|
| Reportar un error | Abre un issue con la plantilla **Bug report**. Incluye tu versión de Windows, qué pulsaste y qué pasó. |
| Proponer un limpiador o una función | Abre un issue con la plantilla **Feature request**. Para limpiadores, indica qué carpeta limpiaría y cómo se regenera. |
| Traducir | Lee [docs/TRANSLATING.es.md](docs/TRANSLATING.es.md). No hace falta saber Rust ni Svelte. |
| Arreglar o construir algo | Comenta primero en el issue para no duplicar trabajo y luego abre un pull request. |
| Reportar un problema de seguridad | No abras un issue público: sigue [SECURITY.md](SECURITY.md). |

## Entorno de desarrollo

Requisitos: Windows 10/11, Rust stable (MSVC), Node 20+, WebView2 y
`cargo install tauri-cli --version "^2" --locked`.

```powershell
cargo test --workspace --exclude rcleaner   # motor, módulos, disco, idiomas
cargo clippy --workspace --all-targets       # 0 avisos
cargo fmt --all

cd app
npm install
npx svelte-check                             # 0 errores y 0 avisos
cargo tauri dev                              # abre la app
```

Arquitectura: [docs/ARCHITECTURE.es.md](docs/ARCHITECTURE.es.md). Reglas de la UI:
[docs/DESIGN.es.md](docs/DESIGN.es.md).

## Pull requests

1. Rama desde `main` (`fix/…`, `feat/…`, `i18n/<código>`).
2. Un PR, un tema. Los PR pequeños se revisan antes.
3. Antes de subir: `cargo fmt`, `cargo clippy` y `cargo test` sin fallos, y
   `npx svelte-check` limpio si tocaste la UI.
4. Mensajes con [Conventional Commits](https://www.conventionalcommits.org/es/):
   `feat: limpiador de caché de Unity`, `fix(space): …`, `i18n(fr): …`.
5. Añade una línea (en inglés) bajo `## [Unreleased]` en [CHANGELOG.md](CHANGELOG.md)
   si el usuario notará el cambio.
6. Rellena la plantilla del PR. En cambios de UI, las capturas ayudan mucho.

La CI ejecuta las mismas comprobaciones en cada pull request.

## Añadir un limpiador

La mayoría son unas pocas líneas declarativas; mira `crates/crp-modules/src/dev/caches.rs`.

```rust
e.register(PathRuleModule::new(
    ModuleMeta::new("dev.pip", Category::Dev),
    |ctx| vec![Target::contents("cache", [ctx.env.local_app_data.join(r"pip\Cache")])],
));
```

Y añade sus textos a **cada** archivo de `locales/` (como mínimo `en.json`):

```json
"module": { "dev": { "pip": {
  "name": "Caché de pip",
  "desc": "Paquetes descargados por pip…",
  "target": { "cache": "Caché de pip" }
} } }
```

Reglas de seguridad para limpiadores:

- **Analizar nunca modifica el disco.** Sólo `clean` borra.
- Riesgo honesto: *Seguro* sólo si se regenera solo sin coste para el usuario;
  *Moderado* si cuesta descargas o recompilar; *Avanzado* si son datos que el
  usuario puso ahí a propósito.
- Mejor vaciar el contenido de una carpeta que borrar la carpeta, y filtrar por
  antigüedad si la app puede estar usando archivos recientes.
- Si una app bloquea sus archivos, declara su ejecutable con
  `.closes(&["app.exe"])` para avisar al usuario.
- Nunca tocar credenciales, cookies, historial, documentos ni ajustes.
- Añade un test con un `Env` de prueba (mira los existentes) y el limpiador a
  [docs/MODULES.es.md](docs/MODULES.es.md) y [docs/MODULES.md](docs/MODULES.md).

## Estilo de código

- Rust 2024, el `rustfmt.toml` del repo, nada de `unsafe` fuera de `crp-platform`.
- Svelte 5 con runas y TypeScript; nada de dependencias nuevas de UI sin hablarlo.
- Todo texto que vea el usuario va **siempre** por `locales/` (`t("…")` en Rust y
  en Svelte). Nunca una frase fija en el código.
- Los comentarios pueden ir en español o inglés; los nuevos, mejor en inglés.
