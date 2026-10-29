$ErrorActionPreference = 'Stop'
$releaseDirectory = Join-Path $PSScriptRoot '..\target\release'
$cargo = Get-Content (Join-Path $PSScriptRoot '..\Cargo.toml') -Raw
$version = [regex]::Match($cargo, 'version\s*=\s*"([^"]+)"').Groups[1].Value
foreach ($name in @('veluno.exe', 'veluno-setup.exe')) {
    $path = Join-Path $releaseDirectory $name
    $info = [Diagnostics.FileVersionInfo]::GetVersionInfo($path)
    if ($info.ProductName.Trim() -ne 'Veluno' -or -not $info.FileVersion.Trim().StartsWith($version)) { throw "$name has incorrect Windows metadata" }
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
Write-Host "Verified Veluno ${version}: Windows GUI subsystem, icons, and version metadata for the app and setup."
