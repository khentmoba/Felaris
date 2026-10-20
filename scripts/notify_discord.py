#!/usr/bin/env python3
"""
Felaris Discord Release & Update Notification Tool
Uploads build binaries (.exe) and announcements to Discord via Webhook.
"""

import os
import sys
import json
import argparse
import mimetypes
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit
from urllib.request import Request, urlopen

PROJECT_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_AVATAR = "https://cdn.discordapp.com/avatars/1561759821349523456/b3cd8c6639f5a31b7ac863859670c423.png"


def get_webhook_url(explicit_url=None):
    if explicit_url:
        return explicit_url.strip()

    env_url = os.environ.get("DISCORD_WEBHOOK_URL", "").strip()
    if env_url:
        return env_url

    candidates = [
        PROJECT_ROOT / "scripts" / "discord_webhook.txt",
        PROJECT_ROOT / "discord_webhook.txt",
    ]
    for env_key in ("LOCALAPPDATA", "APPDATA"):
        base = os.environ.get(env_key)
        if base:
            candidates.append(Path(base) / "Felaris" / "discord_webhook.txt")

    for c in candidates:
        if c.is_file():
            url = c.read_text(encoding="utf-8").strip()
            if url.startswith("http"):
                return url
    return None


def get_cargo_version():
    cargo_path = PROJECT_ROOT / "Cargo.toml"
    if cargo_path.is_file():
        for line in cargo_path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line.startswith("version ="):
                return line.split("=")[1].strip(' "\'')
    return "0.1.0"


def send_webhook(webhook_url, payload, file_path=None):
    parts = urlsplit(webhook_url)
    query = dict(parse_qsl(parts.query))
    query['wait'] = 'true'
    webhook_url = urlunsplit(parts._replace(query=urlencode(query)))
    if not file_path:
        req = Request(
            webhook_url,
            data=json.dumps(payload).encode("utf-8"),
            headers={"Content-Type": "application/json", "User-Agent": "FelarisReleaseBot/1.0"},
        )
        with urlopen(req) as resp:
            return json.load(resp)
    else:
        boundary = "----WebKitFormBoundaryFelarisRelease7MA4YWxkTrZu0gW"
        file_bytes = file_path.read_bytes()
        filename = file_path.name
        mime_type = mimetypes.guess_type(filename)[0] or "application/octet-stream"

        body = [
            f"--{boundary}".encode("utf-8"),
            b'Content-Disposition: form-data; name="payload_json"',
            b"Content-Type: application/json\r\n",
            json.dumps(payload).encode("utf-8"),
            b"",
            f"--{boundary}".encode("utf-8"),
            f'Content-Disposition: form-data; name="files[0]"; filename="{filename}"'.encode("utf-8"),
            f"Content-Type: {mime_type}\r\n".encode("utf-8"),
            file_bytes,
            f"--{boundary}--\r\n".encode("utf-8"),
        ]

        data = b"\r\n".join(body)
        req = Request(
            webhook_url,
            data=data,
            headers={
                "Content-Type": f"multipart/form-data; boundary={boundary}",
                "User-Agent": "FelarisReleaseBot/1.0",
                "Content-Length": str(len(data)),
            },
        )
        with urlopen(req) as resp:
            return json.load(resp)


