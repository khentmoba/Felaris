# Felaris

<!-- impeccable:product-schema 1 -->

## Platform

web

## Product Purpose

A lightweight Windows desktop browser, using a Rust shell and native WebView2 content rendering.

## Operating Context

Everyday desktop navigation, web search, vertical tabs, and switching between General, Dev & Code, and Deep Dive workspaces.

## Capabilities and Constraints

Preserve the existing Rust/wry/tao stack, sidebar, workspaces, new-tab shortcuts, tab restoration, and background-tab sleep controls. No new UI framework or external font/network dependency. The current engine uses one content WebView: switching tabs reloads their URLs rather than preserving independent live page sessions. State is currently session-only. Brave Search is the single search provider, explicitly chosen by the user. Search queries use `https://search.brave.com/search?q=` with URL encoding; suggestions remain local open tabs, with no remote autocomplete.

## Brand Commitments

Keep the Felaris name. The user explicitly requested Zen Browser's sidebar and floating new-tab search flow as the primary references, and a cohesive black theme.

## Product Principles

- Web content gets the space; browser controls stay readable and reachable.
- Prefer familiar browser interactions over novel gestures.
- Show actual state, not fabricated privacy, memory, or performance statistics.

## Open Decisions

Primary audience beyond desktop browser users, persistence, bookmarks, downloads management, and private browsing are not established requirements for this redesign.
