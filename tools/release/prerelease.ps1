<#
.SYNOPSIS
  Startet den GitHub-Workflow "prerelease" (Version + Windows/macOS-Build + Pre-Release),
  wartet auf das Ende und gibt die Release-URL aus. Siehe docs\RELEASE.md.

.PARAMETER Version
  Optional, z. B. 0.3.0. Leer = letzte Version mit Minor + 1.

.EXAMPLE
  .\tools\release\prerelease.ps1
  .\tools\release\prerelease.ps1 -Version 0.5.0
#>
param(
    [string]$Version = "",
    [string]$Repo = "Zignixx/firepixel"
)
$ErrorActionPreference = 'Stop'

$gh = (Get-Command gh -ErrorAction SilentlyContinue).Source
if (-not $gh) { $gh = 'C:\Program Files\GitHub CLI\gh.exe' }
if (-not (Test-Path $gh)) { throw "GitHub CLI (gh) nicht gefunden" }

if ($Version -match '^v?1\.') {
    $a = Read-Host "Version $Version ist eine 1.x-Version (nur auf ausdruecklichen Wunsch). Wirklich? (ja/nein)"
    if ($a -ne 'ja') { Write-Host "Abgebrochen."; exit 1 }
}

$before = Get-Date
$runArgs = @('workflow', 'run', 'prerelease.yml', '--repo', $Repo, '--ref', 'main')
if ($Version) { $runArgs += @('-f', "version=$Version") }
& $gh @runArgs
if ($LASTEXITCODE -ne 0) { throw "gh workflow run fehlgeschlagen" }

# Lauf-ID ermitteln (der neue Lauf erscheint nach wenigen Sekunden)
$runId = $null
for ($i = 0; $i -lt 30 -and -not $runId; $i++) {
    Start-Sleep -Seconds 3
    $runs = & $gh run list --repo $Repo --workflow prerelease.yml --event workflow_dispatch --limit 5 `
        --json databaseId,createdAt,url | ConvertFrom-Json
    $run = $runs | Where-Object { [datetime]$_.createdAt -ge $before.AddSeconds(-30) } |
        Select-Object -First 1
    if ($run) { $runId = $run.databaseId; Write-Host "Lauf: $($run.url)" }
}
if (-not $runId) { throw "Lauf nicht gefunden - bitte unter Actions > prerelease nachsehen" }

Write-Host "Warte auf das Ende (typisch 20-30 min) ..."
& $gh run watch $runId --repo $Repo --interval 30 --exit-status | Out-Null
$ok = ($LASTEXITCODE -eq 0)

if (-not $ok) {
    Write-Host "Lauf fehlgeschlagen:" -ForegroundColor Red
    & $gh run view $runId --repo $Repo --log-failed | Select-Object -Last 40
    exit 1
}

$tag = & $gh release list --repo $Repo --limit 1 --json tagName --jq ".[0].tagName"
$url = & $gh release view $tag --repo $Repo --json url --jq ".url"
Write-Host "Pre-Release ${tag}: $url" -ForegroundColor Green
