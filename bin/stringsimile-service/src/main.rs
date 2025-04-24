use std::process::ExitCode;

use stringsimile_service::service::Service;

fn main() -> ExitCode {
    (Service::init_and_run()
        .code()
        .unwrap_or(exitcode::UNAVAILABLE) as u8)
        .into()
}
