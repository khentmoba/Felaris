$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot '..\src\install.ps1'
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ("Felaris test ' Unicode-" + [guid]::NewGuid())
$registryPath = 'HKCU:\Software\Felaris-Install-Test-' + [guid]::NewGuid()
$testRegistryRoot = $registryPath + '-Root'
$names = @('LOCALAPPDATA', 'APPDATA', 'USERPROFILE', 'FELARIS_SOURCE_EXE', 'FELARIS_VERSION', 'FELARIS_PARENT_PID', 'FELARIS_INSTALL_ACTION')
$saved = @{}
foreach ($name in $names) { $saved[$name] = [Environment]::GetEnvironmentVariable($name) }
function Assert($condition, $message) { if (-not $condition) { throw $message } }
try {
    New-Item -ItemType Directory -Path $testRoot | Out-Null
    $env:LOCALAPPDATA = Join-Path $testRoot 'Local'
    $env:APPDATA = Join-Path $testRoot 'Roaming'
    $env:USERPROFILE = Join-Path $testRoot 'User'
    $env:FELARIS_SOURCE_EXE = Join-Path $testRoot 'source.exe'
    $env:FELARIS_VERSION = '1.2.3'
    $env:FELARIS_PARENT_PID = '2147483647'
    $installArguments = @{
        RegistryPath = $registryPath
        RegistryRoot = $testRegistryRoot
        DesktopDirectory = (Join-Path $env:USERPROFILE 'Desktop')
        ProgramsDirectory = (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs')
    }
    Set-Content -LiteralPath $env:FELARIS_SOURCE_EXE -Value 'test binary'
    $env:FELARIS_INSTALL_ACTION = 'install'
    & $script @installArguments
    & $script @installArguments
    $target = Join-Path $env:LOCALAPPDATA 'Programs\Felaris\felaris.exe'
    Assert (Test-Path -LiteralPath $target) 'App was not copied'
    $registration = Get-ItemProperty -LiteralPath $registryPath
    Assert ($registration.DisplayVersion -eq '1.2.3') 'Incorrect version'
    Assert ($registration.UninstallString -eq "`"$target`" --uninstall") 'Incorrect uninstall command'
    Assert ((Get-ItemProperty "$testRegistryRoot\Clients\StartMenuInternet\Felaris\Capabilities\URLAssociations").https -eq 'FelarisHTML') 'Missing HTTPS registration'
    Assert ((Get-Item "$testRegistryRoot\Classes\FelarisHTML\shell\open\command").GetValue('') -eq "`"$target`" -- `"%1`"") 'Incorrect external link command'
    $shell = New-Object -ComObject WScript.Shell
    $shortcuts = @(
        (Join-Path $env:USERPROFILE 'Desktop\Felaris.lnk'),
        (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Felaris.lnk')
    )
    foreach ($path in $shortcuts) {
        Assert (Test-Path -LiteralPath $path) 'Missing shortcut'
        Assert ($shell.CreateShortcut($path).TargetPath -eq $target) 'Incorrect shortcut target'
    }
    $keep = Join-Path (Split-Path $target) 'keep.txt'
    Set-Content -LiteralPath $keep -Value 'preserve me'
    $env:FELARIS_INSTALL_ACTION = 'uninstall'
    & $script @installArguments
    Assert (-not (Test-Path -LiteralPath $registryPath)) 'Registry entry remains'
    foreach ($path in $shortcuts) { Assert (-not (Test-Path -LiteralPath $path)) 'Shortcut remains' }
    $env:FELARIS_INSTALL_ACTION = 'cleanup'
    & $script @installArguments
    Assert (-not (Test-Path -LiteralPath $target)) 'Executable remains'
    Assert (Test-Path -LiteralPath $keep) 'Uninstaller deleted an unrelated file'
    & $script @installArguments
    Remove-Item -LiteralPath $keep
    & $script @installArguments
    Assert (-not (Test-Path -LiteralPath (Split-Path $target))) 'Empty install directory remains'
    Assert (Test-Path -LiteralPath $env:FELARIS_SOURCE_EXE) 'Portable source was removed'
    $env:FELARIS_SOURCE_EXE = Join-Path $testRoot 'missing.exe'
    $env:FELARIS_INSTALL_ACTION = 'install'
    $failed = $false
    try { & $script @installArguments } catch { $failed = $true }
    Assert $failed 'Missing source should fail installation'
    Write-Host 'Install, reinstall, shortcuts, registration, uninstall, and file preservation passed.'
} finally {
    foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $saved[$name]) }
    if (Test-Path -LiteralPath $registryPath) { Remove-Item -LiteralPath $registryPath }
    if (Test-Path -LiteralPath $testRegistryRoot) { Remove-Item -LiteralPath $testRegistryRoot -Recurse }
    if ((Split-Path $testRoot) -eq ([IO.Path]::GetTempPath()).TrimEnd('\')) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
