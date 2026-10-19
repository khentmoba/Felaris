use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GITHUB_REPO: &str = "khentmoba/Felaris";
pub const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/khentmoba/Felaris/main/version.json";
pub const GITHUB_LATEST_API: &str =
    "https://api.github.com/repos/khentmoba/Felaris/releases/latest";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn parse(s: &str) -> Option<Self> {
        let clean = s.trim().trim_start_matches(['v', 'V']);
        let core = clean.split('-').next()?;
        let mut parts = core.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next().unwrap_or("0").parse().ok()?;
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    pub release_url: String,
    pub download_url: Option<String>,
    pub summary: String,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    assets: Option<Vec<GhAsset>>,
}

pub fn cleanup_old_version() {
    if let Ok(current) = std::env::current_exe() {
        let old = current.with_extension("exe.old");
        if old.exists() {
            let _ = std::fs::remove_file(old);
        }
    }
}

pub fn check_for_updates() -> Option<UpdateManifest> {
    // 1. Try static manifest (fast, zero rate limits)
    if let Some(manifest) = fetch_manifest(MANIFEST_URL) {
        if is_newer(&manifest.version, CURRENT_VERSION) {
            return Some(manifest);
        }
        return None;
    }

    // 2. Fallback to GitHub Releases API
    if let Some(manifest) = fetch_github_latest()
        && is_newer(&manifest.version, CURRENT_VERSION)
    {
        return Some(manifest);
    }

    None
}

pub fn is_newer(remote: &str, current: &str) -> bool {
    match (Version::parse(remote), Version::parse(current)) {
        (Some(r), Some(c)) => r > c,
        _ => false,
    }
}

fn run_curl(args: &[&str]) -> Option<String> {
    let output = Command::new("curl.exe").args(args).output().ok()?;
    if output.status.success() {
        String::from_utf8(output.stdout).ok()
    } else {
        None
    }
}

fn fetch_manifest(url: &str) -> Option<UpdateManifest> {
    let json_str = run_curl(&["-s", "-f", "-L", "--max-time", "6", url])?;
    serde_json::from_str::<UpdateManifest>(&json_str).ok()
}

fn fetch_github_latest() -> Option<UpdateManifest> {
    let json_str = run_curl(&[
        "-s",
        "-f",
        "-L",
        "--max-time",
        "6",
        "-H",
        "User-Agent: Felaris-Browser",
        GITHUB_LATEST_API,
    ])?;

    let release = serde_json::from_str::<GhRelease>(&json_str).ok()?;
    let exe_asset = release.assets.as_ref().and_then(|assets| {
        assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(".exe"))
            .map(|a| a.browser_download_url.clone())
    });

    Some(UpdateManifest {
        version: release.tag_name.trim_start_matches(['v', 'V']).to_string(),
        release_url: release.html_url,
        download_url: exe_asset,
        summary: release
            .body
            .unwrap_or_else(|| "Felaris update available.".to_string()),
    })
}

pub fn download_and_apply_update(download_url: &str) -> Result<(), String> {
    let current_exe =
        std::env::current_exe().map_err(|e| format!("Cannot locate current executable: {e}"))?;
    let target_dir = current_exe
        .parent()
        .ok_or_else(|| "Invalid executable parent directory".to_string())?;

    let new_exe_temp = target_dir.join("felaris.exe.new");
    let old_exe = current_exe.with_extension("exe.old");

    // Download new binary using curl
    let status = Command::new("curl.exe")
        .args([
            "-s",
            "-f",
            "-L",
            "--max-time",
            "120",
            "-o",
            new_exe_temp.to_str().ok_or("Invalid temp path")?,
            download_url,
        ])
        .status()
        .map_err(|e| format!("Failed to spawn curl: {e}"))?;

    if !status.success() {
        let _ = std::fs::remove_file(&new_exe_temp);
        return Err(format!("Download failed with status: {status}"));
    }

    // Sanity check binary size (> 500 KB)
    let meta = std::fs::metadata(&new_exe_temp)
        .map_err(|e| format!("Cannot read downloaded file metadata: {e}"))?;
    if meta.len() < 500_000 {
        let _ = std::fs::remove_file(&new_exe_temp);
        return Err(format!(
            "Downloaded file too small ({} bytes); update aborted",
            meta.len()
        ));
    }

    // Windows atomic binary swap:
    // 1. Rename running felaris.exe -> felaris.exe.old
    // 2. Rename felaris.exe.new -> felaris.exe
    if old_exe.exists() {
        let _ = std::fs::remove_file(&old_exe);
    }
    std::fs::rename(&current_exe, &old_exe)
        .map_err(|e| format!("Failed to move running executable to .old: {e}"))?;

    if let Err(e) = std::fs::rename(&new_exe_temp, &current_exe) {
        // Attempt recovery
        let _ = std::fs::rename(&old_exe, &current_exe);
        return Err(format!(
            "Failed to replace executable with new version: {e}"
        ));
    }

    // Spawn new version
    Command::new(&current_exe)
        .spawn()
        .map_err(|e| format!("Failed to launch updated binary: {e}"))?;

    // Exit old process immediately
    std::process::exit(0);
}

