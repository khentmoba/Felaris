<#
.SYNOPSIS
    Builds Felaris and automatically publishes releases to GitHub and Discord.

.DESCRIPTION
    Runs cargo build --release, publishes to GitHub Releases via gh CLI,
    and invokes notify-discord.ps1 upon success.
#>

[CmdletBinding()]
param(
    [string]$Summary,
    [switch]$SkipDiscordNotify,
    [switch]$SkipGitHub,
    [switch]$DryRun
)

$ProjectRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host " Building Felaris (Release Mode)" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan

Push-Location $ProjectRoot
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[Build] Compilation failed with exit code $LASTEXITCODE." -ForegroundColor Red
        exit $LASTEXITCODE
    }
} finally {
    Pop-Location
}

$exePath = Join-Path $ProjectRoot "target\release\felaris.exe"
if (-not (Test-Path $exePath)) {
    Write-Host "[Build] Binary not found at $exePath" -ForegroundColor Red
    exit 1
}

$fileItem = Get-Item $exePath
$sizeMb = "{0:N2} MB" -f ($fileItem.Length / 1MB)
Write-Host "[Build] Compilation successful: $($fileItem.Name) ($sizeMb)" -ForegroundColor Green

# Extract version from Cargo.toml
$cargoToml = Get-Content (Join-Path $ProjectRoot "Cargo.toml") -Raw
$version = if ($cargoToml -match 'version\s*=\s*"([^"]+)"') { $Matches[1] } else { "0.1.0" }
if (-not $Summary) {
    $Summary = "Felaris v$version release — smooth, lightweight, Zen-inspired browser built in Rust."
}

# Sync version.json
$versionData = @{
    version = $version
    release_url = "https://github.com/khentmoba/Felaris/releases/tag/v$version"
    download_url = "https://github.com/khentmoba/Felaris/releases/download/v$version/felaris.exe"
    summary = $Summary
} | ConvertTo-Json -Depth 2
Set-Content -Path (Join-Path $ProjectRoot "version.json") -Value $versionData -Encoding utf8
Write-Host "[Version] Synced version.json with v$version" -ForegroundColor Gray

# GitHub Release
if (-not $SkipGitHub) {
    $tag = "v$version"
    if (Get-Command gh -ErrorAction SilentlyContinue) {
        Write-Host "`n==================================================" -ForegroundColor Cyan
        Write-Host " Publishing GitHub Release $tag" -ForegroundColor Cyan
        Write-Host "==================================================" -ForegroundColor Cyan

        if ($DryRun) {
            Write-Host "[DryRun] Would run: gh release create $tag $exePath --title 'Felaris $tag' --notes '$Summary'" -ForegroundColor DarkYellow
        } else {
            & gh release create $tag $exePath --title "Felaris $tag" --notes $Summary
            if ($LASTEXITCODE -eq 0) {
                Write-Host "[GitHub] Release $tag created successfully." -ForegroundColor Green
            } else {
                Write-Host "[GitHub] Warning: gh release create exited with code $LASTEXITCODE" -ForegroundColor Yellow
            }
        }
    } else {
        Write-Host "[GitHub] gh CLI not found; skipping GitHub release." -ForegroundColor DarkGray
    }
}

if ($SkipDiscordNotify) {
    Write-Host "[Discord] Skipping Discord notification (-SkipDiscordNotify specified)." -ForegroundColor DarkGray
    exit 0
}

Write-Host "`n==================================================" -ForegroundColor Cyan
Write-Host " Notifying Discord #felaris-releases" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan

$notifyScript = Join-Path $PSScriptRoot "notify-discord.ps1"
$notifyArgs = @()
if ($Summary) {
    $notifyArgs += @("-Summary", $Summary)
}
if ($DryRun) {
    $notifyArgs += "-DryRun"
}

& $notifyScript @notifyArgs
