use crate::state::Shortcut;

pub fn get_start_page_html(shortcuts: &[Shortcut]) -> String {
    let shortcuts_json = serde_json::to_string(shortcuts).unwrap_or_else(|_| "[]".to_string());
    include_str!("start_page.html")
        .replace("__COMMON_SCRIPT__", &crate::ui::script())
        .replace("__SHORTCUTS__", &shortcuts_json)
}
