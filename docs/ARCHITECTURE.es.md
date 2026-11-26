# Arquitectura

[English](ARCHITECTURE.md) · Español

```
          ┌──────────────┐     ┌──────────────────────┐
          │   crp-cli    │     │  app (Tauri + Svelte) │
          └──────┬───────┘     └──────────┬───────────┘
                 │  Engine::scan / clean   │ comandos + eventos
                 ▼                         ▼
          ┌─────────────────────────────┐  ┌──────────────────┐
          │         crp-modules          │  │     crp-disk     │
          │ system · browsers · dev · ai │  │ espacio · dupes  │
          └──────────────┬──────────────┘  └────────┬─────────┘
                         │ implementan CleanerModule / PathRuleModule
          ┌──────────────▼──────────────┐   ┌────────────────┐
          │           crp-core          │   │  crp-platform  │
          │ modelo · motor · fsutil ·   │   │ Win32 (unsafe) │
          │ safety · modos · progreso   │   └────────────────┘
          └─────────────────────────────┘
```

Regla de dependencias: `core` no depende de nadie del proyecto; `platform`
tampoco; `modules` y `disk` usan ambos; CLI y app usan `modules`, `disk` y `core`.

## Conceptos

| Tipo | Dónde | Qué es |
|---|---|---|
| `ModuleMeta` | `core::module` | id (`categoria.nombre`), nombre, descripción, categoría, riesgo por defecto, si requiere admin, procesos que bloquean (`.closes(&["chrome.exe"])`) |
| `CleanerModule` | `core::module` | `scan(ctx) -> Vec<Finding>` (sin efectos) y opcional `clean_custom` |
| `Finding` | `core::model` | algo limpiable: etiqueta, bytes, elementos, riesgo, `selected`, `age_days`, `warning`, `Action` |
| `Action` | `core::model` | `DeleteContents{roots, filter}`, `DeletePaths`, `RunCommand`, `Custom` |
| `Engine` | `core::engine` | registro de módulos, `scan` paralelo (rayon), `clean`; añade avisos si un proceso del módulo está abierto |
| `Mode` | `core::modes` + `modes/builtin.toml` | lista de selectores (`dev.*`, `!ai.*`, `*`); `modes::all/save_user/delete_user` para los del usuario |
| `ScanContext` | `core::context` | `Env` (rutas), cancelación, admin, carpetas de proyectos, **exclusiones**, procesos en marcha |
| `Settings`, `AppPaths`, historial | `core::settings` | `%APPDATA%\RCleaner\{settings.json, history.json, modes\}` con escritura atómica |

## Seguridad del borrado

1. **Escanear no escribe.** Sólo `Engine::clean` borra.
2. `safety::check_target` rechaza raíces de disco, carpetas base del usuario,
   `Program Files`, `ProgramData`, `C:\Windows`, y todo dentro de
   `System32`/`SysWOW64`/`WinSxS`/`servicing`/`Boot`. Se aplica a **cada**
   raíz antes de borrar, aunque el módulo se equivoque.
3. `walkdir` con `follow_links(false)`: symlinks y junctions se eliminan como
   enlace, nunca se recorre su destino (test `never_follows_symlinked_dirs`).
4. Archivos en uso → se omiten y se cuentan (`skipped_items`), no abortan.
5. Sólo lectura → se quita el atributo y se reintenta (p. ej. módulos de Go).
6. La GUI sólo envía **ids** de hallazgos al backend; las rutas las decide Rust
   a partir de su último escaneo.
7. Hallazgos `requires_admin` se omiten si el proceso no está elevado.
8. Un `panic` en un módulo se captura y se reporta como error de ese módulo.
9. **Exclusiones** del usuario: `Target::into_finding` descarta raíces excluidas
   y rutas completas que contengan una exclusión; las exclusiones dentro de una
   carpeta a vaciar viajan en `EntryFilter.exclude` y el recorrido no entra en
   ellas ni al medir ni al borrar. (Nota: el recorrido es en pre-orden; walkdir
   con `contents_first` + `filter_entry` salta hermanos.)
