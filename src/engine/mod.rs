//! Page-content engines. Browser chrome (sidebar, palette, prompts) always uses WebView2;
//! only tab content goes through an engine. Engines report page activity back as
//! `UserEvent`s tagged with the tab id and generation, so the app never sees engine types.

#[cfg(feature = "servo")]
pub mod servo;
pub mod webview2;

use crate::{app::UserEvent, recovery};
use tao::{
    event::WindowEvent,
    event_loop::EventLoopProxy,
    window::{Window, WindowId},
};
use wry::Rect;

/// Everything an engine needs to create one tab's content view.
pub struct TabSpec {
    pub id: usize,
    pub generation: u64,
    pub private: bool,
    pub muted: bool,
    pub bounds: Rect,
    /// Injected into every document; carries the tab's shortcut token.
    pub script: String,
    pub token: String,
}

pub trait Engine {
    fn create_tab(&mut self, window: &Window, spec: TabSpec) -> Result<Box<dyn TabView>, String>;
    /// Input for engines that draw into their own window; returns true when consumed.
    fn window_event(&mut self, _window_id: WindowId, _event: &WindowEvent) -> bool {
        false
    }
    /// Runs pending engine work after `UserEvent::EngineWake`.
    fn wake(&mut self) {}
    /// Downloads in progress per tab id, for engines that track their own.
    fn downloads(&self) -> std::collections::HashMap<usize, usize> {
        std::collections::HashMap::new()
    }
}

/// Commands the app sends to a live tab.
pub trait TabView {
    /// `fresh` bypasses the HTTP cache for this navigation.
    fn load_url(&self, url: &str, fresh: bool) -> Result<(), String>;
    fn load_html(&self, html: &str) -> Result<(), String>;
    fn reload(&self) -> Result<(), String>;
    fn navigate(&self, backwards: bool);
    fn url(&self) -> Result<String, String>;
    fn evaluate_script(&self, script: &str) -> Result<(), String>;
    fn evaluate_script_with_callback(
        &self,
        script: &str,
        callback: Box<dyn Fn(String) + Send + 'static>,
    ) -> Result<(), String>;
    fn set_bounds(&self, bounds: Rect) -> Result<(), String>;
    /// Hidden tabs may also be asked to release memory.
    fn show(&self, visible: bool);
    fn round(&self, width: i32, height: i32, radius: i32);
    fn focus(&self) -> Result<(), String>;
    fn zoom(&self, scale: f64) -> Result<(), String>;
    fn print(&self) -> Result<(), String>;
    /// True when the page is known not to be playing audio.
    fn is_quiet(&self) -> bool;
    fn set_muted(&self, muted: bool) -> Result<(), String>;
    fn open_downloads(&self) -> Result<(), String>;
    fn apply_privacy(&self, _settings: &crate::settings::Settings) -> Result<(), String> {
        Err("These privacy settings are not supported by this engine yet.".into())
    }
    fn clear_browsing_data(
        &self,
        kinds: crate::desktop::DataKinds,
        _range_ms: Option<u64>,
        completed: Box<dyn FnOnce(Result<(), String>)>,
    ) -> Result<(), String> {
        if kinds.site_data || kinds.cache || kinds.passwords {
            return Err("Profile data clearing is not supported by this engine yet.".into());
        }
        completed(Ok(()));
        Ok(())
    }
    fn screenshot(
        &self,
        _title: &str,
        _completed: Box<dyn FnOnce(Result<std::path::PathBuf, String>)>,
    ) -> Result<(), String> {
        Err("Screenshots are not supported by this engine yet.".into())
    }
    /// Clears cookies and storage for `origin`, then sends `UserEvent::SiteDataCleared`.
    fn clear_site_data(
        &self,
        id: usize,
        generation: u64,
        origin: &str,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<(), String>;
    /// Clears cached site files for the page, then sends `UserEvent::SiteFilesCleared`.
    fn clear_site_files(
        &self,
        page: recovery::Page,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<(), String>;
    /// Reloads ignoring the cache; sends `UserEvent::FreshReloadFailed` on failure.
    fn reload_fresh(
        &self,
        page: recovery::Page,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<(), String>;
}

/// The content of the floating sign-in window. Dropping it closes the popup page.
pub trait PopupView {
    fn set_bounds(&self, bounds: Rect) -> Result<(), String>;
}

impl PopupView for wry::WebView {
    fn set_bounds(&self, bounds: Rect) -> Result<(), String> {
        wry::WebView::set_bounds(self, bounds).map_err(|error| error.to_string())
    }
}

/// Shared navigation policy: web pages and blank/inline documents only.
pub fn allow_navigation(url: &str, proxy: &EventLoopProxy<UserEvent>) -> bool {
    let allowed = crate::profile::allowed_url(url)
        || url == "about:blank"
        || url.starts_with("data:text/html;");
    if !allowed {
        let _ = proxy.send_event(UserEvent::Status(
            "This link uses an unsupported application protocol.".into(),
        ));
    }
    allowed
}
