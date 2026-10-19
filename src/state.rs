use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserState {
    pub tabs: Vec<Tab>,
    pub active_tab_id: usize,
    pub active_workspace_id: usize,
    pub workspaces: Vec<Workspace>,
    pub sidebar_collapsed: bool,
    pub is_loading: bool,
    #[serde(skip)]
    next_tab_id: usize,
    pub homepage: String,
    #[serde(skip)]
    pub closed_tabs: Vec<Tab>,
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
        };

        Self {
            tabs: vec![initial_tab],
            active_tab_id: 1,
            active_workspace_id: 1,
            workspaces,
            sidebar_collapsed: false,
            is_loading: false,
            next_tab_id: 2,
            homepage: "felaris://newtab".to_string(),
            closed_tabs: Vec::new(),
        }
    }

    pub fn add_tab(&mut self, url: Option<String>, is_pinned: bool) -> usize {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let target_url = url.unwrap_or_else(|| self.homepage.clone());
        let new_tab = Tab {
            id,
            workspace_id: self.active_workspace_id,
            title: if target_url == "felaris://newtab" {
                "New Tab".to_string()
            } else {
                target_url.clone()
            },
            url: target_url,
            is_pinned,
            is_sleeping: false,
        };
        self.tabs.push(new_tab);
        self.active_tab_id = id;
        id
    }

    pub fn close_tab(&mut self, tab_id: usize) -> Option<usize> {
        let idx = self.tabs.iter().position(|t| t.id == tab_id)?;
        let removed = self.tabs.remove(idx);
        self.closed_tabs.push(removed);

        if self.tabs.is_empty() {
            let new_id = self.add_tab(None, false);
            return Some(new_id);
        }

        if self.active_tab_id == tab_id {
            // Find another tab in the same workspace if possible
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

        Some(self.active_tab_id)
    }

    pub fn restore_closed_tab(&mut self) -> Option<usize> {
        if let Some(mut tab) = self.closed_tabs.pop() {
            tab.id = self.next_tab_id;
            self.next_tab_id += 1;
            tab.is_sleeping = false;
            let id = tab.id;
            self.tabs.push(tab);
            self.active_tab_id = id;
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
        self.sidebar_collapsed
    }

    pub fn toggle_sleep(&mut self, tab_id: usize) {
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

    pub fn clear_unpinned_tabs(&mut self) {
        let active_id = self.active_tab_id;
        self.tabs.retain(|t| t.is_pinned || t.id == active_id);
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == self.active_tab_id)
    }

    pub fn update_active_tab(&mut self, url: String, title: Option<String>) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == self.active_tab_id) {
            tab.url = url;
            if let Some(t) = title {
                if !t.is_empty() {
                    tab.title = t;
                }
            }
        }
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.is_loading = loading;
    }
}
