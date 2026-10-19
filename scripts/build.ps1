[CmdletBinding()]
param(
    [string]$Summary,
    [switch]$Prepare,
    [switch]$SkipDiscordNotify,
    [switch]$SkipGitHub,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$releaseArguments = @((Join-Path $PSScriptRoot 'build.py'))
if ($Summary) { $releaseArguments += @('--summary', $Summary) }
if ($Prepare) { $releaseArguments += '--prepare' }
if ($SkipDiscordNotify) { $releaseArguments += '--skip-discord' }
if ($SkipGitHub) { $releaseArguments += '--skip-github' }
if ($DryRun) { $releaseArguments += '--dry-run' }
& python @releaseArguments
exit $LASTEXITCODE
