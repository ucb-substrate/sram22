use std::process::ExitCode;

use sram22::cli::{run, ReportedError};

fn main() -> ExitCode {
    let result = run();
    sram22::assets::cleanup();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if !error.is::<ReportedError>() {
                eprintln!("error: {error:#}");
            }
            ExitCode::FAILURE
        }
    }
}
