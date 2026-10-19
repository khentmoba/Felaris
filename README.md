<p align="center">
  <img src="assets/logo-wordmark.svg" alt="Felaris Browser" width="320">
</p>

<p align="center">
  <strong>A smooth, lightweight, and easy-to-use modern browser built in Rust.</strong><br>
  Designed from the ground up to be ultra-fast, gentle on PC hardware, and free of background bloat.
</p>

---

## Highlights

- **Rust desktop shell:** Tao handles the Windows window and Wry hosts WebView2.
- **Independent live tabs:** Switching tabs preserves page state and native back/forward history while the tabs stay awake.
- **Session recovery:** Tabs, workspace selection, pins, and recent closed tabs are saved locally, with a backup for recovery. Site storage uses a persistent WebView2 profile.
- **Windows integration:** Setup wizard with uninstaller, Start menu and desktop shortcuts, Installed apps registration, default-browser eligibility, and link handoff to the running instance.
- **Native downloads:** WebView2 provides download progress and file actions; Felaris reports completion and warns before closing active downloads.
- **Explicit tab sleeping:** Unloads inactive pages. Selecting a sleeping tab reloads its saved URL; unsaved form state and page history do not survive sleeping or an app restart.
- **Built-in Upstream Sync & Conflict Inspector:** Includes an automated local tool and agent skill to fetch upstream sources, compare local changes, and simulate pre-merge conflict status in memory before applying.

---

## Architecture

```
┌────────────────────────────────────────────────────────┐
│  FELARIS (Rust Application Core)                       │
│                                                        │
│  Zen-inspired sidebar (260px / 56px compact rail)       │
│  • Navigation and address / search                     │
│  • Pinned tabs and searchable workspace tabs           │
│  • Workspace dock and tab actions                      │
│                                                        │
│  ┌─ Content Viewport (Hardware-Accelerated) ────────┐  │
│  │                                                  │  │
│  │   Web content rendered at native display refresh │  │
│  │                                                  │  │
│  └──────────────────────────────────────────────────┘  │
│                                                        │
│  Rust Event Loop & State:                              │
│  • High-DPI dynamic viewport auto-resizing             │
│  • Memory-efficient tab state & history tracking       │
│  • Fast IPC bridge between Chrome UI and Rust          │
└────────────────────────────────────────────────────────┘
```

---

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Ctrl + T` | Open floating new-tab search |
| `Ctrl + W` | Close active tab |
| `Ctrl + L` | Focus and highlight address / search bar |
| `Ctrl + R` / `F5` | Reload current page |
| `Alt + Left` | Navigate back |
| `Alt + Right` | Navigate forward |
| `Ctrl + Shift + T` | Reopen last closed tab |
| `Ctrl + Tab` / `Ctrl + Shift + Tab` | Cycle tabs within the workspace |
| `Alt + 1` / `2` / `3` | Switch workspace |
| `Ctrl + S` | Expand / collapse sidebar |
| `Ctrl + Shift + S` | Unload background tabs (except active downloads) |
| `Ctrl + J` | Open Downloads |
| `Enter` (in URL bar) | Smart navigate (detects URLs vs. web search) |

**New tabs work like Zen:** press `Ctrl+T` or click **New tab** to open a floating search/address bar over the current page. Enter opens the destination in a new tab; matching open tabs can be selected with the arrow keys. Escape, the close button, or clicking outside cancels without creating an empty tab or reloading the page. The startup/Home page remains available separately. The sidebar, Home page, native title bar, and palette share a black/charcoal theme.

The sidebar has no fixed shortcut-tile dock. Search opens/closes with short, interruptible motion, and tab/workspace changes retain visual continuity. Reduced-motion preferences skip spatial effects. Home-page shortcuts rank your most-visited sites (such as YouTube or X) first and fill any remaining slots from the default 8 shortcuts.

Web searches use **Brave Search** throughout the sidebar, Home page, and floating search bar. Suggestions are local open tabs; no remote autocomplete requests are sent while typing.

Shortcuts work while browser chrome or page content has focus. The bottom sidebar menu contains pin/unpin, reopen, and close-other-tabs actions. Close Others keeps pinned tabs, affects only the current workspace, and can be undone one tab at a time with Reopen.

**Current engine limits:** tab and workspace state is session-only. Felaris uses one content WebView, so switching tabs reloads their URLs rather than preserving independent live page sessions. Sleep labels represent tab state, not measured RAM savings. The compact rail is not Zen's floating edge-hover mode.

See [DESIGN.md](DESIGN.md) for the interface decisions and research references.

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (edition 2024 / rustc 1.85+)
- Windows 10/11 (WebView2 runtime is pre-installed)

### Build & Run

```bash
# Run in development mode
cargo run

