$ErrorActionPreference = 'Stop'
$releaseDirectory = Join-Path $PSScriptRoot '..\target\release'
$cargo = Get-Content (Join-Path $PSScriptRoot '..\Cargo.toml') -Raw
$version = [regex]::Match($cargo, 'version\s*=\s*"([^"]+)"').Groups[1].Value
foreach ($name in @('felaris.exe', 'felaris-setup.exe')) {
    $path = Join-Path $releaseDirectory $name
    $info = [Diagnostics.FileVersionInfo]::GetVersionInfo($path)
    if ($info.ProductName -ne 'Felaris' -or $info.FileVersion -ne $version) { throw "$name has incorrect Windows metadata" }
    $bytes = [IO.File]::ReadAllBytes($path)
    $pe = [BitConverter]::ToInt32($bytes, 0x3c)
    if ([BitConverter]::ToUInt16($bytes, $pe + 24 + 68) -ne 2) { throw "$name is not a Windows GUI application" }
    $sectionCount = [BitConverter]::ToUInt16($bytes, $pe + 6)
    $sectionTable = $pe + 24 + [BitConverter]::ToUInt16($bytes, $pe + 20)
    $resourceTypes = @()
    for ($index = 0; $index -lt $sectionCount; $index++) {
        $section = $sectionTable + 40 * $index
        $sectionName = [Text.Encoding]::ASCII.GetString($bytes, $section, 8).TrimEnd([char]0)
        if ($sectionName -eq '.rsrc') {
            $root = [BitConverter]::ToInt32($bytes, $section + 20)
            $entries = [BitConverter]::ToUInt16($bytes, $root + 12) + [BitConverter]::ToUInt16($bytes, $root + 14)
            for ($entry = 0; $entry -lt $entries; $entry++) { $resourceTypes += [BitConverter]::ToUInt32($bytes, $root + 16 + 8 * $entry) }
        }
    }
    foreach ($type in @(3, 14, 16)) {
        if ($resourceTypes -notcontains $type) { throw "$name is missing an icon or version resource" }
    }
}
$appHash = (Get-FileHash -LiteralPath (Join-Path $releaseDirectory 'felaris.exe') -Algorithm SHA256).Hash
$setupHash = (Get-FileHash -LiteralPath (Join-Path $releaseDirectory 'felaris-setup.exe') -Algorithm SHA256).Hash
if ($appHash -ne $setupHash) { throw 'Setup payload differs from the verified app' }
Write-Host "Verified Felaris ${version}: Windows GUI subsystem, icons, version metadata, and matching setup payload."
