use std::io;
use std::process::ExitCode;

use clap::Parser;
use stringsimile_service::cli::CliArgs;
use stringsimile_service::config::LevelInt;
use stringsimile_service::service::Service;
use tracing::Level;

fn main() -> ExitCode {
    let args = match CliArgs::try_parse().map_err(|error| {
        _ = error.print();
        error.exit_code()
    }) {
        Ok(args) => args,
        Err(code) => return (code as u8).into(),
    };
    if let Some(subcmd) = args.subcommand.clone() {
        if subcmd.is_async() {
            let log_level_increase = args.verbose - args.quiet;
            let current_log_level = Level::INFO.into_u8();
            let new_log_level =
                Level::from_u8(current_log_level.saturating_add(log_level_increase));
            tracing_subscriber::fmt()
                .with_file(false)
                .with_target(false)
                .with_max_level(new_log_level)
                .with_writer(io::stderr)
                .init();
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(
                    args.threads.unwrap_or(
                        std::thread::available_parallelism()
                            .map(|r| r.get())
                            .unwrap_or(1),
                    ),
                )
                .enable_all()
                .build()
                .expect("Building async runtime failed!");
            return runtime.block_on(subcmd.run(args));
        } else {
            return subcmd.run_sync(args);
        }
    }
    (Service::init_and_run(args)
        .code()
        .unwrap_or(exitcode::UNAVAILABLE) as u8)
        .into()
}
