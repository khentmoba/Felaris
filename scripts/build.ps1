<#
.SYNOPSIS
    Builds Felaris and automatically publishes releases to Discord.

.DESCRIPTION
    Runs cargo build --release and invokes notify-discord.ps1 upon success.
#>

[CmdletBinding()]
param(
    [string]$Summary,
    [switch]$SkipDiscordNotify,
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
