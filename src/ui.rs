use crate::{extensions, history, settings, updater};

pub const SIDEBAR_WIDTH_EXPANDED: f64 = 260.0;
pub const SIDEBAR_WIDTH_COLLAPSED: f64 = 56.0;

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

pub fn get_history_html() -> String {
    include_str!("history.html")
        .replace("__COMMON_SCRIPT__", &script())
        .replace(
            "__HISTORY__",
            &serde_json::to_string(&history::all())
                .unwrap_or_else(|_| "[]".into())
                .replace('<', "\\u003c"),
        )
        .replace("__SAVING__", &settings::get().save_history.to_string())
}

pub fn get_settings_html() -> String {
    let data = serde_json::json!({
        "settings": settings::get(),
        "engines": settings::ENGINES.iter().map(|(id, name, _)| [id, name]).collect::<Vec<_>>(),
        "ublock": extensions::status(),
        "ublock_active": extensions::enabled_at_startup(),
        "extensions_folder": extensions::directory(),
        "version": updater::CURRENT_VERSION,
    });
    include_str!("settings.html")
        .replace("__COMMON_SCRIPT__", &script())
        .replace("__SETTINGS__", &data.to_string().replace('<', "\\u003c"))
}
