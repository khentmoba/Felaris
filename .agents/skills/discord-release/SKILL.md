---
name: discord-release
description: Build release, binary upload to Discord, and human-readable changelog publication for Felaris. Use when publishing releases, uploading compiled felaris.exe binaries, or notifying the Discord #felaris-releases channel.
---

# Discord & GitHub Release Pipeline for Felaris

Automates building Felaris in release mode, creating GitHub Releases with `felaris.exe` assets (enabling the built-in auto-updater), uploading to Discord `#felaris-releases`, and notifying the community via webhook.

## Quick Commands

| Action | Command |
|---|---|
| **Build & Publish Release (GitHub + Discord)** | `python scripts/build.py` (or `.\scripts\build.ps1`) |
| **Build & Publish with Custom Changelog** | `python scripts/build.py -s "Overhauled UI with Zen Browser sidebar layout"` |
| **Build & Publish Discord only (skip GitHub)** | `python scripts/build.py --skip-github` |
| **Build without Discord notification** | `python scripts/build.py --skip-discord` |
| **Notify/Upload existing build manually** | `python scripts/notify_discord.py -s "Changelog summary"` |
| **Test webhook connection** | `python scripts/notify_discord.py --test` |
| **Dry run (inspect payload without sending)** | `python scripts/build.py --dry-run` |

## Webhook Configuration

- The webhook URL is resolved in order from:
  1. `DISCORD_WEBHOOK_URL` environment variable
  2. `scripts/discord_webhook.txt` or `discord_webhook.txt` (excluded in `.gitignore`)
  3. `%LOCALAPPDATA%\Felaris\discord_webhook.txt` (shared across all git worktrees)
- **Security Rule**: Never commit `discord_webhook.txt`.
- Channel: `#felaris-releases` (`1561759599609258109`) in the Felary Discord server.
