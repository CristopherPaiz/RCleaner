# RCleaner — Plan por fases

[English](ROADMAP.md) · Español

Limpiador y mantenimiento para Windows escrito en Rust. Une lo que hacían
CCleaner / PC Manager (limpieza), WinDirStat (mapa de disco) y
Bulk Crap Uninstaller (desinstalador masivo), con **modos** pensados por perfil
de usuario (rápido, programador, IA, gamer, profundo).

## Decisiones de stack

| Tema | Elección | Por qué |
|---|---|---|
| Lenguaje | Rust 2024 (stable) | Rápido, binario único, sin GC, seguro en memoria. |
| GUI | **Tauri 2** + Svelte 5 + Vite | Binario ~10 MB (usa WebView2 del sistema), UI moderna, genera el instalador **NSIS** con un comando. Tauri 3 sigue en alpha: no se usa. |
| Paralelismo | `rayon` | Escaneo de módulos y medición de carpetas en paralelo. |
| Recorrido FS | `walkdir` (sin seguir enlaces) | No entra en junctions/symlinks → nunca borra fuera de la ruta. |
| Win32 | `windows-sys` | FFI mínima: admin, papelera, (luego) registro, puntos de restauración. |
| Registro | `winreg` (fase 4-5) | Lectura de desinstaladores y limpieza de claves con respaldo `.reg`. |
| Hash duplicados | `blake3` | Tamaño → hash de 16 KiB → hash completo. BLAKE3 con SIMD ya es más rápido que el disco; un segundo algoritmo no aporta. |
| CLI | `clap` | Automatizable, scriptable y base de pruebas. |

## Principios (no negociables)

1. **Primero vista previa.** Escanear nunca borra. Limpiar exige confirmación (`--yes` en CLI, diálogo en GUI).
2. **Nunca seguir enlaces.** Junctions/symlinks se tratan como hojas.
3. **Lista de rutas protegidas.** Raíces de disco, `C:\Windows`, `System32`, `WinSxS`, perfil de usuario, `Program Files`, etc. jamás son objetivo.
4. **Riesgo explícito.** Cada hallazgo es `Seguro`, `Moderado` o `Avanzado`. Sólo los seguros se preseleccionan.
5. **Admin bajo demanda.** La app corre como usuario; lo que requiere admin se marca y se omite si no hay elevación.
6. **Respaldos** antes de tocar registro (export `.reg`) y punto de restauración opcional.
7. **Módulos declarativos.** Añadir un limpiador = declarar rutas; la lógica de borrado/medición es común.

## Fases

### Fase 0 — Cimientos ✅
- Workspace Cargo: `crp-core`, `crp-platform`, `crp-modules`, `crp-cli`.
- Motor: registro de módulos, escaneo paralelo, ejecución, progreso, cancelación, guardas de seguridad.
- Modos embebidos (TOML), selectores `dev.*`.
- Docs: levantar proyecto, arquitectura, builds.

### Fase 1 — Limpieza base + modos ✅
- **Sistema:** temporales usuario/Windows, miniaturas, WER, volcados, logs de Windows, cachés de shaders (DirectX/NVIDIA/AMD/Intel), Delivery Optimization, descargas de Windows Update, papelera.
- **Navegadores (sólo caché):** Chrome, Edge, Brave, Vivaldi, Opera/GX, Chromium, Firefox.
- **Programador:** pip, uv, poetry, conda (tarballs), npm, pnpm, yarn, bun, cargo, gradle, maven, NuGet, Go, VS Code/Cursor, JetBrains; artefactos de proyectos (`node_modules`, `target`, `__pycache__`, `.venv`, `.next`…).
- **IA:** modelos Ollama (borrado de capas no compartidas), Hugging Face hub, LM Studio, Torch/Whisper/Keras.
- CLI `rcl`: `modes`, `modules`, `scan`, `clean`, `info`.

