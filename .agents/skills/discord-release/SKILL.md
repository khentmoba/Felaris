---
name: discord-release
description: Build release, binary upload to Discord, and human-readable changelog publication for Felaris. Use when publishing releases, uploading compiled felaris.exe binaries, or notifying the Discord #felaris-releases channel.
---

# Discord & GitHub Release Pipeline for Felaris

Automates building Felaris in release mode, creating GitHub Releases with `felaris.exe` assets (enabling the built-in auto-updater), uploading to Discord `#felaris-releases`, and notifying the community via webhook.

## Quick Commands

| Action | Command |
|---|---|
| **Prepare release notes before committing** | `python scripts/build.py --prepare -s "Release summary"` |
| **Build & Publish Release (GitHub + Discord)** | `python scripts/build.py` (or `.\scripts\build.ps1`) |
| **Build & Publish Discord only (skip GitHub)** | `python scripts/build.py --skip-github` |
| **Build without Discord notification** | `python scripts/build.py --skip-discord` |
| **Notify/Upload existing build manually** | `python scripts/notify_discord.py -s "Changelog summary"` |
| **Test webhook connection** | `python scripts/notify_discord.py --test` |
| **Dry run (inspect payload without sending)** | `python scripts/build.py --dry-run` |

## Direct release workflow

A release request authorizes preparing and pushing the release metadata directly
to `main`, publishing the GitHub release, and sending its Discord announcement.
Create a release PR only when the user explicitly requests one. A notification-only
request uses the existing release without a version bump or rebuild.

1. Fetch `origin/main` and prepare the release from that commit, preserving unrelated
   edits and worktrees. The release includes application changes already on `main`.
2. Use Python 3.11+. Set the version in `Cargo.toml`, update `Cargo.lock`, and run
   `python scripts/build.py --prepare -s "Release summary"` to write `version.json`.
   Commit only those three release metadata files on top of the fetched main commit.
3. Push that commit directly with `git push origin HEAD:main`, using a normal
   fast-forward push. If main advances, integrate it and review the combined changes
   before retrying. If repository rules reject direct pushes, report the rejection;
   a PR is not an automatic fallback.
4. Wait for successful Windows push CI on the exact current `origin/main` SHA.
   From a clean checkout of that commit, run `python scripts/build.py --dry-run`,
   then `python scripts/build.py` to package, publish GitHub assets, and notify Discord.
5. Verify the tag points to the release commit, the assets are uploaded, and Discord
   returned a message ID. Retain the release URL and message ID in the result.

GitHub publication reads committed metadata, verifies the package, and pins the tag
to that SHA. GitHub failures stop the Discord announcement; `--skip-github` is the
explicit Discord-only path. Preparation and publication are separate commands;
`--dry-run` validates without building or publishing.

## Webhook Configuration

- The webhook URL is resolved in order from:
  1. `DISCORD_WEBHOOK_URL` environment variable
  2. `scripts/discord_webhook.txt` or `discord_webhook.txt` (excluded in `.gitignore`)
  3. `%LOCALAPPDATA%\Felaris\discord_webhook.txt` (shared across all git worktrees)
- **Security Rule**: Never commit `discord_webhook.txt`.
- Channel: `#felaris-releases` (`1561759599609258109`) in the Felary Discord server.
