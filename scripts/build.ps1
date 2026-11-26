# Genera los instaladores de RCleaner:
#   target\release\bundle\msi\RCleaner_<ver>_x64_es-ES.msi
#   target\release\bundle\nsis\RCleaner_<ver>_x64-setup.exe
#
# Uso:  .\scripts\build.ps1            (instala dependencias npm y compila)
#       .\scripts\build.ps1 -SkipInstall
#       .\scripts\build.ps1 -Target msi   (sólo MSI)  | -Target nsis (sólo EXE)
param(
    [switch]$SkipInstall,
    [ValidateSet('all', 'msi', 'nsis')][string]$Target = 'all'
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

Push-Location (Join-Path $root 'app')
try {
    if (-not $SkipInstall) {
        npm ci --no-fund --no-audit
        if ($LASTEXITCODE) { throw 'npm ci fallo' }
    }

    if ($Target -eq 'all') { cargo tauri build }
    else { cargo tauri build --bundles $Target }
    if ($LASTEXITCODE) { throw 'cargo tauri build fallo' }

    Get-ChildItem (Join-Path $root 'target\release\bundle') -Recurse -Include *.msi, *-setup.exe |
        Select-Object @{n = 'Instalador'; e = { $_.FullName } }, @{n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } }
}
finally {
    Pop-Location
}
