use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Mutex, OnceLock};

pub const ENGINES: [(&str, &str, &str); 4] = [
    (
        "brave",
        "Brave Search",
        "https://search.brave.com/search?q=",
    ),
    ("duckduckgo", "DuckDuckGo", "https://duckduckgo.com/?q="),
    ("google", "Google", "https://www.google.com/search?q="),
    ("bing", "Bing", "https://www.bing.com/search?q="),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub search_engine: String,
    pub ad_blocker: bool,
    pub auto_update: bool,
    pub save_history: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            search_engine: "brave".into(),
            ad_blocker: true,
            auto_update: true,
            save_history: true,
        }
    }
}

fn current() -> &'static Mutex<Settings> {
    static CURRENT: OnceLock<Mutex<Settings>> = OnceLock::new();
    CURRENT.get_or_init(|| {
        let loaded = std::fs::read_to_string(path())
            .ok()
            .and_then(|json| serde_json::from_str::<Settings>(&json).ok())
            .unwrap_or_default();
        Mutex::new(loaded)
    })
}

fn path() -> std::path::PathBuf {
    crate::profile::directory().join("settings.json")
}

pub fn get() -> Settings {
    current()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

pub fn engine() -> (&'static str, &'static str) {
    let id = get().search_engine;
    let (_, name, url) = ENGINES
        .iter()
        .find(|(key, ..)| *key == id)
        .unwrap_or(&ENGINES[0]);
    (name, url)
}

pub fn set(key: &str, value: &Value) -> Result<Settings, String> {
    let mut settings = get();
    match (key, value) {
        ("search_engine", Value::String(id)) if ENGINES.iter().any(|(key, ..)| key == id) => {
            settings.search_engine = id.clone()
        }
        ("ad_blocker", Value::Bool(on)) => settings.ad_blocker = *on,
        ("auto_update", Value::Bool(on)) => settings.auto_update = *on,
        ("save_history", Value::Bool(on)) => settings.save_history = *on,
        _ => return Err(format!("Unknown setting: {key}")),
    }
    let bytes = serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?;
    crate::profile::write_atomic(&path(), &bytes).map_err(|error| error.to_string())?;
    *current()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = settings.clone();
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let settings: Settings = serde_json::from_str(r#"{"search_engine":"google"}"#).unwrap();
        assert_eq!(settings.search_engine, "google");
        assert!(settings.ad_blocker && settings.auto_update && settings.save_history);
    }

    #[test]
    fn unknown_engines_and_keys_are_rejected() {
        assert!(set("search_engine", &Value::String("evil".into())).is_err());
        assert!(set("nope", &Value::Bool(true)).is_err());
        assert!(set("ad_blocker", &Value::String("yes".into())).is_err());
    }
}
