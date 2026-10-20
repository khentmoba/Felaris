use crate::{profile, settings};
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};

const MAX_ENTRIES: usize = 2000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub url: String,
    pub title: String,
    pub time: u64,
}

fn log() -> &'static Mutex<Vec<Entry>> {
    static LOG: OnceLock<Mutex<Vec<Entry>>> = OnceLock::new();
    LOG.get_or_init(|| {
        let loaded = std::fs::read_to_string(path())
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        Mutex::new(loaded)
    })
}

fn path() -> std::path::PathBuf {
    profile::directory().join("history.json")
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}

fn save(entries: &[Entry]) {
    if let Ok(bytes) = serde_json::to_vec(entries) {
        let _ = profile::write_atomic(&path(), &bytes);
    }
}

/// Newest first; reloading the page at the top of the list only refreshes it.
fn record(entries: &mut Vec<Entry>, url: &str, title: &str, time: u64) {
    let title = if title.is_empty() { url } else { title };
    match entries.first_mut() {
        Some(latest) if latest.url == url => {
            latest.title = title.into();
            latest.time = time;
        }
        _ => {
            entries.insert(
                0,
                Entry {
                    url: url.into(),
                    title: title.into(),
                    time,
                },
            );
            entries.truncate(MAX_ENTRIES);
        }
    }
}

pub fn visit(url: &str, title: &str) {
    if !matches!(
        url::Url::parse(url)
            .map(|u| u.scheme().to_owned())
            .as_deref(),
        Ok("http" | "https")
    ) || !settings::get().save_history
    {
        return;
    }
    let mut entries = log().lock().unwrap();
    record(&mut entries, url, title, now());
    save(&entries);
}

/// The page title usually arrives after the visit is recorded.
pub fn retitle(url: &str, title: &str) {
    let mut entries = log().lock().unwrap();
    if let Some(latest) = entries.first_mut().filter(|latest| latest.url == url)
        && latest.title != title
    {
        latest.title = title.into();
        save(&entries);
    }
}

pub fn all() -> Vec<Entry> {
    log().lock().unwrap().clone()
}

pub fn remove(url: &str) {
    let mut entries = log().lock().unwrap();
    entries.retain(|entry| entry.url != url);
    save(&entries);
}

pub fn clear() {
    let mut entries = log().lock().unwrap();
    entries.clear();
    save(&entries);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_visit_leads_and_repeats_only_refresh() {
        let mut entries = Vec::new();
        record(&mut entries, "https://a.test/", "A", 1);
        record(&mut entries, "https://b.test/", "", 2);
        record(&mut entries, "https://b.test/", "B", 3);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title, "B");
        assert_eq!(entries[0].time, 3);
        assert_eq!(entries[1].url, "https://a.test/");
    }

    #[test]
    fn history_is_capped() {
        let mut entries = Vec::new();
        for n in 0..MAX_ENTRIES + 5 {
            record(&mut entries, &format!("https://{n}.test/"), "t", n as u64);
        }
        assert_eq!(entries.len(), MAX_ENTRIES);
        assert_eq!(entries[0].url, format!("https://{}.test/", MAX_ENTRIES + 4));
    }
}