10. Espacio/Duplicados pasan por `deletable()`: `check_target` siempre aplica;
    `protect::reason` (archivos de sistema) y Program Files/Windows sólo se
    levantan si el usuario activa `allow_system`, y la UI pide confirmación en
    cada selección así. Las exclusiones nunca se pueden borrar.
11. Cada borrado se añade a `deleted.log` (líneas JSON, se recorta pasados
    4 MB) con origen, ruta, tamaño y si fue permanente.
12. `unsafe` está prohibido (`#![forbid(unsafe_code)]`) en todo salvo
    `crp-platform`, que envuelve la API de Windows.

## Espacio y duplicados (`crp-disk`)

- `tree::scan` recorre con `std::fs::read_dir` + rayon (pila de 16 MB por la
  recursión). En Windows los metadatos salen del propio listado: sin llamadas
  extra por archivo. Enlaces/uniones = hojas de 0 bytes. Resultado: `Vec<Node>`
  plano con hijos ordenados por tamaño. La UI pide vistas (`view`, `children`,
  `largest_files`) por id; nunca el árbol entero.
- `dupes::find`: tamaño → BLAKE3 de 16 KiB → BLAKE3 completo, con
  `crp_platform::files::identity` para descartar enlaces duros y
  `is_cloud_placeholder` para no leer archivos "a petición" (los descargaría).
- Borrado: `crp_platform::recycle_bin::send` (papelera, `FOF_WANTNUKEWARNING`).
  La app comprueba `safety::check_target` y bloquea Windows / Program Files /
  ProgramData; en duplicados, además, rechaza borrar todas las copias de un grupo.
- `protect::reason` define lo que Windows necesita y nunca va a la papelera desde
  Espacio o Duplicados: `hiberfil.sys`, `pagefile.sys`, `swapfile.sys`, el
  registro del usuario (`NTUSER.DAT`, `UsrClass.dat`), archivos de arranque,
  `$Recycle.Bin`, `System Volume Information`, `Windows.old`, carpetas base de
  perfiles y apps, elementos de primer nivel de una unidad y todo lo que tenga
  el atributo *sistema*. Espacio los muestra con candado y el motivo;
  Duplicados no los ofrece; el backend vuelve a comprobarlo al borrar.

## Añadir un limpiador (caso típico: rutas)

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

Los textos van en `locales/*.json` (ver más abajo): `module.dev.deno.name`,
`module.dev.deno.desc`, `module.dev.deno.target.cache` y, opcional,
`module.dev.deno.detail.cache`.

Opciones de `Target`: `.names(&["*.log"])`, `.older_than_hours(24)`,
`.max_depth(1)`, `.risk(Risk::Moderate)`, `.admin(true)`, `.label(…)` y
`.detail(…)` para textos dinámicos (nombres de modelos, rutas).
`Target::paths(...)` borra rutas completas (p. ej. un modelo).

Lógica a medida (papelera, Ollama, proyectos): implementar `CleanerModule`
directamente y, si hace falta, `Action::Custom` + `clean_custom`.

Después: añadir el id a los modos que correspondan en
`crates/crp-core/modes/builtin.toml`, documentarlo en `docs/MODULES.md` y `docs/MODULES.es.md`, y
escribir un test con `Env::sandbox(tempdir)`.

## Añadir un modo

