<#
.SYNOPSIS
    Installs Felaris Browser on Windows.
.DESCRIPTION
    Downloads the latest release of Felaris from GitHub and installs it to
    %LOCALAPPDATA%\Programs\Felaris, adding Start Menu and Desktop shortcuts.
.EXAMPLE
    irm https://raw.githubusercontent.com/khentmoba/Felaris/main/scripts/install.ps1 | iex
#>

[CmdletBinding()]
param(
    [string]$Version,
    [switch]$NoLaunch
)

$ErrorActionPreference = "Stop"

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host " Installing Felaris Browser" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan

$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\Felaris"
$TargetExe = Join-Path $InstallDir "felaris.exe"

if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$DownloadUrl = if ($Version) {
    "https://github.com/khentmoba/Felaris/releases/download/v$($Version.TrimStart('v'))/felaris.exe"
} else {
    "https://github.com/khentmoba/Felaris/releases/latest/download/felaris.exe"
}

Write-Host "[Install] Downloading latest felaris.exe from $DownloadUrl..." -ForegroundColor Gray
$TempExe = Join-Path $env:TEMP "felaris_setup_$([System.IO.Path]::GetRandomFileName()).exe"

try {
    # Prefer curl.exe if available for speed and reliability
    if (Get-Command curl.exe -ErrorAction SilentlyContinue) {
        & curl.exe -s -f -L --max-time 120 -o $TempExe $DownloadUrl
        if ($LASTEXITCODE -ne 0) {
            throw "curl exited with code $LASTEXITCODE"
        }
    } else {
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempExe -UseBasicParsing
    }

    $fileSize = (Get-Item $TempExe).Length
    if ($fileSize -lt 500000) {
        throw "Downloaded binary is too small ($fileSize bytes). Download may have failed."
    }

    # Stop running instances if any before replacing
    Get-Process felaris -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500

    Move-Item -Path $TempExe -Destination $TargetExe -Force
    Write-Host "[Install] Binary placed at $TargetExe" -ForegroundColor Green
} catch {
    if (Test-Path $TempExe) { Remove-Item -Path $TempExe -Force -ErrorAction SilentlyContinue }
    Write-Host "[Install] Failed to download or install: $_" -ForegroundColor Red
    exit 1
}

# Run the built-in installer mode to create shortcuts & uninstall registry
Write-Host "[Install] Configuring shortcuts and Windows registration..." -ForegroundColor Gray
$installProcess = Start-Process -FilePath $TargetExe -ArgumentList '--install' -Wait -PassThru -WindowStyle Hidden
if ($installProcess.ExitCode -ne 0) {
    throw "Felaris registration failed with exit code $($installProcess.ExitCode)"
}

Write-Host "`n[Install] Felaris installed successfully!" -ForegroundColor Green

if (-not $NoLaunch) {
    Write-Host "[Install] Launching Felaris..." -ForegroundColor Cyan
    Start-Process -FilePath $TargetExe
}
