# Clean-CRP

Limpiador y mantenimiento **modular** para Windows, escrito en Rust.
Inspirado en CCleaner / PC Manager (limpieza), WinDirStat (mapa de disco) y
Bulk Crap Uninstaller (desinstalador), con **modos por perfil**:

| Modo | Para quién | Qué limpia |
|---|---|---|
| **Rápido** | Cualquiera | Temporales, miniaturas, informes de errores, volcados, shaders, papelera, caché de navegadores |
| **Programador** | Devs | Rápido + pip/uv/poetry/conda, npm/pnpm/yarn/bun, cargo, Go, Gradle, Maven, NuGet, VS Code/Cursor, JetBrains, `node_modules`/`target`/`__pycache__`/`.venv`, modelos de IA |
| **IA / ML** | Quien descarga modelos | Ollama, Hugging Face, LM Studio, Torch/Whisper/Keras + cachés Python |
| **Gamer** | Juegos | Rápido + shaders (incl. Steam), Delivery Optimization, Windows Update |
| **Profundo** | Avanzado | Todos los módulos |

Cada hallazgo tiene un **riesgo** (Seguro / Moderado / Avanzado). Sólo lo seguro
se preselecciona. **Analizar nunca borra nada.**

## Estado

Fases 0–2 listas: motor, 37 limpiadores, CLI y app de escritorio con
inicio (discos + historial), limpieza por modos, ajustes (carpetas de
proyectos, exclusiones), editor de modos personalizados, aviso de programas
abiertos y reinicio como administrador. Siguiente: analizador de disco y
duplicados. Plan completo en [docs/ROADMAP.md](docs/ROADMAP.md).

Datos del usuario en `%APPDATA%\CleanCRP\` (`settings.json`, `history.json`,
`modes\*.toml`).

## Inicio rápido

```powershell
# Requisitos: Rust stable (MSVC), Node 20+, WebView2 (incluido en Windows 11)
cargo install tauri-cli --version "^2" --locked

git clone <repo> Clean-CRP; cd Clean-CRP
cargo test                       # tests del motor y módulos
cargo run -p crp-cli -- scan --mode programador   # vista previa en terminal

cd app; npm install
cargo tauri dev                  # app de escritorio con recarga en caliente
```

Instaladores **MSI** y **EXE (NSIS)**:

```powershell
.\scripts\build.ps1
```

Detalles en [docs/BUILD.md](docs/BUILD.md).

## CLI

```text
crp info                              # rutas, admin, carpetas de proyectos detectadas
crp modes                             # modos disponibles
crp modules --mode programador        # módulos de un modo
crp scan --mode programador           # vista previa agrupada
crp scan --module "dev.*" --json      # selectores y salida JSON
crp clean --mode rapido               # SIMULACIÓN de lo preseleccionado
crp clean --mode rapido --yes         # limpia de verdad
crp clean --mode programador --risk moderate --yes
crp clean --module ai.ollama --only "llama3:8b" --yes
```

## Estructura

```
crates/
  crp-core/      motor: modelo, módulos, escaneo paralelo, borrado seguro, modos (TOML)
  crp-platform/  Win32 (admin, papelera…). Único sitio con unsafe
  crp-modules/   limpiadores: system, browsers, dev, ai
  crp-cli/       binario `crp`
app/
  src/           UI Svelte 5 + TypeScript
  src-tauri/     backend Tauri 2 (comandos, estado, bundle MSI/NSIS)
docs/            roadmap, arquitectura, módulos, build
scripts/         build.ps1
```

Más en [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) y el catálogo de
limpiadores en [docs/MODULES.md](docs/MODULES.md).

## Licencia

MIT
