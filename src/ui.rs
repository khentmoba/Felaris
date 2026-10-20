use crate::{extensions, history, settings, state::Bookmark, updater};

pub const SIDEBAR_WIDTH_EXPANDED: f64 = 260.0;
pub const SIDEBAR_HOT_ZONE: f64 = 6.0;
pub const WINDOW_MARGIN: f64 = 6.0;
pub const CONTENT_RADIUS: f64 = 10.0;
pub const TOPBAR_HOT_ZONE: f64 = 4.0;
pub const TOPBAR_EXPANDED: f64 = 34.0;
pub const POPUP_BAR_HEIGHT: f64 = 42.0;

/// The shared page script, using the search engine chosen in Settings.
pub fn script() -> String {
    let (name, url) = settings::engine();
    include_str!("browser.js")
        .replace("'https://search.brave.com/search?q='", &format!("'{url}'"))
        .replace("'Brave Search'", &format!("'{name}'"))
}

pub fn search_script() -> String {
    let (name, url) = settings::engine();
    format!("Felaris.searchUrl='{url}';Felaris.searchName='{name}'")
}

pub fn get_ui_html() -> String {
    include_str!("ui.html").replace("__COMMON_SCRIPT__", &script())
}

pub fn get_new_tab_html() -> String {
    include_str!("new_tab.html").replace("__COMMON_SCRIPT__", &script())
}

pub fn get_history_html(bookmarks: &[Bookmark]) -> String {
    // One placeholder, filled last, so page titles can't collide with another one.
    let data = serde_json::json!({
        "history": history::all(),
        "bookmarks": bookmarks,
        "saving": settings::get().save_history,
    });
    include_str!("history.html")
        .replace("__COMMON_SCRIPT__", &script())
        .replace("__HISTORY__", &data.to_string().replace('<', "\\u003c"))
}

pub fn get_settings_html() -> String {
    let data = serde_json::json!({
        "settings": settings::get(),
        "engines": settings::ENGINES.iter().map(|(id, name, _)| [id, name]).collect::<Vec<_>>(),
        "ublock": extensions::status(),
        "ublock_active": extensions::active_at_startup(),
        "extensions_folder": extensions::directory(),
        "version": updater::CURRENT_VERSION,
    });
    include_str!("settings.html")
        .replace("__COMMON_SCRIPT__", &script())
        .replace("__SETTINGS__", &data.to_string().replace('<', "\\u003c"))
}

pub fn get_topbar_html() -> &'static str {
    include_str!("topbar.html")
}

pub fn get_popup_html() -> &'static str {
    include_str!("popup.html")
}
