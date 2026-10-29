use crate::{profile, updater};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

static INSTALLING: AtomicBool = AtomicBool::new(false);
static FAILED: AtomicBool = AtomicBool::new(false);

const RELEASE_API: &str = "https://api.github.com/repos/uBlockOrigin/uBOL-home/releases/latest";
const DOWNLOAD_PREFIX: &str = "https://github.com/uBlockOrigin/uBOL-home/releases/download/";

#[derive(Deserialize)]
struct Release {
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

/// Every folder in here is loaded by WebView2 as an unpacked extension.
pub fn directory() -> PathBuf {
    profile::directory().join("extensions")
}

fn ublock_directory() -> PathBuf {
    directory().join("ublock-origin-lite")
}

pub fn ublock_installed() -> bool {
    ublock_directory().join("manifest.json").is_file()
}

/// WebView2 cannot switch extensions on or off after startup, so the first answer sticks.
pub fn enabled_at_startup() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| crate::settings::get().ad_blocker)
}

pub fn active_at_startup() -> bool {
    static ACTIVE: OnceLock<bool> = OnceLock::new();
    *ACTIVE.get_or_init(|| enabled_at_startup() && ublock_installed())
}

pub fn status() -> &'static str {
    if ublock_installed() {
        "installed"
    } else if INSTALLING.load(Ordering::SeqCst) {
        "installing"
    } else if FAILED.load(Ordering::SeqCst) {
        "failed"
    } else {
        "missing"
    }
}

/// Downloads in the background unless already installed or in progress.
pub fn install_in_background(done: impl FnOnce(Result<(), String>) + Send + 'static) {
    if ublock_installed() || INSTALLING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let result = install_ublock();
        FAILED.store(result.is_err(), Ordering::SeqCst);
        INSTALLING.store(false, Ordering::SeqCst);
        done(result);
    });
}

/// uBlock Origin Lite is the MV3 build; WebView2's Chromium no longer runs MV2 extensions.
fn install_ublock() -> Result<(), String> {
    if ublock_installed() {
        return Ok(());
    }
    let json = updater::run_curl(&[
        "-s",
        "-f",
        "-L",
        "--max-time",
        "15",
        "-H",
        "User-Agent: Veluno-Browser",
        RELEASE_API,
    ])
    .ok_or("Could not reach GitHub to download the ad blocker.")?;
    let release: Release = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    let asset = release
        .assets
        .into_iter()
        .find(|asset| {
            asset.name.ends_with(".chromium.zip")
                && asset.browser_download_url.starts_with(DOWNLOAD_PREFIX)
        })
        .ok_or("The ad blocker release has no Chromium package.")?;
    let expected = asset
        .digest
        .as_deref()
        .and_then(|digest| digest.strip_prefix("sha256:"))
        .ok_or("The ad blocker release has no checksum.")?;

    // Outside `directory()` so WebView2 never sees a half-installed extension.
    let work = profile::directory().join("extensions-tmp");
    let _ = std::fs::remove_dir_all(&work);
    let unpacked = work.join("unpacked");
    std::fs::create_dir_all(&unpacked).map_err(|error| error.to_string())?;
    let zip = work.join("ublock.zip");
    let result = (|| {
        updater::download(&asset.browser_download_url, &zip)?;
        if !updater::sha256(&zip)?.eq_ignore_ascii_case(expected) {
            return Err("The ad blocker download did not match its checksum.".to_string());
        }
        let tar =
            std::path::Path::new(&std::env::var_os("SystemRoot").unwrap_or("C:\\Windows".into()))
                .join("System32")
                .join("tar.exe");
        let status = updater::hidden_command(tar.to_str().unwrap_or("tar.exe"))
            .arg("-xf")
            .arg(&zip)
            .arg("-C")
            .arg(&unpacked)
            .status()
            .map_err(|error| format!("Could not unpack the ad blocker: {error}"))?;
        if !status.success() || !unpacked.join("manifest.json").is_file() {
            return Err("The ad blocker package could not be unpacked.".to_string());
        }
        std::fs::create_dir_all(directory()).map_err(|error| error.to_string())?;
        let _ = std::fs::remove_dir_all(ublock_directory());
        std::fs::rename(&unpacked, ublock_directory()).map_err(|error| error.to_string())
    })();
    let _ = std::fs::remove_dir_all(&work);
    result
}