- Incluido en el binario: bloque `[[mode]]` en `crates/crp-core/modes/builtin.toml`.
- Del usuario: editor en *Ajustes → Modos personalizados*, o un `.toml` con el
  mismo formato en `%APPDATA%\RCleaner\modes\`. Id: minúsculas, números, `-`, `_`.
  No puede reutilizar el id de un modo incluido.

## App de escritorio

```
app/src/
  app.css              tokens y piezas comunes (ver docs/DESIGN.es.md)
  lib/api.ts           única capa que llama a invoke; tipos espejo de Rust
  lib/app.svelte.ts    estado global: info, modos, ajustes, sesión de limpieza
  lib/space.svelte.ts  analizador: nodo actual, vista, bandeja de borrado
  lib/dupes.svelte.ts  duplicados: carpetas, opciones, selección automática
  lib/sunburst.ts      geometría y colores del mapa radial
  lib/selectors.ts     misma lógica de selectores que Rust (editor de modos)
  lib/nav.svelte.ts    historial de vistas (botones del ratón, Alt+flechas)
  lib/menu.svelte.ts   menú contextual global
  lib/activity.svelte.ts  ruta que se borra y progreso del borrado
  views/               Home, Clean, Space, Dupes, Settings, About
  components/          Glyph, ActionButton, Dock, Modal, Sunburst, Treemap,
                       ResultList, ConfirmDialog, DeleteFlow, ContextMenu,
                       Wizard, DeletionLog, ModeEditor, Icon (SVG en línea)
app/src-tauri/src/
  state.rs             motor, ajustes, último escaneo, árbol de disco, duplicados,
                       un token de cancelación por tarea
  commands.rs          info, modos, ajustes, scan/clean, cancel(task), reveal, admin,
                       registro de borrados, enlaces
  space.rs             disk_* (scan, view, children, crumbs, largest, reveal, delete)
                       y dupes_* (scan, report, reveal, delete)
  tray.rs              icono y menú de la bandeja; al minimizar se destruye la ventana
  background.rs        análisis periódico de sólo lectura + notificación
```

- **Bandeja y memoria**: con "minimizar a la bandeja", minimizar *destruye* la
  ventana (WebView2 y sus procesos se cierran), suelta los árboles de disco y
  duplicados y recorta la memoria. Si hay una tarea en curso (guarda
  `AppState::busy()`) sólo se oculta. `tray::show` la vuelve a crear desde
  `tauri.conf.json` (`"create": false`, así `main()` decide si mostrarla).
- `--background` (inicio con Windows) arranca sin ventana; `--wait-pid` deja
  que un relanzamiento (p. ej. como administrador) espere a que salga la
  instancia anterior. `tauri-plugin-single-instance` manda un segundo arranque
  a `tray::show`.
- Borrar emite `crp://delete` (hecho/total/actual) y `crp://now` (ruta que se
  está borrando, cada 100 ms, desde `CancelToken::current`).

- Trabajo pesado en `spawn_blocking`; progreso por eventos `crp://progress`,
  `crp://disk` (cada 150 ms) y `crp://dupes`.
- El selector de carpetas usa `tauri-plugin-dialog` **desde Rust** (comando
  `pick_folder`): el frontend no recibe permisos de diálogo ni de sistema de archivos.
- La UI sólo envía **ids** (hallazgos, nodos, archivos duplicados). `reveal`
  sólo abre rutas conocidas por el backend.

## Idiomas (`locales/`)

- Un JSON por idioma en `locales/` (raíz). `en.json` es la referencia; las
  claves que falten en otro idioma caen a inglés.
- **Rust:** `crp-core/build.rs` incrusta todos los JSON; `crp_core::{t, tf, tn}`
  traducen en el idioma activo (`i18n::set_language`). La app lo fija al iniciar
  (ajuste guardado o idioma de Windows vía `crp_platform::locale`) y, al
  cambiarlo, reconstruye el motor y descarta el último análisis.
- **UI:** `app/src/lib/i18n.svelte.ts` lee los mismos archivos con
  `import.meta.glob`; `t()` es reactivo al idioma.
- `{variable}` se sustituye; `_one`/`_other` para plurales. El test
  `i18n::every_locale_matches_the_reference` valida claves y variables.
- Guía para traductores: [TRANSLATING.md](TRANSLATING.es.md).

## Tema

`settings.theme` = `light` / `dark` / `null` (sistema). La UI pone
`data-theme` en `<html>`; `app.css` define los colores claros para el sistema
claro y para `data-theme="light"`.

## Pruebas

- Unitarias en cada módulo con `Env::sandbox(tempdir)`: nunca tocan rutas reales.
- `cargo test` = core + platform + modules + disk + cli.
- Frontend: `cd app; npm run check` (svelte-check, TypeScript estricto).
