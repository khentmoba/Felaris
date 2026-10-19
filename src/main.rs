#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod desktop;
mod instance;
mod profile;
mod smoke;
mod start_page;
mod state;
mod ui;
mod updater;

fn has_option(args: &[String], option: &str) -> bool {
    args.iter()
        .skip(1)
        .take_while(|argument| argument.as_str() != "--")
        .any(|argument| argument == option)
}

fn main() -> wry::Result<()> {
    #[cfg(windows)]
    {
        #[link(name = "shell32")]
        unsafe extern "system" {
            fn SetCurrentProcessExplicitAppUserModelID(app_id: *const u16) -> i32;
        }
        let app_id: Vec<u16> = "Felaris.Browser\0".encode_utf16().collect();
        // Windows copies the ID during this call.
        unsafe { SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr()) };
    }
    let args: Vec<String> = std::env::args().collect();
    #[cfg(windows)]
    if has_option(&args, "--setup")
        || std::env::current_exe()
            .ok()
            .and_then(|path| path.file_name().map(|name| name.to_owned()))
            .is_some_and(|name| name.eq_ignore_ascii_case("felaris-setup.exe"))
    {
        let result = updater::install_to_system();
        let message = match &result {
            Ok(message) => format!("{message}\n\nOpen Felaris from the Start menu or desktop."),
            Err(error) => format!("Installation failed:\n{error}"),
        };
        #[link(name = "user32")]
        unsafe extern "system" {
            fn MessageBoxW(
                window: *mut std::ffi::c_void,
                text: *const u16,
                title: *const u16,
                flags: u32,
            ) -> i32;
        }
        let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
        let title: Vec<u16> = "Felaris Setup\0".encode_utf16().collect();
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                if result.is_ok() { 0x40 } else { 0x10 },
            )
        };
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    if has_option(&args, "--install") {
        match updater::install_to_system() {
            Ok(msg) => {
                println!("{msg}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Install failed: {e}");
                std::process::exit(1);
            }
        }
    }
    if has_option(&args, "--uninstall") {
        match updater::uninstall_from_system() {
            Ok(msg) => {
                println!("{msg}");
                std::process::exit(0);
            }
            Err(e) => {
                desktop::message("Could not uninstall Felaris", &e, false);
                eprintln!("Uninstall failed: {e}");
                std::process::exit(1);
            }
        }
    }
    if has_option(&args, "--check-update") {
        println!("Current version: {}", updater::CURRENT_VERSION);
        match updater::check_for_updates() {
            Ok(Some(update)) => {
                println!(
                    "Update available: v{} ({})",
                    update.version, update.release_url
                );
                if let Some(url) = update.download_url {
                    println!("Download URL: {url}");
                }
            }
            Ok(None) => println!("Felaris is up to date."),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    }

    if let Err(error) = app::run(&args) {
        desktop::message(
            "Felaris could not start",
            &format!(
                "{error}\n\nMake sure Microsoft Edge WebView2 Runtime is installed, then try again."
            ),
            false,
        );
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn external_link_arguments_cannot_trigger_installer_options() {
        let arguments = |values: &[&str]| {
            values
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
        };
        assert!(super::has_option(
            &arguments(&["felaris.exe", "--install"]),
            "--install"
        ));
        assert!(!super::has_option(
            &arguments(&["felaris.exe", "--", "--uninstall"]),
            "--uninstall"
        ));
        assert!(!super::has_option(
            &arguments(&["felaris.exe", "--", "https://example.com/?x=--setup"]),
            "--setup"
        ));
    }
}
