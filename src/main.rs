#![forbid(unsafe_code)]

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match double_comma::run_cli(env::args_os().collect()) {
        Ok(code) => ExitCode::from(code.clamp(0, u8::MAX as i32) as u8),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
