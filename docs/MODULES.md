# Cleaner catalog

English · [Español](MODULES.es.md)

Risk: 🟢 Safe (preselected) · 🟡 Moderate · 🔴 Advanced. 🛡 = needs admin.

## System (`system.*`)

| id | What it deletes | Risk |
|---|---|---|
| `system.user_temp` | `%TEMP%` older than 24 h | 🟢 |
| `system.windows_temp` | `C:\Windows\Temp` older than 24 h | 🟢 🛡 |
| `system.thumbnails` | `%LOCALAPPDATA%\Microsoft\Windows\Explorer\thumbcache_*.db` | 🟢 |
| `system.wer` | `WER\ReportArchive`, `ReportQueue`, `Temp` (user and 🛡 system) | 🟢 |
| `system.crash_dumps` | `%LOCALAPPDATA%\CrashDumps`; 🛡 `Minidump`, `LiveKernelReports`, `MEMORY.DMP` | 🟢 |
| `system.windows_logs` | `C:\Windows\Logs` older than 7 days | 🟡 🛡 |
| `system.shader_cache` | `D3DSCache`, NVIDIA `DXCache`/`GLCache`, AMD `DxCache`/`DxcCache`/`VkCache`, Intel; Steam `shadercache` (🟡) | 🟢 |
| `system.delivery_optimization` | Windows Update P2P cache | 🟡 🛡 |
| `system.windows_update` | `SoftwareDistribution\Download` | 🟡 🛡 |
| `system.recycle_bin` | Recycle Bin on every drive (Win32) | 🟡 |
| `system.windows_old` | `C:\Windows.old`, `$Windows.~BT`, `$Windows.~WS` (previous installation; ownership is given to Administrators first) | 🟡 🛡 |
| `system.component_store` | Superseded updates in `WinSxS`, via `DISM /StartComponentCleanup` (size from `/AnalyzeComponentStore`) | 🟡 🛡 |

**Left out on purpose:** Prefetch (slows down startup), `iconcache` (broken
icons until a restart).

## Browsers (`browser.*`) — cache only

Chrome, Edge, Brave, Vivaldi, Chromium, Opera, Opera GX, Firefox.

| What | Risk |
|---|---|
| `Cache`, `Code Cache`, `GPUCache`, `Dawn*Cache`, `ShaderCache`, `GrShaderCache` in every profile | 🟢 |
| `Service Worker\CacheStorage`/`ScriptCache` (offline/PWA data) | 🟡 |
| Firefox: `cache2`, `startupCache`, `thumbnails`, `jumpListCache` | 🟢 |

Cookies, history, passwords and sessions are **never** touched.

## Development (`dev.*`)

| id | Path (or variable that overrides it) | Risk |
|---|---|---|
| `dev.pip` | `PIP_CACHE_DIR` · `%LOCALAPPDATA%\pip\Cache` | 🟢 |
| `dev.uv` | `UV_CACHE_DIR` · `%LOCALAPPDATA%\uv\cache` | 🟢 |
| `dev.poetry` | `POETRY_CACHE_DIR` · `%LOCALAPPDATA%\pypoetry\Cache` | 🟢 |
| `dev.conda` | `*.conda`/`*.tar.bz2` in `pkgs` (anaconda3, miniconda3, miniforge3…) | 🟢 |
| `dev.npm` | `NPM_CONFIG_CACHE` · `%LOCALAPPDATA%\npm-cache` | 🟢 |
| `dev.pnpm` | `%LOCALAPPDATA%\pnpm\store` | 🟡 |
| `dev.yarn` | `YARN_CACHE_FOLDER` · `%LOCALAPPDATA%\Yarn\Cache`, `~\.yarn\berry\cache` | 🟢 |
| `dev.bun` | `BUN_INSTALL_CACHE_DIR` · `~\.bun\install\cache` | 🟢 |
| `dev.cargo` | `CARGO_HOME\registry\{cache,src}`, `git\checkouts` | 🟡 |
| `dev.go` | `GOCACHE` (🟢), `GOMODCACHE` (🟡) | 🟢/🟡 |
| `dev.gradle` | `GRADLE_USER_HOME\caches`, `wrapper\dists` (🟡), daemon logs (🟢) | 🟡 |
| `dev.maven` | `~\.m2\repository` | 🟡 |
| `dev.nuget` | `NuGet\v3-cache` (🟢), `NUGET_PACKAGES` (🟡) | 🟢/🟡 |
| `dev.editors` | VS Code / Cursor / VSCodium / Windsurf: `Cache`, `CachedData`, `CachedExtensionVSIXs`, `logs`… | 🟢 |
| `dev.jetbrains` | `%LOCALAPPDATA%\JetBrains\*\caches` and `log` | 🟡 |
| `dev.projects` | In your project folders: `__pycache__`/`.pytest_cache`/`.mypy_cache`/`.ruff_cache` (🟢, grouped), `node_modules` (with `package.json`), `target` (with `Cargo.toml`), `.next`/`.nuxt`/`.svelte-kit`/`.turbo`/`.parcel-cache`/`.angular`, `.gradle` (🟡), `.venv`/`venv` with `pyvenv.cfg` (🔴). Shows days since the last change. | 🟡 |

Default project folders (if they exist): `~\Documents`, `~\source`,
`~\repos`, `~\projects`, `~\dev`, `~\code`, `~\src`, `~\workspace`, `~\Github`,
`~\git`, `~\Desktop`. CLI: `--root C:\path`.

## AI (`ai.*`)

| id | What | Risk |
|---|---|---|
| `ai.ollama` | One finding per model (`OLLAMA_MODELS` · `~\.ollama\models`). Deletes the manifest + layers no other model uses; the server doesn't need to run. Size = space actually freed. | 🔴 |
| `ai.huggingface` | One finding per repo in `HF_HUB_CACHE`/`HF_HOME\hub` · `~\.cache\huggingface\hub` (models, datasets, spaces) with days since download | 🔴 |
| `ai.lmstudio` | One finding per model in `~\.lmstudio\models` and `~\.cache\lm-studio\models` | 🔴 |
| `ai.misc` | `TORCH_HOME\hub` (🟡), `~\.cache\whisper` (🔴), `~\.keras\models` (🟡) | 🟡 |

## Coming next (see [ROADMAP](ROADMAP.md))

Docker/WSL (`wsl --unregister`, compact VHDX), unused global npm/pip packages,
uninstaller, registry, disk analyzer, duplicates, Windows.old,
`DISM /StartComponentCleanup`, browser privacy (opt-in).
