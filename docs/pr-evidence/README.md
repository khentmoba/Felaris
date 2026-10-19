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