def main():
    parser = argparse.ArgumentParser(description="Send Felaris release/update notifications to Discord.")
    parser.add_argument("--test", action="store_true", help="Send a webhook connection test message")
    parser.add_argument("--version", "-v", help="Release version (defaults to Cargo.toml)")
    parser.add_argument("--summary", "-s", help="Release notes or announcement summary")
    parser.add_argument("--exe", help="Path to compiled felaris.exe binary")
    parser.add_argument("--no-upload", action="store_true", help="Skip uploading the .exe binary")
    parser.add_argument("--webhook", help="Explicit webhook URL")
    parser.add_argument("--dry-run", action="store_true", help="Print payload without sending")

    args = parser.parse_args()

    webhook_url = get_webhook_url(args.webhook)
    if not webhook_url and not args.dry_run:
        print("[Felaris] Error: No Discord webhook URL found.")
        print("  Place your webhook URL in %LOCALAPPDATA%\\Felaris\\discord_webhook.txt, scripts/discord_webhook.txt, or set DISCORD_WEBHOOK_URL.")
        sys.exit(1)

    if args.test:
        payload = {
            "username": "Felaris Release Bot",
            "avatar_url": DEFAULT_AVATAR,
            "embeds": [
                {
                    "title": "Felaris Release Webhook Connected",
                    "description": "Discord webhook integration for **Felaris** browser is active and verified.",
                    "color": 959977,
                    "fields": [
                        {"name": "Status", "value": "Ready to publish updates & binary releases", "inline": True},
                        {"name": "Engine", "value": "Rust 2024 / Wry / WebView2", "inline": True},
                        {"name": "Channel", "value": "#felaris-releases", "inline": True},
                    ],
                    "footer": {"text": "Felaris Release Pipeline"},
                }
            ],
        }
        if args.dry_run:
            print("[DryRun] Test payload:", json.dumps(payload, indent=2))
            return

        print("[Felaris] Sending test ping to Discord webhook...")
        send_webhook(webhook_url, payload)
        print("[Felaris] Test notification sent successfully!")
        return

    ver = args.version or get_cargo_version()
    summary = args.summary or json.loads((PROJECT_ROOT / 'version.json').read_text(encoding='utf-8-sig'))['summary']
    bullets = '\n'.join('- ' + line.strip().lstrip('-* ').strip() for line in summary.splitlines() if line.strip())
    if not bullets or len(bullets) > 1024:
        parser.error("Release notes must fit the 1024-character What's New field")

    exe_file = None
    if not args.no_upload:
        candidate = Path(args.exe) if args.exe else PROJECT_ROOT / 'target/release/felaris.exe'
        # Larger binaries use the public release asset to fit the default upload budget.
        if candidate.is_file() and candidate.stat().st_size <= 10 * 1024 * 1024:
            exe_file = candidate
    download_url = f'https://github.com/khentmoba/Felaris/releases/download/v{ver}/felaris.exe'
    delivery = ({"name": "Attached File", "value": f'{exe_file.name} ({exe_file.stat().st_size / (1024 * 1024):.2f} MB)', "inline": True}
                if exe_file else {"name": "Download", "value": f'[felaris.exe]({download_url})', "inline": True})
    payload = {
        "username": "Felaris Release Bot",
        "avatar_url": DEFAULT_AVATAR,
        "content": f"New Release: **Felaris v{ver}** is now available! " + ('Download the attached file.' if exe_file else 'Download felaris.exe using the link below.'),
        "allowed_mentions": {"parse": []},
        "embeds": [{
            "title": f"Release: Felaris v{ver}",
            "description": "Run felaris.exe on Windows to start browsing.",
            "color": 4363994,
            "fields": [
                {"name": "Version", "value": f"v{ver}", "inline": True},
                {"name": "Configuration", "value": "Windows x64 / Release", "inline": True},
                delivery,
                {"name": "What's New", "value": bullets, "inline": False},
            ],
            "footer": {"text": "Felaris Automated Release System"},
            "timestamp": datetime.now(timezone.utc).isoformat(),
        }],
    }

    if args.dry_run:
        print("[DryRun] Release payload:", json.dumps(payload, indent=2))
        if exe_file:
            print(f"[DryRun] Attaching binary: {exe_file}")
        return

    print(f"[Felaris] Publishing v{ver} release to Discord...")
    if exe_file:
        print(f"[Felaris] Attaching binary: {exe_file.name}")
    message = send_webhook(webhook_url, payload, file_path=exe_file)
    print(f"[Felaris] Discord message ID: {message['id']}")
    print(f"[Felaris] Successfully published v{ver} to #felaris-releases!")


if __name__ == "__main__":
    main()
