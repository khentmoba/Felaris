//! Where Servo downloads are saved. Files go to the user's Downloads folder without a prompt,
//! as in WebView2 tabs, and are marked as coming from the internet so SmartScreen checks them.

use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};

/// The user's Downloads folder.
pub(super) fn folder() -> Option<PathBuf> {
    use windows::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_Downloads, KF_FLAG_DEFAULT, SHGetKnownFolderPath},
    };
    unsafe {
        let path = SHGetKnownFolderPath(&FOLDERID_Downloads, KF_FLAG_DEFAULT, None).ok()?;
        let folder = path.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(path.0 as _));
        folder
    }
}

/// Creates an empty file for `suggested` in `folder`, numbering the name if it is taken, so
/// two downloads never write to the same file.
pub(super) fn reserve(folder: &Path, suggested: &str) -> Option<(PathBuf, File)> {
    let name = file_name(suggested);
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem, format!(".{extension}")),
        _ => (name.as_str(), String::new()),
    };
    (0..1000).find_map(|number| {
        let candidate = if number == 0 {
            folder.join(&name)
        } else {
            folder.join(format!("{stem} ({number}){extension}"))
        };
        use std::os::windows::fs::OpenOptionsExt;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .share_mode(1)
            .access_mode(0x40010000) // GENERIC_WRITE | DELETE
            .custom_flags(0x02000000) // FILE_FLAG_BACKUP_SEMANTICS
            .open(&candidate)
            .ok()
            .map(|file| (candidate, file))
    })
}

/// A safe Windows file name for a server-suggested one.
fn file_name(suggested: &str) -> String {
    let base = suggested.rsplit(['/', '\\']).next().unwrap_or_default();
    let mut name: String = base
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    name = name.trim_end_matches(['.', ' ']).trim_start().to_owned();
    if name.chars().count() > 180 {
        name = name
            .chars()
            .rev()
            .take(180)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit());
    if reserved {
        name.insert(0, '_');
    }
    if name.is_empty() {
        "download".into()
    } else {
        name
    }
}

/// Asks whether to cancel the running downloads.
pub(super) fn confirm_cancel(owner: isize, running: usize) -> bool {
    use windows::{
        Win32::{
            Foundation::HWND,
            UI::WindowsAndMessaging::{IDYES, MB_ICONQUESTION, MB_YESNO, MessageBoxW},
        },
        core::HSTRING,
    };
    let text = if running == 1 {
        "A download is still running. Cancel it?

Choose No to show the Downloads folder."
            .to_owned()
    } else {
        format!(
            "{running} downloads are still running. Cancel them?

Choose No to show the Downloads folder."
        )
    };
    unsafe {
        MessageBoxW(
            Some(HWND(owner as _)),
            &HSTRING::from(text),
            &HSTRING::from("Downloads — Felaris"),
            MB_YESNO | MB_ICONQUESTION,
        ) == IDYES
    }
}

/// Opens File Explorer with `path` selected, or the Downloads folder.
pub(super) fn reveal(path: Option<&Path>) -> Result<(), String> {
    let mut explorer = std::process::Command::new("explorer.exe");
    match path.filter(|path| path.exists()) {
        Some(path) => explorer.arg(format!("/select,{}", path.display())),
        None => explorer.arg(folder().ok_or("The Downloads folder could not be found.")?),
    };
    explorer
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::file_name;

    #[test]
    fn suggested_names_become_safe_file_names() {
        assert_eq!(file_name("report.pdf"), "report.pdf");
        assert_eq!(file_name("../../evil.exe"), "evil.exe");
        assert_eq!(file_name("C:\\Windows\\x.dll"), "x.dll");
        assert_eq!(file_name("a<b>:c?.txt"), "a_b__c_.txt");
        assert_eq!(file_name("name. . "), "name");
        assert_eq!(file_name("CON.txt"), "_CON.txt");
        assert_eq!(file_name("com1"), "_com1");
        assert_eq!(file_name("computer.txt"), "computer.txt");
        assert_eq!(file_name(""), "download");
        assert_eq!(file_name("..."), "download");
    }
}
