#![forbid(unsafe_code)]

mod cli;
mod output;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::execute(std::env::args_os().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("minilua: {error}");
            ExitCode::FAILURE
        }
    }
}
