<#
.SYNOPSIS
    Installs Felaris Browser on Windows.
.DESCRIPTION
    Downloads the latest felaris-setup.exe from GitHub, checks it against the
    release SHA256SUMS, and runs it silently (per-user, no administrator needed).
.EXAMPLE
    irm https://raw.githubusercontent.com/khentmoba/Felaris/main/scripts/install.ps1 | iex
#>

[CmdletBinding()]
param(
    [string]$Version,
    [switch]$NoLaunch
)

$ErrorActionPreference = "Stop"
$base = if ($Version) {
    "https://github.com/khentmoba/Felaris/releases/download/v$($Version.TrimStart('v'))"
} else {
    "https://github.com/khentmoba/Felaris/releases/latest/download"
}
$setup = Join-Path $env:TEMP "felaris-setup-$([IO.Path]::GetRandomFileName()).exe"
try {
    Write-Host "Downloading Felaris setup..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri "$base/felaris-setup.exe" -OutFile $setup -UseBasicParsing
    $sums = (Invoke-WebRequest -Uri "$base/SHA256SUMS" -UseBasicParsing).Content -split "`n"
    $expected = ($sums | Where-Object { $_ -match '\s+felaris-setup\.exe\s*$' } | Select-Object -First 1) -split '\s+' | Select-Object -First 1
    if (-not $expected -or (Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash -ne $expected) {
        throw "Download did not match the release checksum."
    }
    $arguments = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CLOSEAPPLICATIONS')
    if ($NoLaunch) { $arguments += '/NOLAUNCH' }
    $process = Start-Process -FilePath $setup -ArgumentList $arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "Setup failed with exit code $($process.ExitCode)" }
    Write-Host "Felaris installed. Find it in the Start menu." -ForegroundColor Green
} finally {
    Remove-Item -LiteralPath $setup -Force -ErrorAction SilentlyContinue
}
