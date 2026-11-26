# Arquitectura

```
          ┌──────────────┐     ┌──────────────────────┐
          │   crp-cli    │     │  app (Tauri + Svelte) │
          └──────┬───────┘     └──────────┬───────────┘
                 │  Engine::scan / clean   │ comandos + eventos
                 ▼                         ▼
          ┌─────────────────────────────────────────┐
          │              crp-modules                 │  catálogo de limpiadores
          │  system · browsers · dev · ai · (…)      │
          └──────────────┬──────────────────────────┘
                         │ implementan CleanerModule / PathRuleModule
          ┌──────────────▼──────────────┐   ┌────────────────┐
          │           crp-core          │   │  crp-platform  │
          │ modelo · motor · fsutil ·   │   │ Win32 (unsafe) │
          │ safety · modos · progreso   │   └────────────────┘
          └─────────────────────────────┘
```

Regla de dependencias: `core` no depende de nadie del proyecto; `platform`
tampoco; `modules` usa ambos; CLI y app sólo usan `modules`/`core`.

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
| `Settings`, `AppPaths`, historial | `core::settings` | `%APPDATA%\CleanCRP\{settings.json, history.json, modes\}` con escritura atómica |

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

## Añadir un limpiador (caso típico: rutas)

```rust
// crates/crp-modules/src/dev/caches.rs
e.register(PathRuleModule::new(
    ModuleMeta::new("dev.deno", "Caché de Deno", Category::Dev)
        .describe("Módulos remotos y compilados de Deno."),
    |ctx| {
        let def = ctx.env.local_app_data.join("deno");
        vec![Target::contents("cache", "Caché de Deno", [ctx.env.var_path("DENO_DIR").unwrap_or(def)])]
    },
));
```

Opciones de `Target`: `.names(&["*.log"])`, `.older_than_hours(24)`,
`.max_depth(1)`, `.risk(Risk::Moderate)`, `.admin(true)`, `.detail("…")`.
`Target::paths(...)` borra rutas completas (p. ej. un modelo).

Lógica a medida (papelera, Ollama, proyectos): implementar `CleanerModule`
directamente y, si hace falta, `Action::Custom` + `clean_custom`.

Después: añadir el id a los modos que correspondan en
`crates/crp-core/modes/builtin.toml`, documentarlo en `docs/MODULES.md` y
escribir un test con `Env::sandbox(tempdir)`.

## Añadir un modo

- Incluido en el binario: bloque `[[mode]]` en `crates/crp-core/modes/builtin.toml`.
- Del usuario: editor en *Ajustes → Modos personalizados*, o un `.toml` con el
  mismo formato en `%APPDATA%\CleanCRP\modes\`. Id: minúsculas, números, `-`, `_`.
  No puede reutilizar el id de un modo incluido.

## App de escritorio

```
app/src/
  lib/api.ts           única capa que llama a invoke; tipos espejo de crp-core
  lib/app.svelte.ts    estado global (runas): info, modos, ajustes, sesión de limpieza
  lib/selectors.ts     misma lógica de selectores que Rust (editor de modos)
  views/Home.svelte    discos, accesos a modos, historial
  views/Clean.svelte   análisis, filtro/orden, selección, limpieza, resumen
  views/Settings.svelte carpetas, exclusiones, modo al abrir, modos propios
  components/          ResultList, ConfirmDialog, ModeEditor, Icon (SVG en línea)
app/src-tauri/src/
  state.rs             motor, ajustes, último escaneo, token de cancelación
  commands.rs          system_info, drives, history, list/save/delete_mode,
                       list_modules, get/save_settings, pick_folder, scan, clean,
                       cancel, reveal, restart_as_admin
```

- Trabajo pesado en `spawn_blocking`; progreso por el evento `crp://progress`.
- El selector de carpetas usa `tauri-plugin-dialog` **desde Rust** (comando
  `pick_folder`): el frontend no recibe permisos de diálogo ni de sistema de archivos.
- `reveal` sólo abre rutas del último escaneo, carpetas de proyectos o la de ajustes.

## Pruebas

- Unitarias en cada módulo con `Env::sandbox(tempdir)`: nunca tocan rutas reales.
- `cargo test` = core + platform + modules + cli.
- Frontend: `cd app; npm run check` (svelte-check, TypeScript estricto).
