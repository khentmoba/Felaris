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
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    let mut file = std::fs::File::create(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(temporary, path)
}

pub fn allowed_url(value: &str) -> bool {
    normalize_url(value).is_some()
}

pub fn is_internal(value: &str) -> bool {
    matches!(
        value,
        "felaris://newtab" | "felaris://blank" | "felaris://settings" | "felaris://history"
    )
}

pub fn normalize_url(value: &str) -> Option<String> {
    if is_internal(value) {
        return Some(value.into());
    }
    let url = url::Url::parse(value).ok()?;
    matches!(url.scheme(), "http" | "https" | "file").then(|| url.into())
}

pub fn is_auth_url(value: &str) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let path = url.path().to_ascii_lowercase();
    let query = url.query().unwrap_or_default().to_ascii_lowercase();
    matches!(
        host.as_str(),
        "accounts.google.com"
            | "login.microsoftonline.com"
            | "login.live.com"
            | "appleid.apple.com"
            | "id.atlassian.com"
    ) || host.starts_with("auth.")
        || host.starts_with("login.")
        || host.starts_with("sso.")
        || host.starts_with("id.")
        || host.ends_with(".auth0.com")
        || host.ends_with(".okta.com")
        || host.ends_with(".clerk.accounts.dev")
        || [
            "/oauth",
            "/authorize",
            "/login",
            "/signin",
            "/sign-in",
            "/auth/",
            "/sso",
            "/session",
            "/callback",
            "/confirm",
            "/installations/",
        ]
        .iter()
        .any(|needle| path.contains(needle))
        || [
            "client_id=",
            "redirect_uri=",
            "response_type=",
            "oauth_token=",
            "code_challenge=",
        ]
        .iter()
        .any(|needle| query.contains(needle))
}

pub fn startup_url(value: &str) -> Option<String> {
    if let Some(url) = normalize_url(value) {
        return Some(url);
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

    #[test]
    fn navigation_urls_match_browser_normalization() {
        assert_eq!(
            normalize_url("HTTPS://Example.COM").as_deref(),
            Some("https://example.com/")
        );
        assert_eq!(
            normalize_url("https://example.com/café").as_deref(),
            Some("https://example.com/caf%C3%A9")
        );
        assert_eq!(
            normalize_url("felaris://newtab").as_deref(),
            Some("felaris://newtab")
        );
        assert_eq!(
            normalize_url("felaris://settings").as_deref(),
            Some("felaris://settings")
        );
        assert!(normalize_url("felaris://other").is_none());
    }

    #[test]
    fn auth_urls_detect_oauth_login_and_installation_flows() {
        assert!(is_auth_url(
            "https://github.com/settings/installations/173231206"
        ));
        assert!(is_auth_url(
            "https://github.com/login/oauth/authorize?client_id=abc"
        ));
        assert!(is_auth_url(
            "https://accounts.google.com/o/oauth2/v2/auth?response_type=code"
        ));
        assert!(!is_auth_url("https://doc.rust-lang.org/book/"));
        assert!(is_auth_url("https://example.com/api/callback?code=1"));
        assert!(!is_auth_url("https://example.com/dashboard"));
        assert!(!is_auth_url("http://127.0.0.1:8080/popup"));
    }
}