pub fn get_install_paths() -> (PathBuf, PathBuf, PathBuf) {
    let local_app_data = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Users\\Default\\AppData\\Local"));

    let app_data = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Users\\Default\\AppData\\Roaming"));

    let user_profile = std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Users\\Default"));

    let target_dir = local_app_data.join("Programs").join("Felaris");
    let target_exe = target_dir.join("felaris.exe");

    let start_menu_shortcut = app_data
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Felaris.lnk");

    let desktop_shortcut = user_profile.join("Desktop").join("Felaris.lnk");

    (target_exe, start_menu_shortcut, desktop_shortcut)
}

fn create_shortcut(target_exe: &Path, shortcut_path: &Path) -> Result<(), String> {
    let ps_script = format!(
        "$ws = New-Object -ComObject WScript.Shell; $s = $ws.CreateShortcut('{}'); $s.TargetPath = '{}'; $s.Description = 'Felaris Browser'; $s.Save();",
        shortcut_path.display(),
        target_exe.display()
    );

    let status = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &ps_script,
        ])
        .status()
        .map_err(|e| format!("Failed to run powershell for shortcut: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Shortcut creation exited with: {status}"))
    }
}

pub fn install_to_system() -> Result<String, String> {
    let current_exe =
        std::env::current_exe().map_err(|e| format!("Cannot locate current executable: {e}"))?;
    let (target_exe, start_menu_shortcut, desktop_shortcut) = get_install_paths();
    let target_dir = target_exe.parent().unwrap();

    std::fs::create_dir_all(target_dir)
        .map_err(|e| format!("Failed to create install directory: {e}"))?;

    // Copy to install location if not running from there
    if current_exe != target_exe {
        std::fs::copy(&current_exe, &target_exe)
            .map_err(|e| format!("Failed to copy binary to install directory: {e}"))?;
    }

    // Create Start Menu & Desktop shortcuts
    let _ = create_shortcut(&target_exe, &start_menu_shortcut);
    let _ = create_shortcut(&target_exe, &desktop_shortcut);

    // Register with Windows Uninstall registry
    let reg_key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Felaris";
    let exe_str = target_exe.to_str().unwrap_or_default();
    let dir_str = target_dir.to_str().unwrap_or_default();
    let uninstall_cmd = format!("\"{exe_str}\" --uninstall");

    let _ = Command::new("reg.exe")
        .args(["add", reg_key, "/v", "DisplayName", "/d", "Felaris", "/f"])
        .status();
    let _ = Command::new("reg.exe")
        .args([
            "add",
            reg_key,
            "/v",
            "DisplayVersion",
            "/d",
            CURRENT_VERSION,
            "/f",
        ])
        .status();
    let _ = Command::new("reg.exe")
        .args([
            "add",
            reg_key,
            "/v",
            "Publisher",
            "/d",
            "Felaris Browser",
            "/f",
        ])
        .status();
    let _ = Command::new("reg.exe")
        .args(["add", reg_key, "/v", "DisplayIcon", "/d", exe_str, "/f"])
        .status();
    let _ = Command::new("reg.exe")
        .args(["add", reg_key, "/v", "InstallLocation", "/d", dir_str, "/f"])
        .status();
    let _ = Command::new("reg.exe")
        .args([
            "add",
            reg_key,
            "/v",
            "UninstallString",
            "/d",
            &uninstall_cmd,
            "/f",
        ])
        .status();

    Ok(format!(
        "Felaris installed successfully to {}",
        target_dir.display()
    ))
}

pub fn uninstall_from_system() -> Result<String, String> {
    let (_, start_menu_shortcut, desktop_shortcut) = get_install_paths();

    if start_menu_shortcut.exists() {
        let _ = std::fs::remove_file(start_menu_shortcut);
    }
    if desktop_shortcut.exists() {
        let _ = std::fs::remove_file(desktop_shortcut);
    }

    let reg_key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Felaris";
    let _ = Command::new("reg.exe")
        .args(["delete", reg_key, "/f"])
        .status();

    Ok("Felaris shortcuts and uninstall registry removed.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!(
            Version::parse("0.1.0"),
            Some(Version {
                major: 0,
                minor: 1,
                patch: 0
            })
        );
        assert_eq!(
            Version::parse("v1.2.3"),
            Some(Version {
                major: 1,
                minor: 2,
                patch: 3
            })
        );
        assert_eq!(
            Version::parse("V0.9.15-beta"),
            Some(Version {
                major: 0,
                minor: 9,
                patch: 15
            })
        );
        assert_eq!(Version::parse("invalid"), None);
    }

    #[test]
    fn test_is_newer() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("v0.1.1", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
    }

    #[test]
    fn test_parse_manifest_json() {
        let raw = r#"{
            "version": "0.2.0",
            "release_url": "https://github.com/khentmoba/Felaris/releases/tag/v0.2.0",
            "download_url": "https://github.com/khentmoba/Felaris/releases/download/v0.2.0/felaris.exe",
            "summary": "Zen UI update"
        }"#;

        let parsed: UpdateManifest = serde_json::from_str(raw).expect("valid manifest");
        assert_eq!(parsed.version, "0.2.0");
        assert_eq!(
            parsed.download_url.as_deref(),
            Some("https://github.com/khentmoba/Felaris/releases/download/v0.2.0/felaris.exe")
        );
        assert!(is_newer(&parsed.version, "0.1.0"));
    }
}
