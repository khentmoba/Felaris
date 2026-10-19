# Felaris

**Felaris** is a smooth, lightweight, and easy-to-use modern browser built in Rust.

Designed from the ground up to be ultra-fast, gentle on PC hardware, and free of background bloat.

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
│  ┌─ Modern Top Bar (78px) ──────────────────────────┐  │
│  │ [◀] [▶] [🔄] [⌂]  https://duckduckgo.com     [⚑] │  │
│  │ [Tab 1] [Tab 2 💤] [Tab 3] [+]                   │  │
│  └──────────────────────────────────────────────────┘  │
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
| `Ctrl + T` | Open new tab |
| `Ctrl + W` | Close active tab |
| `Ctrl + L` | Focus and highlight address / search bar |
| `Ctrl + R` / `F5` | Reload current page |
| `Alt + Left` | Navigate back |
| `Alt + Right` | Navigate forward |
| `Enter` (in URL bar) | Smart navigate (detects URLs vs. web search) |

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

## Upstream Synchronization & Conflict Detection

Felaris includes a built-in upstream inspector that simulates upstream branch merges in-memory without dirtying your working tree:

```bash
# Using Python
python scripts/upstream_sync.py --upstream <remote_or_url> --branch main

# Or via PowerShell wrapper
.\scripts\upstream-sync.ps1 -Upstream <remote_or_url> -Branch main
```

A dedicated agent skill is available at `.agents/skills/upstream-sync/SKILL.md` (and `.pi/skills/upstream-sync/SKILL.md`) to guide automated synchronization and conflict audits.

---

## License

MIT OR Apache-2.0
