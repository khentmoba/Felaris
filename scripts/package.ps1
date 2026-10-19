<# Builds the portable app and the double-click Windows setup executable. #>
[CmdletBinding()]
param([string]$CertificateThumbprint, [string]$TimestampServer)

$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    if ($CertificateThumbprint) {
        if (-not $TimestampServer) { throw 'Provide -TimestampServer when signing a release' }
        $certificate = Get-Item -LiteralPath "Cert:\CurrentUser\My\$CertificateThumbprint"
        if (-not $certificate.HasPrivateKey) { throw 'The signing certificate has no private key' }
        $signature = Set-AuthenticodeSignature -FilePath 'target\release\felaris.exe' -Certificate $certificate -HashAlgorithm SHA256 -TimestampServer $TimestampServer
        if ($signature.Status -ne 'Valid') { throw "Code signing failed: $($signature.StatusMessage)" }
    }
    Copy-Item -LiteralPath 'target\release\felaris.exe' -Destination 'target\release\felaris-setup.exe' -Force
    & (Join-Path $PSScriptRoot 'verify-package.ps1')
    python (Join-Path $PSScriptRoot '..\tests\native_smoke.py') --exe 'target\release\felaris.exe'
    if ($LASTEXITCODE -ne 0) { throw 'Native app verification failed' }
    $checksums = foreach ($name in @('felaris.exe', 'felaris-setup.exe')) {
        $hash = (Get-FileHash -LiteralPath (Join-Path 'target\release' $name) -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $name"
    }
    Set-Content -LiteralPath 'target\release\SHA256SUMS' -Value $checksums -Encoding ASCII
    Write-Host 'App: target\release\felaris.exe'
    Write-Host 'Setup: target\release\felaris-setup.exe'
} finally {
    Pop-Location
}
