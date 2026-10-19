# Desktop polish evidence

`sidebar-before.png` renders `src/ui.html` and `src/browser.js` from base commit
`6b233e3`. `sidebar-after.png` renders the implementation in this PR. Both use
headless Chromium at 260 x 800 pixels and the same synthetic tab, loading,
update, and download state. These are screenshots of the embedded sidebar HTML,
not screenshots of a native Windows window. No personal browsing data is shown.

Native behavior is checked separately by `tests/native_smoke.py`. It runs the
Rust/WebView2 executable with hidden windows, a temporary profile, and a local
HTTP fixture. It covers live form retention, history, pop-ups, file download,
page failure/retry, address focus after sidebar expansion, palette draft
retention, second-process link handoff, forced-exit session recovery, and
recovery from a corrupt session file through the saved backup.

`tests/native_regressions.py` drives the production sidebar IPC through a
loopback WebView2 debugging connection. Its hidden, isolated app checks URL
normalization, SPA route updates, Back after a failed navigation, waking a
sleeping replacement tab, and Close Other Tabs with pins, workspaces, undo,
and an active download. `tests/install.test.ps1` verifies that repeated setup
and uninstall preserve another browser's registration.
