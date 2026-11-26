# Catálogo de limpiadores

[English](MODULES.md) · Español

Riesgo: 🟢 Seguro (preseleccionado) · 🟡 Moderado · 🔴 Avanzado. 🛡 = requiere admin.

## Sistema (`system.*`)

| id | Qué borra | Riesgo |
|---|---|---|
| `system.user_temp` | `%TEMP%` con más de 24 h | 🟢 |
| `system.windows_temp` | `C:\Windows\Temp` con más de 24 h | 🟢 🛡 |
| `system.thumbnails` | `%LOCALAPPDATA%\Microsoft\Windows\Explorer\thumbcache_*.db` | 🟢 |
| `system.wer` | `WER\ReportArchive`, `ReportQueue`, `Temp` (usuario y 🛡 sistema) | 🟢 |
| `system.crash_dumps` | `%LOCALAPPDATA%\CrashDumps`; 🛡 `Minidump`, `LiveKernelReports`, `MEMORY.DMP` | 🟢 |
| `system.windows_logs` | `C:\Windows\Logs` con más de 7 días | 🟡 🛡 |
| `system.shader_cache` | `D3DSCache`, NVIDIA `DXCache`/`GLCache`, AMD `DxCache`/`DxcCache`/`VkCache`, Intel; Steam `shadercache` (🟡) | 🟢 |
| `system.delivery_optimization` | Caché P2P de Windows Update | 🟡 🛡 |
| `system.windows_update` | `SoftwareDistribution\Download` | 🟡 🛡 |
| `system.recycle_bin` | Papelera de todas las unidades (Win32) | 🟡 |

**No incluido a propósito:** Prefetch (empeora arranque), `iconcache` (iconos rotos hasta reiniciar).

## Navegadores (`browser.*`) — sólo caché

Chrome, Edge, Brave, Vivaldi, Chromium, Opera, Opera GX, Firefox.

| Qué | Riesgo |
|---|---|
| `Cache`, `Code Cache`, `GPUCache`, `Dawn*Cache`, `ShaderCache`, `GrShaderCache` de todos los perfiles | 🟢 |
| `Service Worker\CacheStorage`/`ScriptCache` (datos offline/PWA) | 🟡 |
| Firefox: `cache2`, `startupCache`, `thumbnails`, `jumpListCache` | 🟢 |

Cookies, historial, contraseñas y sesiones **no** se tocan.

## Desarrollo (`dev.*`)

| id | Ruta (o variable que la sobrescribe) | Riesgo |
|---|---|---|
| `dev.pip` | `PIP_CACHE_DIR` · `%LOCALAPPDATA%\pip\Cache` | 🟢 |
| `dev.uv` | `UV_CACHE_DIR` · `%LOCALAPPDATA%\uv\cache` | 🟢 |
| `dev.poetry` | `POETRY_CACHE_DIR` · `%LOCALAPPDATA%\pypoetry\Cache` | 🟢 |
| `dev.conda` | `*.conda`/`*.tar.bz2` en `pkgs` (anaconda3, miniconda3, miniforge3…) | 🟢 |
| `dev.npm` | `NPM_CONFIG_CACHE` · `%LOCALAPPDATA%\npm-cache` | 🟢 |
| `dev.pnpm` | `%LOCALAPPDATA%\pnpm\store` | 🟡 |
| `dev.yarn` | `YARN_CACHE_FOLDER` · `%LOCALAPPDATA%\Yarn\Cache`, `~\.yarn\berry\cache` | 🟢 |
| `dev.bun` | `BUN_INSTALL_CACHE_DIR` · `~\.bun\install\cache` | 🟢 |
| `dev.cargo` | `CARGO_HOME\registry\{cache,src}`, `git\checkouts` | 🟡 |
| `dev.go` | `GOCACHE` (🟢), `GOMODCACHE` (🟡) | 🟢/🟡 |
| `dev.gradle` | `GRADLE_USER_HOME\caches`, `wrapper\dists` (🟡), logs del daemon (🟢) | 🟡 |
| `dev.maven` | `~\.m2\repository` | 🟡 |
| `dev.nuget` | `NuGet\v3-cache` (🟢), `NUGET_PACKAGES` (🟡) | 🟢/🟡 |
| `dev.editors` | VS Code / Cursor / VSCodium / Windsurf: `Cache`, `CachedData`, `CachedExtensionVSIXs`, `logs`… | 🟢 |
| `dev.jetbrains` | `%LOCALAPPDATA%\JetBrains\*\caches` y `log` | 🟡 |
| `dev.projects` | En tus carpetas de proyectos: `__pycache__`/`.pytest_cache`/`.mypy_cache`/`.ruff_cache` (🟢, agrupados), `node_modules` (con `package.json`), `target` (con `Cargo.toml`), `.next`/`.nuxt`/`.svelte-kit`/`.turbo`/`.parcel-cache`/`.angular`, `.gradle` (🟡), `.venv`/`venv` con `pyvenv.cfg` (🔴). Muestra días sin cambios. | 🟡 |

Carpetas de proyectos por defecto (si existen): `~\Documents`, `~\source`,
`~\repos`, `~\projects`, `~\dev`, `~\code`, `~\src`, `~\workspace`, `~\Github`,
`~\git`, `~\Desktop`. CLI: `--root C:\ruta`.

## IA (`ai.*`)

| id | Qué | Riesgo |
|---|---|---|
| `ai.ollama` | Un hallazgo por modelo (`OLLAMA_MODELS` · `~\.ollama\models`). Borra manifiesto + capas no compartidas; no necesita el servidor. Tamaño = espacio que realmente se libera. | 🔴 |
| `ai.huggingface` | Un hallazgo por repo de `HF_HUB_CACHE`/`HF_HOME\hub` · `~\.cache\huggingface\hub` (modelos, datasets, spaces) con días desde la descarga | 🔴 |
| `ai.lmstudio` | Un hallazgo por modelo en `~\.lmstudio\models` y `~\.cache\lm-studio\models` | 🔴 |
| `ai.misc` | `TORCH_HOME\hub` (🟡), `~\.cache\whisper` (🔴), `~\.keras\models` (🟡) | 🟡 |

## Próximos (ver [ROADMAP](ROADMAP.es.md))

Docker/WSL (`wsl --unregister`, compactar VHDX), paquetes globales npm/pip sin
uso, desinstalador, registro, analizador de disco, duplicados, Windows.old,
`DISM /StartComponentCleanup`, privacidad de navegadores (opt-in).
