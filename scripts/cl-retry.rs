//! A `cl.exe` that runs the real one (`FELARIS_REAL_CL`) and retries when it fails with
//! C1056. ANGLE's full build intermittently failed on this machine, while the same commands
//! succeeded in isolation. Built and used by `scripts/servo.ps1`.

use std::io::Write;
use std::process::{Command, exit};
use std::{env, thread, time::Duration};

fn main() {
    let real = env::var_os("FELARIS_REAL_CL").expect("FELARIS_REAL_CL is not set");
    let args: Vec<_> = env::args_os().skip(1).collect();
    for attempt in 1.. {
        let output = match Command::new(&real).args(&args).output() {
            Ok(output) => output,
            Err(error) => {
                eprintln!("cl-retry: could not run {}: {error}", real.to_string_lossy());
                exit(1);
            }
        };
        let locked = !output.status.success()
            && [&output.stdout, &output.stderr]
                .iter()
                .any(|text| String::from_utf8_lossy(text).contains("C1056"));
        if locked && attempt < 6 {
            thread::sleep(Duration::from_millis(250 * attempt));
            continue;
        }
        let _ = std::io::stdout().write_all(&output.stdout);
        let _ = std::io::stderr().write_all(&output.stderr);
        exit(output.status.code().unwrap_or(1));
    }
}
