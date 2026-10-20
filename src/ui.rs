pub const SIDEBAR_WIDTH_EXPANDED: f64 = 260.0;
pub const SIDEBAR_HOT_ZONE: f64 = 6.0;
pub const WINDOW_MARGIN: f64 = 6.0;
pub const CONTENT_RADIUS: f64 = 10.0;
pub const TOPBAR_HOT_ZONE: f64 = 4.0;
pub const TOPBAR_EXPANDED: f64 = 34.0;

pub fn get_ui_html() -> String {
    include_str!("ui.html").replace("__COMMON_SCRIPT__", include_str!("browser.js"))
}

pub fn get_new_tab_html() -> String {
    include_str!("new_tab.html").replace("__COMMON_SCRIPT__", include_str!("browser.js"))
}

pub fn get_topbar_html() -> &'static str {
    include_str!("topbar.html")
}
