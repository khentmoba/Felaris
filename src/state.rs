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
}

impl BrowserState {
    pub fn new() -> Self {
        let initial_tab = Tab {
            id: 1,
            title: "New Tab".to_string(),
            url: "https://duckduckgo.com".to_string(),
            is_sleeping: false,
        };

        Self {
            tabs: vec![initial_tab],
            active_tab_id: 1,
            next_tab_id: 2,
            homepage: "https://duckduckgo.com".to_string(),
        }
    }

    pub fn add_tab(&mut self, url: Option<String>) -> usize {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let target_url = url.unwrap_or_else(|| self.homepage.clone());
        let new_tab = Tab {
            id,
            title: "New Tab".to_string(),
            url: target_url,
            is_sleeping: false,
        };
        self.tabs.push(new_tab);
        self.active_tab_id = id;
        id
    }

    pub fn close_tab(&mut self, tab_id: usize) -> Option<usize> {
        if self.tabs.len() <= 1 {
            // Don't close the last tab, reset it to homepage
            if let Some(first) = self.tabs.first_mut() {
                first.url = self.homepage.clone();
                first.title = "New Tab".to_string();
                first.is_sleeping = false;
            }
            return Some(self.active_tab_id);
        }

        let idx = self.tabs.iter().position(|t| t.id == tab_id)?;
        self.tabs.remove(idx);

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

    pub fn switch_tab(&mut self, tab_id: usize) -> Option<&Tab> {
        if self.tabs.iter().any(|t| t.id == tab_id) {
            self.active_tab_id = tab_id;
            // Wake up if sleeping
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
            // Cannot sleep active tab
            if tab.id != self.active_tab_id {
                tab.is_sleeping = !tab.is_sleeping;
            }
        }
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == self.active_tab_id)
    }

    pub fn update_active_tab(&mut self, url: String, title: Option<String>) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == self.active_tab_id) {
            tab.url = url;
            if let Some(t) = title {
                tab.title = t;
            }
        }
    }
}
