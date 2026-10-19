<#
.SYNOPSIS
    Felaris Upstream Sync & Conflict Inspector PowerShell Wrapper
#>
param(
    [string]$Upstream = "upstream",
    [string]$Branch = "",
    [switch]$ShowCommits,
    [switch]$AllowUnrelatedHistories
)

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$PyScript = Join-Path $ScriptDir "upstream_sync.py"

$ArgsList = @("--upstream", $Upstream)
if ($Branch) {
    $ArgsList += @("--branch", $Branch)
}
if ($ShowCommits) {
    $ArgsList += "--show-commits"
}
if ($AllowUnrelatedHistories) {
    $ArgsList += "--allow-unrelated-histories"
}

python $PyScript @ArgsList
exit $LASTEXITCODE
