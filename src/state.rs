use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: usize,
    pub title: String,
    pub url: String,
    pub is_sleeping: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserState {
    pub tabs: Vec<Tab>,
    pub active_tab_id: usize,
    #[serde(skip)]
    next_tab_id: usize,
    pub homepage: String,
    pub show_bookmarks: bool,
    pub is_loading: bool,
    #[serde(skip)]
    pub closed_tabs: Vec<Tab>,
}

impl BrowserState {
    pub fn new() -> Self {
        let initial_tab = Tab {
            id: 1,
            title: "New Tab".to_string(),
            url: "felaris://newtab".to_string(),
            is_sleeping: false,
        };

        Self {
            tabs: vec![initial_tab],
            active_tab_id: 1,
            next_tab_id: 2,
            homepage: "felaris://newtab".to_string(),
            show_bookmarks: false,
            is_loading: false,
            closed_tabs: Vec::new(),
        }
    }

    pub fn add_tab(&mut self, url: Option<String>) -> usize {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let target_url = url.unwrap_or_else(|| self.homepage.clone());
        let new_tab = Tab {
            id,
            title: if target_url == "felaris://newtab" {
                "New Tab".to_string()
            } else {
                target_url.clone()
            },
            url: target_url,
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
            // Keep at least one tab
            let new_id = self.add_tab(None);
            return Some(new_id);
        }

        if self.active_tab_id == tab_id {
            let new_idx = if idx >= self.tabs.len() {
                self.tabs.len() - 1
            } else {
                idx
            };
            self.active_tab_id = self.tabs[new_idx].id;
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
        if self.tabs.iter().any(|t| t.id == tab_id) {
            self.active_tab_id = tab_id;
            if let Some(t) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                t.is_sleeping = false;
            }
            self.active_tab()
        } else {
            None
        }
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

    pub fn toggle_bookmarks_shelf(&mut self) -> bool {
        self.show_bookmarks = !self.show_bookmarks;
        self.show_bookmarks
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