# Build optimized production release (< 3 MB binary)
cargo build --release
```

The compiled release executable will be located at:
```
target/release/felaris.exe
```

---

## UI Verification

```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
node tests/browser.test.cjs
```

For the 44 DOM interaction checks, serve the repository with `python -m http.server 0` (using an ephemeral port so worktrees do not collide), then open `http://localhost:<port>/tests/ui.html`. This is a fixture with sample tabs; its IPC messages are recorded rather than sent to Rust. Test both 1380×860 and the minimum 640×480 desktop window; the compact rail can be inspected by setting `fixtureState.sidebar_collapsed = true`, `sidebar.style.width = '56px'`, and calling `sidebar.contentWindow.updateState(fixtureState)` in the test page console.

To exercise actual WebView2 keyboard input and Rust IPC on Windows, build with `cargo build` and run the self-contained native smoke test with Python 3.11+ and Node 22+. The Node command delegates to the headless regression harness, which copies the executable, uses an isolated profile and dynamic debugging port, and cleans up the child process. It also runs all 44 DOM checks at 1380×860 and 640×480, including reduced motion:

```bash
cargo build
node tests/native-smoke.cjs
```

Native results and diagnostic logs are saved under `target/native-regressions/`. The runner always launches its own hidden app; it never attaches to an existing browser on port 9223. The same suite is mandatory during packaging and Windows CI.

## Upstream Synchronization & Conflict Detection

Felaris includes a built-in upstream inspector that simulates upstream branch merges in-memory without dirtying your working tree:

```bash
# Using Python
python scripts/upstream_sync.py --upstream <remote_or_url> --branch main

# Or via PowerShell wrapper
.\scripts\upstream-sync.ps1 -Upstream <remote_or_url> -Branch main
```

A dedicated agent skill is available at `.agents/skills/upstream-sync/SKILL.md` to guide automated synchronization and conflict audits.

---

## Installation & Updates

### Windows App & Setup

Felaris is a Rust desktop app using Tao and Wry (Windows WebView2). Release
builds include an Explorer icon, Windows product/version details, and a taskbar
identity, and open without a console window.

