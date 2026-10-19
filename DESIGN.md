# Felaris interface

## Direction

Operate mode: a Zen-inspired desktop sidebar, not a browser feature clone. Keep the page primary, keep navigation and workspace tabs uncluttered, and put workspace switching at the bottom. Retain native Windows/system typography, offline assets, and the lightweight Rust shell.

## Surfaces

- `src/ui.html`: 260px expanded sidebar; 56px compact rail. Navigation, address field, workspace heading, new-tab/search actions, pinned and ordinary tab groups, sleep state, and bottom workspace dock.
- `src/start_page.html`: quiet, left-aligned startup/Home page with eight shortcuts ranked by user visit frequency (`LOCALAPPDATA/Felaris/shortcuts.json`) and padded with default sites. Two-column shortcuts in narrow content panes; scrolling remains available at the minimum desktop window size.
- `src/new_tab.html`: Zen-style floating new-tab search in a separate transparent, normally hidden WebView. It covers the client area without replacing or modifying the current page.
- `src/browser.js`: shared address resolution and keyboard shortcuts. Embedded into both local pages and initialized before remote content. Management IPC from content requires the injected trusted-key handler's per-process token; ordinary page scripts do not receive that token through the public helper.

## Visual system

User-requested black theme: sidebar `#101011`, page `#0c0c0e`, neutral accent `#dedee3`, primary text `#f5f5f7`. Sidebar surfaces `#1b1b1e`, active tab `#2e2e33`, secondary text `#b2b2b8`. The native Windows title bar also uses dark mode. The floating search panel uses `#171719` over a translucent black scrim. Use solid fills rather than decorative gradients/glows. Mostly 8–10px control corners, 40px tab rows, 32px navigation buttons, and consistent authored stroke icons. Active state comes from fill and weight, not color alone. Focus rings are 2px; address/search fields instead highlight the containing form. Loading animation respects reduced-motion preferences.

## Interaction contracts

- Ctrl+T and the sidebar New tab action open the floating search palette, focused immediately, without creating a tab or reloading the current page. Enter creates a tab for the typed address or Brave Search query. Open-tab suggestions can be selected with arrows/Enter or a click to switch instead. Escape, the close button, or the backdrop dismisses without creating a blank tab and returns focus to content. Ctrl+Shift+T still restores immediately. The startup/Home page remains available separately.
- The palette owns native overlay bounds and blocks navigation to remote documents. Its canvas must stay transparent: root color-scheme is normal, with dark color-scheme scoped to the opaque panel. Reopening clears the draft; IME composition must not accidentally submit.

- Blurred HTTPS addresses display the complete hostname; focus exposes and selects the full URL. HTTP keeps its scheme visible. State updates must not overwrite an in-progress address edit or discard a focused tab button.
- Pinning is workspace-scoped and protects a tab from Close Others. Closing others affects only the current workspace and retains restoration history.
- Search filters tab titles and URLs within the selected workspace; Escape clears the filter and returns focus to its trigger.
- The compact rail uses title tooltips. Opening its actions menu expands the rail first rather than clipping a menu into a narrow WebView. Ctrl+L also expands it and focuses the address field.
- Loading changes Reload to Stop. Sleeping tabs use subdued text and a moon icon; never report invented MB savings.
- The user-highlighted four-tile Essentials dock is removed entirely, including compact-mode markup/styles. The separate Home page shortcuts remain.

## Research used

Two Gemini 3.8 Flash research subagents investigated mainstream browsers and Zen. Applied the structural patterns, not every proposed capability or unverified implementation detail.

- [Zen Workspaces](https://docs.zen-browser.app/user-manual/workspaces): bottom workspace switching and scoped tabs.
- [Zen Compact Mode](https://docs.zen-browser.app/user-manual/compact-mode): a content-first compact rail; full floating/edge-hover behavior is not implemented across Felaris's native child WebViews.
- [Zen desktop source](https://github.com/zen-browser/desktop): essentials/pinned/ordinary tab hierarchy as the reference, without importing Zen branding or source assets.
- [Chromium omnibox principles](https://www.chromium.org/user-experience/omnibox/): one search/address entry point and familiar keyboard behavior.
- [Edge sleeping tabs](https://blogs.windows.com/msedgedev/2020/12/09/sleeping-tabs-beta-performance/): quiet dormant-tab treatment instead of noisy resource claims.

## Motion

The focal interaction is the floating search palette: each invocation resolves into focus over a dimmed page in 220ms, and exits in 100ms before the native overlay hides. A new invocation cancels a pending exit so stale dismiss/submit IPC cannot fire. Reduced-motion mode skips these spatial effects and never delays submission or cancellation.

Tab rows preserve identity and use 200ms position transitions when the list changes; workspace entries stagger at most 64ms. Active/hover fills and press feedback take 100–160ms. Loading state updates do not rebuild unchanged tab or workspace controls. Menus and feedback toasts have short, bounded reveals. Native WebView resizing and remote websites are not animated; there are no new dependencies or idle animation loops. The existing loading indicator remains tied to actual loading and becomes static under reduced motion.

## Deliberate limits

No new UI dependency, fake settings, permission badges, private mode, downloads/bookmarks backend, audio controls, floating edge-hover sidebar, or multi-WebView tab engine. Only the new-tab palette is a floating overlay; suggestions are local open tabs, not a network autocomplete/history service. Tab/workspace state is still session-only. Switching tabs still reloads URLs in the single content WebView; this redesign does not imply independent live sessions or measured memory reclamation.
