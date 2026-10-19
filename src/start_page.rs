pub fn get_start_page_html() -> String {
    include_str!("start_page.html").replace("__COMMON_SCRIPT__", include_str!("browser.js"))
}