Build the app and its installer locally (needs [Inno Setup 6](https://jrsoftware.org/isinfo.php):
`winget install JRSoftware.InnoSetup`):

```powershell
.\scripts\package.ps1
```

Run `target\release\felaris-setup.exe` to install for the current user: a
standard setup wizard with Start menu and optional desktop shortcuts, an entry in
Windows Installed apps with a real uninstaller, and no administrator prompt. The
source is `installer\felaris.iss`. For unattended installs use
`felaris-setup.exe /VERYSILENT /NOLAUNCH`; add `/DIR="path"` to choose a folder.
The portable app, `target\release\felaris.exe`, runs from anywhere without installing.
After installation, choose **Set default browser?** from the sidebar menu, then
select Felaris in Windows Settings. Felaris registers HTTP, HTTPS, and HTML
file support; Windows keeps the final choice with you.
Uninstall from Windows Settings to remove the app and shortcuts. Browser data
is preserved. Running the new setup over an install from before the wizard
replaces it and removes the old Installed apps entry.

### Verification and signing

`package.ps1` verifies Windows icons/version resources, installs and uninstalls the built
setup silently, and runs the release app
headlessly against local fixtures using a temporary profile. The tests cover
live tab form retention, native history, pop-ups, downloads, error/retry,
address and palette drafts, second-launch link handoff, forced-exit recovery,
and fallback from a corrupt session file. It then writes `target/release/SHA256SUMS`.
The same verification runs in Windows CI, which retains the app, setup, and checksums.

To sign with an existing certificate in your Windows user certificate store:

```powershell
.\scripts\package.ps1 -CertificateThumbprint YOUR_THUMBPRINT -TimestampServer YOUR_TIMESTAMP_URL
```

Without a signing certificate, the package remains unsigned. Checksums detect
file changes; they do not establish publisher identity. Packaging does not
publish a GitHub release or send a Discord message.

### Browser data and recovery

Browser data is stored in `%LOCALAPPDATA%\Felaris`. `session.json` and
`session.json.bak` retain tab URLs, workspaces, pins, and recently closed tabs.
`WebView2` contains persistent site storage. Restarting restores URLs, rather
than unsaved forms or per-tab navigation history. Uninstall preserves this data.

Page failures retain WebView2's native error page and navigation history; Reload
retries the address. Renderer failures preserve tab URLs for reload.
Camera, microphone, location, notifications, and clipboard-read requests show
an origin-labelled native prompt. Unknown permission types are denied. Standard
file pickers and downloads use WebView2's native interfaces.

For an isolated headless development check after `cargo build --locked`:

```powershell
python tests/native_smoke.py
python tests/native_regressions.py
```

The regression suite requires Node 22+ and checks normalized addresses, SPA
routes, Back after page failures, sleeping-tab replacement, and the production
Close Other Tabs command while a download is running. It also exercises native
keyboard input and the DOM fixture at desktop and minimum sizes. Both suites
use isolated profiles and hidden windows.

### One-Line Install (PowerShell)

Download the latest `felaris-setup.exe`, check it against the release `SHA256SUMS`,
and install it silently (add `-NoLaunch` to skip opening Felaris):

```powershell
irm https://raw.githubusercontent.com/khentmoba/Felaris/main/scripts/install.ps1 | iex
```

### Automatic Updates

- Felaris checks for a new version shortly after launch and every 6 hours while it runs.
  `felaris.exe --check-update` checks from the command line.
- When one is available, an "Update Available" banner appears in the sidebar.
- **Update now** saves your session, downloads the release, verifies it against the
  release `SHA256SUMS`, and restarts Felaris on the new version.
  - Installed copies run `felaris-setup.exe` silently, so Windows' version record stays correct.
  - Portable copies swap `felaris.exe` in place and restore the previous one if that fails.

---

## Releases & Distribution Pipeline

Felaris automates releases to both GitHub Releases and Discord (`#felaris-releases`):

```bash
# Test Discord webhook connectivity
python scripts/notify_discord.py --test

# Prepare metadata before committing and merging the release change
python scripts/build.py --prepare -s "Release summary"

# From a clean checkout of the checked commit on origin/main
python scripts/build.py --dry-run
python scripts/build.py
```

Python 3.11+ is required. Set the release version in `Cargo.toml` and update
`Cargo.lock` before preparation. Commit and merge `version.json` with that change,
then wait for CI. Publication checks the checkout against the current remote
`main`, requires successful Windows CI for that exact SHA, verifies the release
package, and pins the GitHub tag to the built SHA.
An existing tag, dirty checkout, mismatched metadata, or failed GitHub publication
stops the release. Discord runs only after GitHub succeeds, unless `--skip-github`
was explicitly requested. `--dry-run` only validates and prints the plan.
The PowerShell entry point forwards to this same implementation.

Webhook configuration is resolved from `DISCORD_WEBHOOK_URL`, `scripts/discord_webhook.txt` (gitignored), or `%LOCALAPPDATA%\Felaris\discord_webhook.txt` (shared across worktrees). See `.agents/skills/discord-release/SKILL.md`.

---

## License

MIT OR Apache-2.0
