use crate::{profile, settings};
use serde::{Deserialize, Serialize};
use std::sync::{
    Mutex, MutexGuard, OnceLock, PoisonError,
    atomic::{AtomicBool, Ordering},
};

const MAX_ENTRIES: usize = 2000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub url: String,
    pub title: String,
    pub time: u64,
}

static DIRTY: AtomicBool = AtomicBool::new(false);

fn log() -> MutexGuard<'static, Vec<Entry>> {
    static LOG: OnceLock<Mutex<Vec<Entry>>> = OnceLock::new();
    LOG.get_or_init(|| {
        let loaded = std::fs::read_to_string(path())
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        Mutex::new(loaded)
    })
    .lock()
    .unwrap_or_else(PoisonError::into_inner)
}

fn path() -> std::path::PathBuf {
    profile::directory().join("history.json")
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}

/// Writes the log if it changed; the maintenance thread calls this so visits never block on disk.
pub fn flush() {
    static WRITING: Mutex<()> = Mutex::new(());
    let _writing = WRITING.lock().unwrap_or_else(PoisonError::into_inner);
    if !DIRTY.swap(false, Ordering::AcqRel) {
        return;
    }
    if let Ok(bytes) = serde_json::to_vec(&*log())
        && profile::write_atomic(&path(), &bytes).is_err()
    {
        DIRTY.store(true, Ordering::Release);
    }
}

fn changed() {
    DIRTY.store(true, Ordering::Release);
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
    let mut entries = log();
    record(&mut entries, url, title, now());
    changed();
}

/// The page title usually arrives after the visit is recorded.
pub fn retitle(url: &str, title: &str) {
    let mut entries = log();
    if let Some(latest) = entries.first_mut().filter(|latest| latest.url == url)
        && latest.title != title
    {
        latest.title = title.into();
        changed();
    }
}

pub fn all() -> Vec<Entry> {
    log().clone()
}

pub fn remove(url: &str) {
    let mut entries = log();
    entries.retain(|entry| entry.url != url);
    changed();
}

pub fn clear() {
    let mut entries = log();
    entries.clear();
    changed();
}

/// Removes visits from the last `range_ms` milliseconds; `None` removes everything.
pub fn clear_recent(range_ms: Option<u64>) {
    let mut entries = log();
    retain_older(
        &mut entries,
        range_ms.map(|range| now().saturating_sub(range)),
    );
    changed();
}

fn retain_older(entries: &mut Vec<Entry>, cutoff: Option<u64>) {
    match cutoff {
        Some(cutoff) => entries.retain(|entry| entry.time < cutoff),
        None => entries.clear(),
    }
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
    fn recent_clearing_keeps_older_visits() {
        let mut entries = Vec::new();
        for (n, time) in [10, 20, 30].into_iter().enumerate() {
            record(&mut entries, &format!("https://{n}.test/"), "t", time);
        }
        retain_older(&mut entries, Some(20));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].time, 10);
        retain_older(&mut entries, None);
        assert!(entries.is_empty());
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
