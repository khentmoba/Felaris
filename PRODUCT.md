# Felaris

<!-- impeccable:product-schema 1 -->

## Platform

web

## Product Purpose

A lightweight Windows desktop browser, using a Rust shell and native WebView2 content rendering.

## Operating Context

Everyday desktop navigation, web search, vertical tabs, and switching between General, Dev & Code, and Deep Dive workspaces.

## Capabilities and Constraints

Preserve the existing Rust/wry/tao stack, sidebar, workspaces, new-tab shortcuts, tab restoration, and background-tab sleep controls. No new UI framework or external font/network dependency. Live tabs use independent WebViews and a shared persistent WebView2 profile. Tabs, workspaces, pins, and recent closed tabs are saved with backup recovery. Sleeping drops a page; waking or restarting reloads its saved URL. Native downloads, error recovery, site permission prompts, single-instance link handoff, and Windows default-browser registration are supported. Brave Search is the single search provider, explicitly chosen by the user. Search queries use `https://search.brave.com/search?q=` with URL encoding; suggestions use open tabs, bookmarks, saved history, and shortcuts; no remote autocomplete is used.

## Brand Commitments

Keep the Felaris name. The user explicitly requested Zen Browser's sidebar and floating new-tab search flow as the primary references, and a cohesive black theme.

## Product Principles

- Web content gets the space; browser controls stay readable and reachable.
- Prefer familiar browser interactions over novel gestures.
- Show actual state, not fabricated privacy, memory, or performance statistics.

## Open Decisions

Primary audience beyond desktop browser users remains an open decision. Bookmarks are a flat list (no folders). Private browsing is a Private profile backed by InPrivate WebViews that is never saved.
