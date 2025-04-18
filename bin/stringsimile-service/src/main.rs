mod cli;
mod service;
mod signal;

use std::process::ExitCode;

use service::Service;

fn main() -> ExitCode {
    (Service::init_and_run()
        .code()
        .unwrap_or(exitcode::UNAVAILABLE) as u8)
        .into()
}
