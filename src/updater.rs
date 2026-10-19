use serde::{Deserialize, Serialize};
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

pub fn check_for_updates() -> Result<Option<UpdateManifest>, String> {
    // 1. Try static manifest (fast, zero rate limits)
    if let Some(manifest) = fetch_manifest(MANIFEST_URL) {
        if is_newer(&manifest.version, CURRENT_VERSION) {
            return Ok(Some(manifest));
        }
        return Ok(None);
    }

    let manifest = fetch_github_latest()
        .ok_or("Could not check for updates. Check your connection and try again.")?;
    Ok(is_newer(&manifest.version, CURRENT_VERSION).then_some(manifest))
}

pub fn is_newer(remote: &str, current: &str) -> bool {
    match (Version::parse(remote), Version::parse(current)) {
        (Some(r), Some(c)) => r > c,
        _ => false,
    }
}

fn run_curl(args: &[&str]) -> Option<String> {
    let output = hidden_command("curl.exe").args(args).output().ok()?;
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
            .find(|a| a.name.eq_ignore_ascii_case("felaris.exe"))
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
    validate_download_url(download_url)?;
    let status = hidden_command("curl.exe")
        .args([
            "-s",
            "-f",
            "-L",
            "--max-time",
            "120",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--connect-timeout",
            "10",
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

    let meta = std::fs::metadata(&new_exe_temp)
        .map_err(|e| format!("Cannot read downloaded file metadata: {e}"))?;
    if meta.len() < 500_000 {
        let _ = std::fs::remove_file(&new_exe_temp);
        return Err(format!(
            "Downloaded file too small ({} bytes); update aborted",
            meta.len()
        ));
    }

    if !valid_executable(&std::fs::read(&new_exe_temp).map_err(|error| error.to_string())?) {
        let _ = std::fs::remove_file(&new_exe_temp);
        return Err(
            "The downloaded file is not a compatible Windows app. Your current version was kept."
                .into(),
        );
    }

    replace_and_launch(&current_exe, &new_exe_temp)?;
    std::process::exit(0);
}

fn replace_and_launch(
    current_exe: &std::path::Path,
    new_exe: &std::path::Path,
) -> Result<(), String> {
    let old_exe = current_exe.with_extension("exe.old");
    if old_exe.exists() {
        std::fs::remove_file(&old_exe)
            .map_err(|error| format!("Could not clear the previous update backup: {error}"))?;
    }
    std::fs::rename(current_exe, &old_exe)
        .map_err(|error| format!("Could not back up the running app: {error}"))?;
    if let Err(error) = std::fs::rename(new_exe, current_exe) {
        std::fs::rename(&old_exe, current_exe).map_err(|recovery| {
            format!(
                "Update failed ({error}); recovery failed ({recovery}). Your previous app is at {}",
                old_exe.display()
            )
        })?;
        return Err(format!(
            "Update could not be installed; your previous version was restored: {error}"
        ));
    }
    if let Err(error) = Command::new(current_exe).arg("--updated").spawn() {
        std::fs::remove_file(current_exe).and_then(|_| std::fs::rename(&old_exe, current_exe))
            .map_err(|recovery| format!("The update could not start ({error}); recovery failed ({recovery}). Your previous app is at {}", old_exe.display()))?;
        return Err(format!(
            "The update could not start; your previous version was restored: {error}"
        ));
    }
    Ok(())
}

fn hidden_command(program: &str) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

fn validate_download_url(value: &str) -> Result<(), String> {
    let url = url::Url::parse(value).map_err(|_| "Invalid update address")?;
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || !url
            .path()
            .starts_with("/khentmoba/Felaris/releases/download/")
        || !url.path().ends_with("/felaris.exe")
        || url.port().is_some()
    {
        return Err("Update address is not an official Felaris release.".into());
    }
    Ok(())
}

fn valid_executable(bytes: &[u8]) -> bool {
    if bytes.len() < 64 || &bytes[..2] != b"MZ" {
        return false;
    }
    let offset = u32::from_le_bytes(bytes[60..64].try_into().unwrap()) as usize;
    let Some(header) = bytes.get(offset..offset.saturating_add(94)) else {
        return false;
    };
    let machine = u16::from_le_bytes([header[4], header[5]]);
    let expected = if cfg!(target_arch = "aarch64") {
        0xaa64
    } else {
        0x8664
    };
    &header[..4] == b"PE\0\0"
        && machine == expected
        && u16::from_le_bytes([header[92], header[93]]) == 2
}

fn installer_command(action: &str) -> Result<Command, String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            include_str!("install.ps1"),
        ])
        .env("FELARIS_INSTALL_ACTION", action)
        .env("FELARIS_SOURCE_EXE", current_exe)
        .env("FELARIS_VERSION", CURRENT_VERSION)
        .env("FELARIS_PARENT_PID", std::process::id().to_string());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    Ok(command)
}

pub fn install_to_system() -> Result<String, String> {
    run_installer("install")
}

fn run_installer(action: &str) -> Result<String, String> {
    let output = installer_command(action)?
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn uninstall_from_system() -> Result<String, String> {
    run_installer("uninstall")?;
    installer_command("cleanup")?
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start uninstaller: {e}"))?;
    Ok("Felaris uninstall started. Browser data is preserved.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn failed_update_launch_restores_the_previous_binary() {
        let directory =
            std::env::temp_dir().join(format!("felaris-rollback-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let current = directory.join("felaris.exe");
        let new = directory.join("felaris.exe.new");
        std::fs::write(&current, b"previous app").unwrap();
        std::fs::write(&new, b"not executable").unwrap();
        assert!(replace_and_launch(&current, &new).is_err());
        assert_eq!(std::fs::read(&current).unwrap(), b"previous app");
        assert!(!current.with_extension("exe.old").exists());
        assert!(replace_and_launch(&current, &new).is_err());
        assert_eq!(std::fs::read(&current).unwrap(), b"previous app");
        std::fs::remove_file(current).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn updates_only_accept_official_https_app_assets() {
        assert!(
            validate_download_url(
                "https://github.com/khentmoba/Felaris/releases/download/v0.2.0/felaris.exe"
            )
            .is_ok()
        );
        for address in [
            "http://github.com/khentmoba/Felaris/releases/download/v0.2.0/felaris.exe",
            "https://github.com.evil.test/khentmoba/Felaris/releases/download/v0.2.0/felaris.exe",
            "https://github.com/other/project/releases/download/v0.2.0/felaris.exe",
            "https://github.com/khentmoba/Felaris/releases/download/v0.2.0/felaris-setup.exe",
        ] {
            assert!(validate_download_url(address).is_err(), "{address}");
        }
    }

    #[test]
    fn update_validation_rejects_html_and_truncated_executables() {
        assert!(!valid_executable(&vec![0; 600_000]));
        assert!(!valid_executable(b"MZ"));
        let mut bytes = vec![0; 512];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&64u32.to_le_bytes());
        bytes[64..68].copy_from_slice(b"PE\0\0");
        let machine: u16 = if cfg!(target_arch = "aarch64") {
            0xaa64
        } else {
            0x8664
        };
        bytes[68..70].copy_from_slice(&machine.to_le_bytes());
        bytes[156..158].copy_from_slice(&2u16.to_le_bytes());
        assert!(valid_executable(&bytes));
        bytes[60..64].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(!valid_executable(&bytes));
    }

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
