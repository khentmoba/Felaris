use serde::{Deserialize, Serialize};

use crate::updater::UpdateManifest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: usize,
    pub name: String,
    pub icon: String,
    pub accent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: usize,
    pub workspace_id: usize,
    pub title: String,
    pub url: String,
    pub is_pinned: bool,
    pub is_sleeping: bool,
    #[serde(default)]
    pub is_loading: bool,
    #[serde(default)]
    pub favicon: Option<String>,
    #[serde(default)]
    pub can_go_back: bool,
    #[serde(default)]
    pub can_go_forward: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(skip)]
    pub generation: u64,
    #[serde(skip)]
    pub pending_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shortcut {
    pub host: String,
    pub title: String,
    pub url: String,
    pub visits: usize,
    #[serde(default)]
    pub favicon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserState {
    pub tabs: Vec<Tab>,
    pub active_tab_id: usize,
    pub active_workspace_id: usize,
    pub workspaces: Vec<Workspace>,
    pub sidebar_collapsed: bool,
    #[serde(default)]
    pub sidebar_peek: bool,
    #[serde(skip)]
    pub topbar_peek: bool,
    pub is_loading: bool,
    pub can_restore_closed_tab: bool,
    pub shortcuts: Vec<Shortcut>,
    #[serde(skip)]
    next_tab_id: usize,
    pub homepage: String,
    #[serde(default)]
    pub closed_tabs: Vec<Tab>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_available: Option<UpdateManifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_status: Option<String>,
    #[serde(default)]
    pub active_downloads: usize,
    #[serde(default)]
    pub update_busy: bool,
    #[serde(default)]
    pub zoom: u32,
}

impl BrowserState {
    pub fn new() -> Self {
        let workspaces = vec![
            Workspace {
                id: 1,
                name: "General".to_string(),
                icon: "🪐".to_string(),
                accent: "#38bdf8".to_string(),
            },
            Workspace {
                id: 2,
                name: "Dev & Code".to_string(),
                icon: "⚡".to_string(),
                accent: "#a855f7".to_string(),
            },
            Workspace {
                id: 3,
                name: "Deep Dive".to_string(),
                icon: "📖".to_string(),
                accent: "#34d399".to_string(),
            },
        ];

        let initial_tab = Tab {
            id: 1,
            workspace_id: 1,
            title: "New Tab".to_string(),
            url: "felaris://newtab".to_string(),
            is_pinned: false,
            is_sleeping: false,
            is_loading: false,
            favicon: None,
            can_go_back: false,
            can_go_forward: false,
            error: None,
            generation: 0,
            pending_url: None,
        };

        Self {
            tabs: vec![initial_tab],
            active_tab_id: 1,
            active_workspace_id: 1,
            workspaces,
            sidebar_collapsed: false,
            sidebar_peek: false,
            topbar_peek: false,
            is_loading: false,
            can_restore_closed_tab: false,
            shortcuts: Self::load_shortcuts(),
            next_tab_id: 2,
            homepage: "felaris://newtab".to_string(),
            closed_tabs: Vec::new(),
            update_available: None,
            update_status: None,
            active_downloads: 0,
            update_busy: false,
            zoom: 100,
        }
    }

    pub fn restore() -> Self {
        let path = crate::profile::directory().join("session.json");
        for candidate in [path.clone(), path.with_extension("json.bak")] {
            if let Ok(json) = std::fs::read_to_string(&candidate)
                && let Some(mut state) = Self::from_session(&json)
            {
                if candidate != path {
                    state.update_status =
                        Some("Recovered your previous session from a backup.".into());
                }
                return state;
            }
        }
        let mut state = Self::new();
        if path.exists() {
            state.update_status =
                Some("Your saved session could not be read. A new session was opened.".into());
        }
        state
    }

    fn from_session(json: &str) -> Option<Self> {
        let mut state: Self = serde_json::from_str(json).ok()?;
        state.workspaces = Self::new().workspaces;
        let mut ids = std::collections::HashSet::new();
        state.tabs.retain(|tab| {
            tab.id > 0
                && tab.id < usize::MAX - 1
                && (1..=3).contains(&tab.workspace_id)
                && crate::profile::allowed_url(&tab.url)
                && ids.insert(tab.id)
        });
        if state.tabs.is_empty() {
            return None;
        }
        for tab in &mut state.tabs {
            tab.is_loading = false;
            tab.can_go_back = false;
            tab.can_go_forward = false;
            tab.error = None;
            tab.is_sleeping = tab.id != state.active_tab_id;
        }
        if !state.tabs.iter().any(|tab| tab.id == state.active_tab_id) {
            state.active_tab_id = state.tabs[0].id;
        }
        state.active_workspace_id = state.active_tab()?.workspace_id;
        state
            .tabs
            .iter_mut()
            .find(|tab| tab.id == state.active_tab_id)?
            .is_sleeping = false;
        state.next_tab_id = state.tabs.iter().map(|tab| tab.id).max()?.checked_add(1)?;
        state.closed_tabs.retain(|tab| {
            crate::profile::allowed_url(&tab.url) && (1..=3).contains(&tab.workspace_id)
        });
        state.closed_tabs.truncate(30);
        state.can_restore_closed_tab = !state.closed_tabs.is_empty();
        state.is_loading = false;
        state.active_downloads = 0;
        state.update_busy = false;
        state.zoom = 100;
        state.update_status = None;
        state.update_available = None;
        state.homepage = "felaris://newtab".into();
        Some(state)
    }

    pub fn save_session(&self) -> Result<(), String> {
        let path = crate::profile::directory().join("session.json");
        if let Ok(previous) = std::fs::read(&path)
            && std::str::from_utf8(&previous)
                .ok()
                .and_then(Self::from_session)
                .is_some()
        {
            crate::profile::write_atomic(&path.with_extension("json.bak"), &previous)
                .map_err(|error| error.to_string())?;
        }
        let bytes = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        crate::profile::write_atomic(&path, &bytes).map_err(|error| error.to_string())
    }

    pub fn add_tab(&mut self, url: Option<String>, is_pinned: bool) -> usize {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let target_url = url
            .map(|value| crate::profile::normalize_url(&value).unwrap_or(value))
            .unwrap_or_else(|| self.homepage.clone());
        let new_tab = Tab {
            id,
            workspace_id: self.active_workspace_id,
            title: match target_url.as_str() {
                "felaris://newtab" => "New Tab".to_string(),
                "felaris://settings" => "Settings".to_string(),
                "felaris://history" => "History".to_string(),
                _ => target_url.clone(),
            },
            url: target_url.clone(),
            is_pinned,
            is_sleeping: false,
            is_loading: false,
            favicon: None,
            can_go_back: false,
            can_go_forward: false,
            error: None,
            generation: 0,
            pending_url: None,
        };
        self.tabs.push(new_tab);
        self.active_tab_id = id;
        self.record_visit(&target_url);
        id
    }

    pub fn open_internal(&mut self, url: &str) {
        if let Some(id) = self
            .tabs
            .iter()
            .find(|tab| tab.url == url)
            .map(|tab| tab.id)
        {
            self.switch_tab(id);
        } else {
            self.add_tab(Some(url.into()), false);
        }
    }

    pub fn close_tab(&mut self, tab_id: usize) -> Option<usize> {
        let idx = self.tabs.iter().position(|t| t.id == tab_id)?;
        let removed = self.tabs.remove(idx);
        self.closed_tabs.push(removed);
        if self.closed_tabs.len() > 30 {
            self.closed_tabs.remove(0);
        }
        self.can_restore_closed_tab = true;

        if self.tabs.is_empty() {
            let new_id = self.add_tab(None, false);
            return Some(new_id);
        }

        if self.active_tab_id == tab_id {
            let same_workspace: Vec<usize> = self
                .tabs
                .iter()
                .filter(|t| t.workspace_id == self.active_workspace_id)
                .map(|t| t.id)
                .collect();

            if !same_workspace.is_empty() {
                self.active_tab_id = *same_workspace.last().unwrap();
            } else {
                self.active_tab_id = self.tabs.last().unwrap().id;
                self.active_workspace_id = self.active_tab().unwrap().workspace_id;
            }
        }

        self.switch_tab(self.active_tab_id);
        Some(self.active_tab_id)
    }

    pub fn restore_closed_tab(&mut self) -> Option<usize> {
        if let Some(mut tab) = self.closed_tabs.pop() {
            tab.id = self.next_tab_id;
            self.next_tab_id += 1;
            tab.is_sleeping = false;
            tab.is_loading = false;
            tab.can_go_back = false;
            tab.can_go_forward = false;
            tab.error = None;
            tab.pending_url = None;
            let id = tab.id;
            self.active_workspace_id = tab.workspace_id;
            self.tabs.push(tab);
            self.active_tab_id = id;
            self.can_restore_closed_tab = !self.closed_tabs.is_empty();
            Some(id)
        } else {
            None
        }
    }

    pub fn switch_tab(&mut self, tab_id: usize) -> Option<&Tab> {
        if let Some(t) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            t.is_sleeping = false;
            self.active_tab_id = tab_id;
            self.active_workspace_id = t.workspace_id;
        }
        self.active_tab()
    }

    pub fn switch_workspace(&mut self, workspace_id: usize) -> Option<&Tab> {
        if !self
            .workspaces
            .iter()
            .any(|workspace| workspace.id == workspace_id)
        {
            return self.active_tab();
        }
        self.active_workspace_id = workspace_id;
        let is_current_in_workspace = self
            .active_tab()
            .map(|t| t.workspace_id == workspace_id)
            .unwrap_or(false);

        if is_current_in_workspace {
            return self.active_tab();
        }

        let existing = self
            .tabs
            .iter()
            .find(|t| t.workspace_id == workspace_id)
            .map(|t| t.id);

        let target_id = if let Some(id) = existing {
            id
        } else {
            self.add_tab(None, false)
        };
        self.switch_tab(target_id)
    }

    pub fn toggle_sidebar(&mut self) -> bool {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        self.sidebar_peek = false;
        self.sidebar_collapsed
    }

    pub fn toggle_sleep(&mut self, tab_id: usize) {
        if tab_id == self.active_tab_id {
            return;
        }
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.is_sleeping = !tab.is_sleeping;
        }
    }

    pub fn sleep_inactive_tabs(&mut self) {
        for tab in self.tabs.iter_mut() {
            if tab.id != self.active_tab_id {
                tab.is_sleeping = true;
            }
        }
    }

    pub fn toggle_pin(&mut self, tab_id: usize) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            tab.is_pinned = !tab.is_pinned;
        }
    }

    pub fn cycle_tab(&mut self, backwards: bool) -> Option<&Tab> {
        let ids: Vec<usize> = self
            .tabs
            .iter()
            .filter(|tab| tab.workspace_id == self.active_workspace_id)
            .map(|tab| tab.id)
            .collect();
        let index = ids.iter().position(|id| *id == self.active_tab_id)?;
        let next = if backwards {
            (index + ids.len() - 1) % ids.len()
        } else {
            (index + 1) % ids.len()
        };
        self.switch_tab(ids[next])
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == self.active_tab_id)
    }

    pub fn update_active_tab(&mut self, url: String, title: Option<String>) {
        let should_count = self
            .active_tab()
            .map(|tab| title.is_none() && tab.url != url)
            .unwrap_or(false);
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == self.active_tab_id) {
            tab.url = url.clone();
            if let Some(t) = title
                && !t.is_empty()
            {
                tab.title = t;
            }
        }
        if should_count {
            self.record_visit(&url);
        }
    }

    pub fn top_shortcuts(&self) -> Vec<Shortcut> {
        let mut items = self.shortcuts.clone();
        items.sort_by_key(|a| std::cmp::Reverse(a.visits));
        for fallback in Self::default_shortcuts() {
            if items.len() >= 8 {
                break;
            }
            if !items.iter().any(|item| item.host == fallback.host) {
                items.push(fallback);
            }
        }
        items.truncate(8);
        items
    }

    pub fn set_favicon(&mut self, id: usize, favicon: Option<String>) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) else {
            return;
        };
        tab.favicon = favicon.clone();
        let (Some(favicon), Some((host, ..))) = (favicon, Self::shortcut_origin(&tab.url)) else {
            return;
        };
        for tab in &mut self.tabs {
            if tab.favicon.is_none()
                && Self::shortcut_origin(&tab.url).is_some_and(|origin| origin.0 == host)
            {
                tab.favicon = Some(favicon.clone());
            }
        }
        if let Some(item) = self.shortcuts.iter_mut().find(|item| item.host == host)
            && item.favicon.as_ref() != Some(&favicon)
        {
            item.favicon = Some(favicon);
            self.save_shortcuts();
        }
    }

    fn record_visit(&mut self, url: &str) {
        let Some((host, root_url, label)) = Self::shortcut_origin(url) else {
            return;
        };
        if let Some(index) = self.shortcuts.iter().position(|item| item.host == host) {
            let mut item = self.shortcuts.remove(index);
            item.visits = item.visits.saturating_add(1);
            item.url = root_url;
            item.title = label;
            self.shortcuts.insert(0, item);
        } else {
            self.shortcuts.insert(
                0,
                Shortcut {
                    host,
                    title: label,
                    url: root_url,
                    visits: 1,
                    favicon: None,
                },
            );
        }
        self.shortcuts.truncate(40);
        self.save_shortcuts();
    }

    fn shortcut_origin(url: &str) -> Option<(String, String, String)> {
        let (scheme, rest) = url.split_once("://")?;
        if !matches!(scheme, "http" | "https") {
            return None;
        }
        let authority = rest.split(['/', '?', '#']).next()?.trim();
        let host_port = authority.rsplit('@').next()?.to_ascii_lowercase();
        if host_port.is_empty() {
            return None;
        }
        let raw_host = host_port.strip_prefix("www.").unwrap_or(&host_port);
        let (host, root_url) = match raw_host {
            "m.youtube.com" => ("youtube.com", "https://youtube.com".to_string()),
            "twitter.com" | "mobile.twitter.com" => ("x.com", "https://x.com".to_string()),
            "old.reddit.com" => ("reddit.com", "https://reddit.com".to_string()),
            "en.wikipedia.org" => ("wikipedia.org", "https://wikipedia.org".to_string()),
            "doc.rust-lang.org" => (
                "doc.rust-lang.org",
                "https://doc.rust-lang.org/book/".to_string(),
            ),
            "search.brave.com" | "duckduckgo.com" => return None,
            _ => (raw_host, format!("{scheme}://{raw_host}")),
        };
        let label = Self::label_for_host(host);
        Some((host.to_string(), root_url, label))
    }

    fn label_for_host(host: &str) -> String {
        match host {
            "youtube.com" | "m.youtube.com" => "YouTube".to_string(),
            "x.com" | "twitter.com" => "X".to_string(),
            "github.com" => "GitHub".to_string(),
            "doc.rust-lang.org" => "Rust Docs".to_string(),
            "crates.io" => "Crates.io".to_string(),
            "news.ycombinator.com" => "Hacker News".to_string(),
            "wikipedia.org" | "en.wikipedia.org" => "Wikipedia".to_string(),
            "reddit.com" | "old.reddit.com" => "Reddit".to_string(),
            _ => {
                let stem = host.split(':').next().unwrap_or(host);
                let first = stem.split('.').next().unwrap_or(stem);
                let mut chars = first.chars();
                match chars.next() {
                    Some(_) if first.len() <= 2 => first.to_ascii_uppercase(),
                    Some(ch) => format!("{}{}", ch.to_ascii_uppercase(), chars.as_str()),
                    None => host.to_string(),
                }
            }
        }
    }

    fn default_shortcuts() -> Vec<Shortcut> {
        [
            ("github.com", "GitHub", "https://github.com"),
            (
                "doc.rust-lang.org",
                "Rust Docs",
                "https://doc.rust-lang.org/book/",
            ),
            ("crates.io", "Crates.io", "https://crates.io"),
            (
                "search.brave.com",
                "Brave Search",
                "https://search.brave.com",
            ),
            ("youtube.com", "YouTube", "https://youtube.com"),
            (
                "news.ycombinator.com",
                "Hacker News",
                "https://news.ycombinator.com",
            ),
            ("wikipedia.org", "Wikipedia", "https://wikipedia.org"),
            ("reddit.com", "Reddit", "https://reddit.com"),
        ]
        .into_iter()
        .map(|(host, title, url)| Shortcut {
            host: host.to_string(),
            title: title.to_string(),
            url: url.to_string(),
            visits: 0,
            favicon: None,
        })
        .collect()
    }

    fn shortcuts_path() -> Option<std::path::PathBuf> {
        if cfg!(test) {
            return None;
        }
        Some(crate::profile::directory().join("shortcuts.json"))
    }

    fn load_shortcuts() -> Vec<Shortcut> {
        let Some(path) = Self::shortcuts_path() else {
            return Vec::new();
        };
        std::fs::read_to_string(path)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    fn save_shortcuts(&self) {
        let Some(path) = Self::shortcuts_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string(&self.shortcuts) {
            let _ = crate::profile::write_atomic(&path, json.as_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_session_restores_pins_workspaces_and_unique_ids() {
        let mut state = BrowserState::new();
        state.toggle_pin(1);
        state.switch_workspace(2);
        let active = state.add_tab(Some("https://example.com/draft".into()), false);
        state.is_loading = true;
        state.active_downloads = 2;
        state.update_busy = true;
        let mut restored =
            BrowserState::from_session(&serde_json::to_string(&state).unwrap()).unwrap();
        assert_eq!(restored.active_tab_id, active);
        assert_eq!(restored.active_workspace_id, 2);
        assert!(restored.tabs[0].is_pinned);
        assert!(restored.tabs[0].is_sleeping);
        assert!(!restored.active_tab().unwrap().is_sleeping);
        assert!(!restored.is_loading);
        assert_eq!(restored.active_downloads, 0);
        assert!(!restored.update_busy);
        assert!(restored.add_tab(None, false) > active);
    }

    #[test]
    fn session_rejects_invalid_tabs_and_repairs_active_selection() {
        let mut state = BrowserState::new();
        state.add_tab(Some("javascript:alert(1)".into()), false);
        state.tabs.push(state.tabs[0].clone());
        state.active_tab_id = 999;
        let restored = BrowserState::from_session(&serde_json::to_string(&state).unwrap()).unwrap();
        assert_eq!(restored.tabs.len(), 1);
        assert_eq!(restored.active_tab_id, 1);
        assert_eq!(restored.active_workspace_id, 1);
        assert!(BrowserState::from_session("{truncated").is_none());
    }

    #[test]
    fn active_tab_cannot_be_put_to_sleep() {
        let mut state = BrowserState::new();
        state.toggle_sleep(1);
        assert!(!state.active_tab().unwrap().is_sleeping);
        state.switch_workspace(999);
        assert_eq!(state.active_workspace_id, 1);
    }

    #[test]
    fn closing_active_tab_wakes_its_replacement() {
        let mut state = BrowserState::new();
        let active = state.add_tab(Some("https://example.com".into()), false);
        state.sleep_inactive_tabs();
        state.close_tab(active);
        assert_eq!(state.active_tab_id, 1);
        assert!(!state.active_tab().unwrap().is_sleeping);
    }

    #[test]
    fn restoration_selects_original_workspace_and_wakes_tab() {
        let mut state = BrowserState::new();
        state.switch_workspace(2);
        let id = state.add_tab(Some("https://example.com".into()), false);
        state.close_tab(id);
        state.switch_workspace(1);
        state.restore_closed_tab().unwrap();
        let tab = state.active_tab().unwrap();
        assert_eq!(tab.workspace_id, state.active_workspace_id);
        assert_eq!(state.active_workspace_id, 2);
        assert!(!tab.is_sleeping);
        assert_ne!(tab.id, id);
    }

    #[test]
    fn cycling_wraps_within_workspace_and_wakes_tab() {
        let mut state = BrowserState::new();
        let second = state.add_tab(None, false);
        state.switch_workspace(2);
        state.switch_tab(second);
        state.sleep_inactive_tabs();
        assert_eq!(state.cycle_tab(false).unwrap().id, 1);
        assert!(!state.active_tab().unwrap().is_sleeping);
        assert_eq!(state.cycle_tab(true).unwrap().id, second);
    }

    #[test]
    fn closing_last_tab_creates_a_recoverable_new_tab() {
        let mut state = BrowserState::new();
        state.close_tab(1);
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.active_tab().unwrap().url, "felaris://newtab");
        assert!(state.can_restore_closed_tab);
        state.restore_closed_tab();
        assert_eq!(state.tabs.len(), 2);
        assert!(!state.can_restore_closed_tab);
    }

    #[test]
    fn shortcuts_rank_frequent_sites_and_keep_eight_tiles() {
        let mut state = BrowserState::new();
        for _ in 0..3 {
            state.add_tab(Some("https://www.youtube.com/watch?v=123".into()), false);
        }
        state.update_active_tab(
            "https://www.youtube.com/watch?v=123".into(),
            Some("YouTube Video".into()),
        );
        state.add_tab(Some("https://x.com/rustlang/status/1".into()), false);
        state.add_tab(Some("https://twitter.com/rustlang".into()), false);

        let shortcuts = state.top_shortcuts();
        assert_eq!(shortcuts.len(), 8);
        assert_eq!(shortcuts[0].host, "youtube.com");
        assert_eq!(shortcuts[0].title, "YouTube");
        assert_eq!(shortcuts[0].url, "https://youtube.com");
        assert_eq!(shortcuts[0].visits, 3);
        assert_eq!(shortcuts[1].host, "x.com");
        assert_eq!(shortcuts[1].title, "X");
        assert_eq!(shortcuts[1].url, "https://x.com");
        assert_eq!(shortcuts[1].visits, 2);
        assert_eq!(
            shortcuts
                .iter()
                .filter(|item| item.host == "youtube.com")
                .count(),
            1
        );
    }
}
