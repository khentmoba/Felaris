pub const SIDEBAR_WIDTH_EXPANDED: f64 = 260.0;
pub const SIDEBAR_WIDTH_COLLAPSED: f64 = 56.0;

pub fn get_ui_html() -> String {
    include_str!("ui.html").replace("__COMMON_SCRIPT__", include_str!("browser.js"))
}

pub fn get_new_tab_html() -> String {
    include_str!("new_tab.html").replace("__COMMON_SCRIPT__", include_str!("browser.js"))
}
