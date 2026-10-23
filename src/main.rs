#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod desktop;
mod engine;
mod extensions;
mod history;
mod import;
mod instance;
mod permission;
mod profile;
mod recovery;
mod session;
mod settings;
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
    // Servo starts page content in child copies of this executable.
    #[cfg(feature = "servo")]
    {
        let mut args = std::env::args().skip(1);
        if args.next().as_deref() == Some("--content-process")
            && let Some(token) = args.next()
        {
            servo::run_content_process(token);
            return Ok(());
        }
    }
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
    fn external_link_arguments_cannot_trigger_command_options() {
        let arguments = |values: &[&str]| {
            values
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
        };
        assert!(super::has_option(
            &arguments(&["felaris.exe", "--check-update"]),
            "--check-update"
        ));
        assert!(!super::has_option(
            &arguments(&["felaris.exe", "--", "--check-update"]),
            "--check-update"
        ));
        assert!(!super::has_option(
            &arguments(&["felaris.exe", "--", "https://example.com/?x=--check-update"]),
            "--check-update"
        ));
    }
}
