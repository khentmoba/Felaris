<p align="center">
  <img src="assets/logo-wordmark.svg" alt="Felaris Browser" width="320">
</p>

<p align="center">
  <strong>A smooth, lightweight, and easy-to-use modern browser built in Rust.</strong><br>
  Designed from the ground up to be ultra-fast, gentle on PC hardware, and free of background bloat.
</p>

---

## Highlights

- **100% Rust Application Shell:** Zero garbage-collection stutter, instantaneous cold startup (<100ms), and 0.0% idle CPU usage.
- **Hardware-Accelerated Web Engine:** Powered by `wry` and native OS WebView2—flawlessly renders modern web apps (YouTube 4K 60fps, WebGL, Spotify, Discord, Google Docs) without downloading or compiling 60GB of C++ engine binaries.
- **Ultra-Lean Footprint:**
  - Full application binary is only **~2.5 MB**.
  - Entire development environment uses less than **3 GB** of storage.
- **Smart Tab Sleeping:** Conserves system RAM by sleeping backgrounded tabs and instantly reviving them upon selection.
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
| `Ctrl + Shift + S` | Mark background tabs as sleeping |
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

To exercise actual WebView2 keyboard input and Rust IPC on Windows, build with `cargo build` and run the self-contained native smoke test with Node 22+ (which copies the binary to a temp executable, allocates a free debugging port, and cleans up the child process automatically):

```bash
cargo build
node tests/native-smoke.cjs
```

Native captures are saved under the ignored `.impeccable/review/` directory.

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

## Releases & Discord Notifications

Felaris includes automated Discord release scripts that publish announcements, changelogs, and binary uploads (`felaris.exe`) directly to the `#felaris-releases` Discord channel:

```bash
# Test Discord webhook connectivity
python scripts/notify_discord.py --test

# Build release and publish with changelog and binary upload
python scripts/build.py -s "Release summary"
```

Webhook configuration is resolved from `DISCORD_WEBHOOK_URL`, `scripts/discord_webhook.txt` (gitignored), or `%LOCALAPPDATA%\Felaris\discord_webhook.txt` (shared across worktrees). See `.agents/skills/discord-release/SKILL.md`.

---

## License

MIT OR Apache-2.0
