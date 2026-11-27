# Genera el instalador de RCleaner (pregunta el idioma: inglés / español):
#   target\release\bundle\nsis\RCleaner_<ver>_x64-setup.exe
# y el ejecutable portable target\release\rcleaner.exe.
#
# Uso:  .\scripts\build.ps1            (instala dependencias npm y compila)
#       .\scripts\build.ps1 -SkipInstall
param(
    [switch]$SkipInstall
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

Push-Location (Join-Path $root 'app')
try {
    if (-not $SkipInstall) {
        npm ci --no-fund --no-audit
        if ($LASTEXITCODE) { throw 'npm ci fallo' }
    }

    cargo tauri build
    if ($LASTEXITCODE) { throw 'cargo tauri build fallo' }

    Get-ChildItem (Join-Path $root 'target\release\bundle') -Recurse -Include *-setup.exe |
        Select-Object @{n = 'Instalador'; e = { $_.FullName } }, @{n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } }
}
finally {
    Pop-Location
}
