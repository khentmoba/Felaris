use std::path::{Path, PathBuf};

pub fn directory() -> PathBuf {
    std::env::var_os("FELARIS_PROFILE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .or_else(|| std::env::var_os("APPDATA"))
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join("Felaris")
        })
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    std::fs::create_dir_all(path.parent().unwrap())?;
    let temporary = path.with_extension("tmp");
    let mut file = std::fs::File::create(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(temporary, path)
}

pub fn allowed_url(value: &str) -> bool {
    value == "felaris://newtab"
        || url::Url::parse(value).is_ok_and(|url| matches!(url.scheme(), "http" | "https" | "file"))
}

pub fn startup_url(value: &str) -> Option<String> {
    if allowed_url(value) {
        return Some(value.into());
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return url::Url::from_file_path(path).ok().map(Into::into);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_writes_replace_the_previous_file() {
        let directory = std::env::temp_dir().join(format!("felaris-atomic-{}", std::process::id()));
        let path = directory.join("session.json");
        write_atomic(&path, b"first").unwrap();
        write_atomic(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        assert!(!path.with_extension("tmp").exists());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn external_links_reject_commands_and_accept_windows_html_paths() {
        assert!(startup_url("https://example.com/a?b=c").is_some());
        assert!(startup_url("javascript:alert(1)").is_none());
        assert!(startup_url("--install").is_none());
        #[cfg(windows)]
        assert_eq!(
            startup_url(r"C:\Browser tests\hello.html").as_deref(),
            Some("file:///C:/Browser%20tests/hello.html")
        );
    }
}
