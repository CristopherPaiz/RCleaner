# Clean-CRP — Plan por fases

Limpiador y mantenimiento para Windows escrito en Rust. Une lo que hacían
CCleaner / PC Manager (limpieza), WinDirStat (mapa de disco) y
Bulk Crap Uninstaller (desinstalador masivo), con **modos** pensados por perfil
de usuario (rápido, programador, IA, gamer, profundo).

## Decisiones de stack

| Tema | Elección | Por qué |
|---|---|---|
| Lenguaje | Rust 2024 (stable) | Rápido, binario único, sin GC, seguro en memoria. |
| GUI | **Tauri 2** + Svelte 5 + Vite | Binario ~10 MB (usa WebView2 del sistema), UI moderna, genera **MSI (WiX)** y **EXE (NSIS)** con un comando. Tauri 3 sigue en alpha: no se usa. |
| Paralelismo | `rayon` | Escaneo de módulos y medición de carpetas en paralelo. |
| Recorrido FS | `walkdir` (sin seguir enlaces) | No entra en junctions/symlinks → nunca borra fuera de la ruta. |
| Win32 | `windows-sys` | FFI mínima: admin, papelera, (luego) registro, puntos de restauración. |
| Registro | `winreg` (fase 4-5) | Lectura de desinstaladores y limpieza de claves con respaldo `.reg`. |
| Hash duplicados | `xxhash` (rápido) + `blake3` (confirmación) | Tamaño → hash parcial → hash completo. |
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
- CLI `crp`: `modes`, `modules`, `scan`, `clean`, `info`.

### Fase 2 — App de escritorio ✅
- Tauri 2 + Svelte 5. Vistas **Inicio** (discos, accesos a modos, historial y total liberado), **Limpiar** (progreso en vivo, filtro, orden por tamaño/antigüedad, selección rápida Seguros/Todo/Nada, avisos, confirmación, resumen con errores) y **Ajustes**.
- Ajustes persistentes (`%APPDATA%\CleanCRP\settings.json`): carpetas de proyectos (auto o manual), **exclusiones** (respetadas al medir y al borrar), modo al abrir.
- **Modos personalizados** con editor visual (o duplicando uno incluido) → `%APPDATA%\CleanCRP\modes\*.toml`. La CLI también los ve.
- **Aviso de programas abiertos** (navegadores, editores, IDEs JetBrains, Ollama, LM Studio) vía Toolhelp32.
- **Reiniciar como administrador** (UAC) desde la app.
- Historial de limpiezas (`history.json`, últimas 500).
- Builds **MSI** + **NSIS (.exe)**, icono, versión única en `Cargo.toml`. Firma opcional documentada.
- Pendiente menor: firma de código real (requiere certificado), instancia única, bandeja del sistema.

### Fase 3 — Analizador de disco (tipo WinDirStat) + duplicados
- Escaneo paralelo de unidades (MFT opcional más adelante), árbol por tamaño, **treemap** (squarified) en canvas, top archivos, por extensión.
- Duplicados: agrupar por tamaño → hash parcial (xxh3 4 KB inicio/fin) → blake3 completo; borrar a la papelera / hardlink.
- Archivos grandes y antiguos.

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
- Salud de discos, actualizaciones de la app, firma de código, telemetría cero.

## Qué NO se hará (a propósito)
- Borrar **Prefetch**: empeora el arranque; Windows lo gestiona.
- "Optimizadores" de RAM o del registro tipo placebo.
- Borrar cookies/historial sin opción explícita (privacidad = módulo aparte, opt-in).
