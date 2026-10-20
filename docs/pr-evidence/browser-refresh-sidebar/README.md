# Browser refresh and sidebar evidence

`sidebar-before.png` is the sidebar from base commit `083eab4`; `sidebar-after.png`
is this branch. Both come from `python -B tests/native_capture.py` against the
native WebView2 app with a local fixture page (no personal browsing data). The
fixture page has no favicon, so the after capture shows the globe fallback; the
before capture shows the old letter tile. The top bar WebView is not part of the
capture; its empty drag area is asserted by `tests/ui.html` and `tests/native_ui.mjs`.
