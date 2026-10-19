# Installs the built felaris-setup.exe silently, checks the result, then uninstalls it. Run after package.ps1 builds it.
$ErrorActionPreference = 'Stop'
$setup = Join-Path $PSScriptRoot '..\target\release\felaris-setup.exe'
$dir = Join-Path ([IO.Path]::GetTempPath()) ("Felaris installer test " + [guid]::NewGuid())
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{6F1D2C8E-4B7A-4E55-9D3A-FE1A215B0C11}_is1'
$legacyKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Felaris'
$startMenu = Join-Path ([Environment]::GetFolderPath('Programs')) 'Felaris.lnk'
function Assert($condition, $message) { if (-not $condition) { throw $message } }
function Run($path, $arguments) {
    $p = Start-Process -FilePath $path -ArgumentList $arguments -Wait -PassThru
    Assert ($p.ExitCode -eq 0) "$path exited with $($p.ExitCode)"
}
try {
    New-Item -Path $legacyKey -Force | Out-Null
    Run $setup "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /NOLAUNCH /DIR=`"$dir`""
    Assert (Test-Path -LiteralPath (Join-Path $dir 'felaris.exe')) 'App was not installed'
    Assert (Test-Path -LiteralPath (Join-Path $dir 'unins000.exe')) 'Uninstaller is missing'
    Assert (Test-Path -LiteralPath $uninstallKey) 'Not listed in Installed apps'
    Assert (-not (Test-Path -LiteralPath $legacyKey)) 'Legacy Installed apps entry remains'
    Assert (Test-Path -LiteralPath $startMenu) 'Start menu shortcut is missing'
    $capabilities = 'HKCU:\Software\Clients\StartMenuInternet\Felaris\Capabilities'
    Assert ((Get-ItemProperty "$capabilities\URLAssociations").https -eq 'FelarisHTML') 'Missing HTTPS registration'
    Assert ((Get-ItemProperty "$capabilities\FileAssociations").'.html' -eq 'FelarisHTML') 'Missing HTML registration'
    Assert ((Get-Item 'HKCU:\Software\Classes\FelarisHTML\shell\open\command').GetValue('') -eq "`"$dir\felaris.exe`" -- `"%1`"") 'Incorrect link command'
    Run $setup "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /NOLAUNCH /DIR=`"$dir`""
    Run (Join-Path $dir 'unins000.exe') '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART'
    for ($i = 0; $i -lt 50 -and (Test-Path -LiteralPath (Join-Path $dir 'felaris.exe')); $i++) { Start-Sleep -Milliseconds 200 }
    Assert (-not (Test-Path -LiteralPath (Join-Path $dir 'felaris.exe'))) 'App remains after uninstall'
    Assert (-not (Test-Path -LiteralPath $uninstallKey)) 'Installed apps entry remains'
    Assert (-not (Test-Path -LiteralPath $startMenu)) 'Start menu shortcut remains'
    Assert (-not (Test-Path 'HKCU:\Software\Clients\StartMenuInternet\Felaris')) 'Browser registration remains'
    Write-Host 'Installer install, upgrade, legacy cleanup, registration, and uninstall passed.'
} finally {
    if (Test-Path -LiteralPath (Join-Path $dir 'unins000.exe')) { & (Join-Path $dir 'unins000.exe') /VERYSILENT /SUPPRESSMSGBOXES | Out-Null; Start-Sleep 2 }
    if (Test-Path -LiteralPath $legacyKey) { Remove-Item -LiteralPath $legacyKey }
    if ((Test-Path -LiteralPath $dir) -and $dir.StartsWith([IO.Path]::GetTempPath())) { Remove-Item -LiteralPath $dir -Recurse -Force }
}
