param(
    [string]$RegistryPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Felaris',
    [string]$RegistryRoot = 'HKCU:\Software',
    [string]$DesktopDirectory = [Environment]::GetFolderPath('Desktop'),
    [string]$ProgramsDirectory = [Environment]::GetFolderPath('Programs')
)

$ErrorActionPreference = 'Stop'
$targetDir = Join-Path $env:LOCALAPPDATA 'Programs\Felaris'
$targetExe = Join-Path $targetDir 'felaris.exe'
$shortcuts = @(
    (Join-Path $ProgramsDirectory 'Felaris.lnk'),
    (Join-Path $DesktopDirectory 'Felaris.lnk')
)
$regKey = $RegistryPath
$classKey = Join-Path $RegistryRoot 'Classes\FelarisHTML'
$clientKey = Join-Path $RegistryRoot 'Clients\StartMenuInternet\Felaris'
$applicationsKey = Join-Path $RegistryRoot 'RegisteredApplications'

function Set-RegistryString($path, $name, $value) {
    if (-not (Test-Path -LiteralPath $path)) { New-Item -Path $path -Force | Out-Null }
    New-ItemProperty -Path $path -Name $name -Value $value -PropertyType String -Force | Out-Null
}

if ($env:FELARIS_INSTALL_ACTION -eq 'install') {
    New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
    if ([IO.Path]::GetFullPath($env:FELARIS_SOURCE_EXE) -ne [IO.Path]::GetFullPath($targetExe)) {
        Copy-Item -LiteralPath $env:FELARIS_SOURCE_EXE -Destination $targetExe -Force
    }
    $shell = New-Object -ComObject WScript.Shell
    foreach ($path in $shortcuts) {
        New-Item -ItemType Directory -Path (Split-Path $path) -Force | Out-Null
        $shortcut = $shell.CreateShortcut($path)
        $shortcut.TargetPath = $targetExe
        $shortcut.WorkingDirectory = $targetDir
        $shortcut.IconLocation = "$targetExe,0"
        $shortcut.Description = 'Felaris Browser'
        $shortcut.Save()
    }
    New-Item -Path $regKey -Force | Out-Null
    $properties = @{
        DisplayName = 'Felaris'
        DisplayVersion = $env:FELARIS_VERSION
        Publisher = 'Felaris Browser'
        DisplayIcon = "`"$targetExe`",0"
        InstallLocation = $targetDir
        UninstallString = "`"$targetExe`" --uninstall"
    }
    foreach ($name in $properties.Keys) {
        New-ItemProperty -Path $regKey -Name $name -Value $properties[$name] -PropertyType String -Force | Out-Null
    }
    foreach ($name in @('NoModify', 'NoRepair')) {
        New-ItemProperty -Path $regKey -Name $name -Value 1 -PropertyType DWord -Force | Out-Null
    }
    $openCommand = "`"$targetExe`" -- `"%1`""
    Set-RegistryString $classKey '(default)' 'Felaris HTML Document'
    Set-RegistryString $classKey 'URL Protocol' ''
    Set-RegistryString "$classKey\DefaultIcon" '(default)' "`"$targetExe`",0"
    Set-RegistryString "$classKey\shell\open\command" '(default)' $openCommand
    Set-RegistryString $clientKey '(default)' 'Felaris'
    Set-RegistryString "$clientKey\shell\open\command" '(default)' "`"$targetExe`""
    Set-RegistryString "$clientKey\DefaultIcon" '(default)' "`"$targetExe`",0"
    Set-RegistryString "$clientKey\Capabilities" 'ApplicationName' 'Felaris'
    Set-RegistryString "$clientKey\Capabilities" 'ApplicationDescription' 'Browse the web with Felaris.'
    Set-RegistryString "$clientKey\Capabilities" 'ApplicationIcon' "`"$targetExe`",0"
    foreach ($protocol in @('http', 'https')) {
        Set-RegistryString "$clientKey\Capabilities\URLAssociations" $protocol 'FelarisHTML'
    }
    foreach ($extension in @('.htm', '.html')) {
        Set-RegistryString "$clientKey\Capabilities\FileAssociations" $extension 'FelarisHTML'
    }
    Set-RegistryString $applicationsKey 'Felaris' 'Software\Clients\StartMenuInternet\Felaris\Capabilities'
    Write-Output "Felaris installed to $targetDir"
} elseif ($env:FELARIS_INSTALL_ACTION -eq 'uninstall') {
    $running = Get-Process -Name felaris -ErrorAction SilentlyContinue | Where-Object {
        $_.Id -ne [int]$env:FELARIS_PARENT_PID -and $_.Path -eq $targetExe
    }
    if ($running) { throw 'Close Felaris before uninstalling, then try again.' }
    foreach ($path in $shortcuts) {
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }
    }
    if (Test-Path -LiteralPath $regKey) { Remove-Item -LiteralPath $regKey }
    foreach ($key in @($classKey, $clientKey)) {
        if (Test-Path -LiteralPath $key) { Remove-Item -LiteralPath $key -Recurse }
    }
    if (Test-Path -LiteralPath $applicationsKey) {
        Remove-ItemProperty -LiteralPath $applicationsKey -Name 'Felaris' -ErrorAction SilentlyContinue
    }
} elseif ($env:FELARIS_INSTALL_ACTION -eq 'cleanup') {
    # The running executable stays locked until the Rust process exits.
    Wait-Process -Id ([int]$env:FELARIS_PARENT_PID) -ErrorAction SilentlyContinue
    foreach ($path in @($targetExe, "$targetExe.old", "$targetExe.new")) {
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }
    }
    if ((Test-Path -LiteralPath $targetDir) -and @(Get-ChildItem -LiteralPath $targetDir -Force).Count -eq 0) {
        Remove-Item -LiteralPath $targetDir
    }
}
