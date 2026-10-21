# Download-only tab cleanup

These are native WebView2 sidebar captures from isolated local fixtures at 260 x 450 CSS pixels. The localhost ports differ between runs. No user data or profiles are included. They show rendered sidebar content, not Windows composition.

Before, on the PR's original c9aabdd build, opening `/slow-download` in a new tab left four tabs and selected the empty download URL. The gated transfer was still active. `tests/native_downloads.mjs` failed with `Timed out: empty download tab removed while transfer is active`.

After, the same source-page fixture has three tabs, the source page selected, and the completed-download message. The empty tab was already removed while the transfer was active. The file then completed with all 65,536 expected bytes. The source input still contained `Keep my draft`. The screenshot comes from the first passing focused run; subsequent tests also cover an opener away from the end of the list, redirects, fast downloads, Home, saved sessions, and bounded retained download owners.

The fix keeps active/resumable native download owners hidden and at most one completed owner for Downloads. It does not use normal tab-close history, so Undo cannot inadvertently download the file again. Existing document downloads preserve their document, input, and address.

Run `cargo build --locked` followed by `python -B tests/native_downloads.py`. The same JavaScript driver also runs within `python -B tests/native_regressions.py` and Windows packaging CI.