### Fase 2 — App de escritorio ✅
- Tauri 2 + Svelte 5. Vistas **Inicio** (discos, accesos a modos, historial y total liberado), **Limpiar** (progreso en vivo, filtro, orden por tamaño/antigüedad, selección rápida Seguros/Todo/Nada, avisos, confirmación, resumen con errores) y **Ajustes**.
- Ajustes persistentes (`%APPDATA%\RCleaner\settings.json`): carpetas de proyectos (auto o manual), **exclusiones** (respetadas al medir y al borrar), modo al abrir.
- **Modos personalizados** con editor visual (o duplicando uno incluido) → `%APPDATA%\RCleaner\modes\*.toml`. La CLI también los ve.
- **Aviso de programas abiertos** (navegadores, editores, IDEs JetBrains, Ollama, LM Studio) vía Toolhelp32.
- **Reiniciar como administrador** (UAC) desde la app.
- Historial de limpiezas (`history.json`, últimas 500).
- Builds **MSI** + **NSIS (.exe)**, icono, versión única en `Cargo.toml`. Firma opcional documentada.
- Pendiente menor: firma de código real (requiere certificado), instancia única, bandeja del sistema.

### Fase 3 — Espacio + duplicados + nueva UI ✅
- **Nueva interfaz** (ver [DESIGN.md](DESIGN.es.md)): riel de iconos, una pantalla por herramienta con glifo, botón circular y color propio; resultados en filas por categoría.
- Crate `crp-disk`:
  - **Analizador de espacio:** recorrido paralelo (rayon, sin seguir enlaces/uniones), árbol compacto en memoria, vistas recortadas por niveles, top de archivos grandes, tipos por extensión. Archivos de la nube sin copia local cuentan 0 bytes. C:\ completo (≈3 M archivos) en ~35 s sin admin.
  - **Mapa radial** (sunburst, estilo DaisyDisk) navegable + lista de la carpeta + "bandeja" de elementos a eliminar.
  - **Duplicados:** tamaño → BLAKE3 de 16 KiB → BLAKE3 completo; descarta enlaces duros; nunca lee archivos "a petición" de OneDrive/Drive (no los descarga); omite por defecto AppData, `node_modules`, `.git`, Windows, Program Files…; selección automática (conservar el más antiguo/reciente/ruta más corta). Nunca permite borrar todas las copias.
- Todo lo eliminado desde Espacio/Duplicados va a la **papelera** (`SHFileOperationW` + aviso si algo no cabe) y pasa por `safety::check_target` + bloqueo de Windows/Program Files/ProgramData.
- CLI: `rcl space <ruta>` y `rcl dupes <rutas…>`.
- Pendiente: lectura de la MFT (más rápido con admin), treemap alternativo, reemplazar duplicados por enlaces duros.

### Fase 3.5 — Proyecto abierto ✅
- **Idiomas:** español e inglés (por defecto el de Windows) en app, motor y CLI, desde `locales/*.json`; añadir uno es añadir un archivo.
- **Tema** claro / oscuro / sistema. Ventana sin marco con botones propios.
- Documentación para colaborar: README (en/es), CONTRIBUTING, código de conducta, SECURITY, guía de traducción, plantillas de issues/PR y CI en GitHub Actions.

### Fase 4 — Desinstalador (tipo BCUninstaller) + herramientas dev
- Inventario: `HKLM/HKCU\...\Uninstall` (+ WOW6432Node), MSIX/Store (`Get-AppxPackage`), winget.
- Desinstalación masiva (silenciosa cuando haya `QuietUninstallString`/MSI `/qn`), cola secuencial.
- Restos: carpetas en Program Files/AppData, claves huérfanas.
- Dev: paquetes globales npm/pip sin uso, `wsl --list` / `wsl --unregister` (+ compactar VHDX), `docker system prune`, modelos IA por antigüedad de uso.

### Fase 5 — Registro (con respaldo)
- Entradas de desinstalación huérfanas, MUICache/rutas inexistentes, extensiones sin programa, inicio apuntando a archivos borrados, DLL compartidas faltantes.
- Export `.reg` previo y restauración desde la app. Punto de restauración (`SystemRestore.CreateRestorePoint`).

### Fase 6 — Mantenimiento
- Programas de inicio, tareas programadas, servicios.
- Limpieza programada (Task Scheduler), `DISM /StartComponentCleanup`, `Windows.old`.
- Salud de discos, firma de código, telemetría cero.

## Qué NO se hará (a propósito)
- Borrar **Prefetch**: empeora el arranque; Windows lo gestiona.
- "Optimizadores" de RAM o del registro tipo placebo.
- Borrar cookies/historial sin opción explícita (privacidad = módulo aparte, opt-in).
