# Prepares and publishes a release of RCleaner.
#
#   .\scripts\release.ps1 0.2.0            (bump, commit, tag and push)
#   .\scripts\release.ps1 0.2.0 -NoPush    (everything except the push)
#
# 1. Moves the "## [Unreleased]" notes of CHANGELOG.md under "## [0.2.0] - <today>".
# 2. Sets the version in Cargo.toml, Cargo.lock and app/package*.json.
# 3. Commits "chore(release): v0.2.0" and creates the tag v0.2.0.
# 4. Pushes main and the tag; GitHub Actions (release.yml) then builds the
#    installers and publishes the release with those notes.
param(
    [Parameter(Mandatory)][string]$Version,
    [switch]$NoPush
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$repo = 'https://github.com/CristopherPaiz/RCleaner'
$tag = "v$Version"
$utf8 = New-Object System.Text.UTF8Encoding($false)

function Read-Text($path) { [IO.File]::ReadAllText((Join-Path $root $path), $utf8) }
function Write-Text($path, $text) { [IO.File]::WriteAllText((Join-Path $root $path), $text, $utf8) }
# Native tools write progress to stderr. With 'Stop', Windows PowerShell turns
# that into an error when output is redirected, so run them with 'Continue'
# and check the exit code instead.
function Invoke-Native {
    $ErrorActionPreference = 'Continue'
    $exe, $rest = $args
    & $exe @rest 2>&1 | ForEach-Object { "$_" } | Out-Host
    $LASTEXITCODE
}
function Invoke-Git { if (Invoke-Native git -C $root @args) { throw "git $args failed" } }

# Plain numeric versions only (X.Y.Z).
if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw "Version must look like 1.2.3 (got '$Version')" }
if ((git -C $root branch --show-current) -ne 'main') { throw 'Releases are made from main' }
if (git -C $root status --porcelain) { throw 'Commit or stash your changes first' }
if (git -C $root tag --list $tag) { throw "Tag $tag already exists" }

# 1. CHANGELOG.md
$log = Read-Text 'CHANGELOG.md'
$nl = if ($log.Contains("`r`n")) { "`r`n" } else { "`n" }
$m = [regex]::Match($log, '(?ms)^## \[Unreleased\][^\n]*\n(.*?)(?=^## \[|^\[[^\]]+\]: |\z)')
if (-not $m.Success -or -not $m.Groups[1].Value.Trim()) {
    throw 'CHANGELOG.md has nothing under "## [Unreleased]"'
}
$date = Get-Date -Format 'yyyy-MM-dd'
$log = $log.Remove($m.Index, $m.Length).Insert($m.Index,
    "## [Unreleased]$nl$nl## [$Version] - $date$nl$($m.Groups[1].Value)")
$prev = [regex]::Matches($log, '(?m)^## \[(\d+\.\d+\.\d+)\]') |
    ForEach-Object { $_.Groups[1].Value } | Where-Object { $_ -ne $Version } | Select-Object -First 1
$versionLink = if ($prev) { "$repo/compare/v$prev...$tag" } else { "$repo/releases/tag/$tag" }
$log = [regex]::Replace($log, '(?m)^\[Unreleased\]: .*$', "[Unreleased]: $repo/compare/$tag...HEAD$nl[$Version]: $versionLink")
Write-Text 'CHANGELOG.md' $log

# 2. Versions
$cargo = Read-Text 'Cargo.toml'
$cargo = [regex]::Replace($cargo, '(?m)^version = "[^"]*"', "version = `"$Version`"")
Write-Text 'Cargo.toml' $cargo
Push-Location (Join-Path $root 'app')
try {
    npm version $Version --no-git-tag-version --allow-same-version | Out-Null
    if ($LASTEXITCODE) { throw 'npm version failed' }
}
finally { Pop-Location }
$manifest = Join-Path $root 'Cargo.toml'
if (Invoke-Native cargo update --workspace --offline --manifest-path $manifest) {
    if (Invoke-Native cargo update --workspace --manifest-path $manifest) { throw 'cargo update failed' }
}

# 3. Commit and tag
Invoke-Git add CHANGELOG.md Cargo.toml Cargo.lock app/package.json app/package-lock.json
Invoke-Git commit -q -m "chore(release): $tag"
Invoke-Git tag -a $tag -m "RCleaner $Version"

# 4. Push
if ($NoPush) {
    Write-Host "Ready. Publish with: git push origin main $tag"
}
else {
    Invoke-Git push origin main $tag
    Write-Host "Pushed $tag. GitHub is building the release: $repo/actions"
}
