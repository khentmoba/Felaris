# Builds Felaris with the experimental Servo engine and optionally runs it.
#   powershell -NoProfile -File scripts/servo.ps1 [-Release] [-Run] [-RetryLockedCompiles] [-- <felaris args>]
#   powershell -NoProfile -File scripts/servo.ps1 -Check [-RetryLockedCompiles]   # clippy and tests
# Run Servo cargo commands through this script: a different environment reruns the C++ builds.
# Needs libclang for ANGLE's bindgen step. Set LIBCLANG_PATH, or the script downloads the
# libclang wheel from PyPI into target/servo-tools.
# -RetryLockedCompiles builds C++ through scripts/cl-retry.rs, which retries compiles that fail
# with C1056. Switching it on or off reruns every
# C/C++ build script.
param(
    [switch]$Release,
    [switch]$Check,
    [switch]$Run,
    [switch]$RetryLockedCompiles,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$AppArgs
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

# bindgen reads include paths from the clang on PATH, so its libclang must be the same version.
$clang = Get-Command clang.exe -ErrorAction SilentlyContinue
if (-not $env:LIBCLANG_PATH -and $clang -and (Test-Path (Join-Path (Split-Path $clang.Source) 'libclang.dll'))) {
    $env:LIBCLANG_PATH = Split-Path $clang.Source
}
if (-not $env:LIBCLANG_PATH) {
    $tools = Join-Path $root 'target/servo-tools'
    $native = Join-Path $tools 'libclang-18.1.1.data/platlib/clang/native'
    if (-not (Test-Path (Join-Path $native 'libclang.dll'))) {
        New-Item -ItemType Directory -Force $tools | Out-Null
        python -m pip download libclang==18.1.1 --no-deps --only-binary=:all: --platform win_amd64 -d $tools
        if ($LASTEXITCODE) { throw 'Could not download libclang.' }
        $wheel = Get-ChildItem $tools -Filter 'libclang-*.whl' | Select-Object -First 1
        python -I -c "import sys,zipfile; z=zipfile.ZipFile(sys.argv[1]); [z.extract(n, sys.argv[2]) for n in z.namelist() if n.endswith('.dll')]" $wheel.FullName $tools
        if ($LASTEXITCODE) { throw 'Could not extract libclang.' }
    }
    $env:LIBCLANG_PATH = $native
}
# libclang 18 parses headers only; these let it accept newer MSVC STL headers.
$env:BINDGEN_EXTRA_CLANG_ARGS = "-D_ALLOW_COMPILER_AND_STL_VERSION_MISMATCH '-D__builtin_verbose_trap(a,b)=__builtin_trap()'"

if ($RetryLockedCompiles) {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    $vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if (-not $vs) { throw 'Could not find Visual Studio with the C++ tools.' }
    $version = (Get-Content (Join-Path $vs 'VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt')).Trim()
    $msvc = Join-Path $vs "VC/Tools/MSVC/$version"
    $kits = (Get-ItemProperty 'HKLM:/SOFTWARE/Microsoft/Windows Kits/Installed Roots').KitsRoot10
    $sdk = Get-ChildItem (Join-Path $kits 'Include') -Directory |
        Where-Object { Test-Path (Join-Path $_.FullName 'um/windows.h') } |
        Sort-Object { [version]$_.Name } | Select-Object -Last 1
    if (-not $sdk) { throw 'Could not find the Windows SDK headers.' }
    # cc sets up MSVC's environment only for the cl.exe it finds itself.
    $env:INCLUDE = (@(Join-Path $msvc 'include') +
        ('ucrt', 'um', 'shared', 'winrt' | ForEach-Object { Join-Path $sdk.FullName $_ })) -join ';'
    $env:FELARIS_REAL_CL = Join-Path $msvc 'bin/HostX64/x64/cl.exe'
    $wrapper = Join-Path $root 'target/servo-tools/cl-retry/cl.exe'
    $source = Join-Path $root 'scripts/cl-retry.rs'
    if (-not (Test-Path $wrapper) -or (Get-Item $source).LastWriteTime -gt (Get-Item $wrapper).LastWriteTime) {
        New-Item -ItemType Directory -Force (Split-Path $wrapper) | Out-Null
        rustc -O --edition 2021 $source -o $wrapper
        if ($LASTEXITCODE) { throw 'Could not build cl-retry.' }
    }
    $env:CXX = $wrapper
}

$profile = if ($Release) { 'release' } else { 'debug' }
if ($Check) {
    cargo clippy --all-targets --features servo --locked
    if ($LASTEXITCODE) { exit $LASTEXITCODE }
    cargo test --features servo --locked
    exit $LASTEXITCODE
}

$cargo = @('build', '--features', 'servo', '--locked')
if ($Release) { $cargo += '--release' }
cargo @cargo
if ($LASTEXITCODE) { exit $LASTEXITCODE }

# surfman loads ANGLE at runtime; the newest mozangle build output is the one just linked.
$out = Join-Path $root "target/$profile"
foreach ($dll in 'libEGL.dll', 'libGLESv2.dll') {
    $built = Get-ChildItem (Join-Path $out 'build') -Recurse -Filter $dll |
        Where-Object { $_.FullName -match 'mozangle' } |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $built) { throw "ANGLE did not produce $dll." }
    Copy-Item $built.FullName $out -Force
}

if ($Run) {
    & (Join-Path $out 'felaris.exe') --engine=servo @AppArgs
}
