# Exercise the real resource build twice without cleaning its target directory.
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
$fixture = Join-Path $root ('target\version-test\' + [guid]::NewGuid())
New-Item -ItemType Directory -Path "$fixture\src", "$fixture\assets" -Force | Out-Null
Copy-Item -LiteralPath "$root\build.rs" -Destination $fixture
Copy-Item -LiteralPath "$root\src\windows.rc" -Destination "$fixture\src"
Copy-Item -LiteralPath "$root\assets\icon.ico" -Destination "$fixture\assets"
Set-Content -LiteralPath "$fixture\src\main.rs" -Value 'fn main() {}' -Encoding ASCII
Push-Location $fixture
try {
    foreach ($version in @('1.2.3', '1.2.4')) {
        Set-Content -LiteralPath 'Cargo.toml' -Encoding ASCII -Value @"
[package]
name = "felaris-version-test"
version = "$version"
edition = "2024"
[build-dependencies]
embed-resource = "3.0"
"@
        cargo build --offline --quiet
        if ($LASTEXITCODE -ne 0) { throw 'Version fixture build failed' }
        $info = [Diagnostics.FileVersionInfo]::GetVersionInfo("$fixture\target\debug\felaris-version-test.exe")
        if ($info.FileVersion -ne $version -or $info.ProductVersion -ne $version) {
            throw "Incremental build expected $version; found file=$($info.FileVersion), product=$($info.ProductVersion)"
        }
    }
    Write-Host 'Incremental Windows file and product versions follow Cargo.toml without a clean build.'
} finally {
    Pop-Location
}
