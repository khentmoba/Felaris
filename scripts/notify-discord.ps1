<#
.SYNOPSIS
    Publishes Veluno releases and announcements to Discord via Webhook.
#>

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Version,

    [Parameter(Position = 1)]
    [string]$Summary,

    [Alias('ExePath')]
    [string]$InstallerPath,

    [string]$WebhookUrl,

    [switch]$Test,

    [switch]$DryRun,

    [switch]$NoUpload
)

$pyScript = Join-Path $PSScriptRoot "notify_discord.py"
$pyArgs = @()
if ($Test) { $pyArgs += "--test" }
if ($DryRun) { $pyArgs += "--dry-run" }
if ($NoUpload) { $pyArgs += "--no-upload" }
if ($Version) { $pyArgs += @("--version", $Version) }
if ($Summary) { $pyArgs += @("--summary", $Summary) }
if ($InstallerPath) { $pyArgs += @("--installer", $InstallerPath) }
if ($WebhookUrl) { $pyArgs += @("--webhook", $WebhookUrl) }

python $pyScript @pyArgs
exit $LASTEXITCODE
