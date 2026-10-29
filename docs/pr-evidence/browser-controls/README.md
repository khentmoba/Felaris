# Browser controls evidence

These captures compare the sidebar HTML at `d0a3e59c533d48105ef6b99380b2c149b487dcf7` with this change. Both use the same synthetic tabs, workspace names, local fixture favicon, and history entries at 260 x 640 CSS pixels. The local port shown in the images belongs to the capture fixture. No user history, cookies, or browser profiles are included.

The sources ran in isolated srcdoc frames inside an owned WebView2 content view. Screenshot IPC was stubbed. These images show rendered UI, not native Windows composition or animation smoothness. The native regression suite tests actual sidebar geometry and stacking separately.

- `address-before.png` and `address-after.png`: tab and history suggestions replace generic icons with the site's favicon. The synthetic site uses the local Veluno SVG as its favicon.
- `workspaces-before.png` and `workspaces-after.png`: clearer naming, shared-login copy, and workspace rename/delete controls.
- `feedback-before.png` and `feedback-after.png`: feedback uses less vertical space; the after image also shows the muted-tab control.
- `site-controls-after.png`: full origin, HTTP connection explanation, and the entry point to confirmed data clearing. The baseline had a static connection indicator and no equivalent panel.

`capture-manifest.json` records source hashes and the baseline revision.

Runtime safety evidence is in `tests/native_controls.mjs`, called by `tests/native_ui.mjs`. It exercises the real sidebar IPC and WebView2 behavior. Workspace deletion keeps a live input draft and pin intact. A mismatched clear-data origin is rejected. Confirmed clearing removes the selected site's cookie and local storage while another origin retains both. Native audio events report mute state, and mute survives recreating a sleeping view. Native Alt+Shift+Arrow input reorders tabs. The DOM fixture checks drag routing, loaded icons, retries, and fallbacks. Rust tests reject reorder requests across workspaces or pin groups.

An earlier native run timed out waiting for the notification permission prompt. Isolated and subsequent full reruns passed. Its cause remains unproven; it was not addressed by changing assertions or adding delays. The final PR validation results are listed in the PR body. Installer lifecycle and release packaging are delegated to Windows CI.
