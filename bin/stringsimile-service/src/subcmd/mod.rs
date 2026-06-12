use std::process::ExitCode;

use clap::Subcommand;

use crate::cli::CliArgs;

mod validate;

#[derive(Subcommand, Clone)]
pub enum SubCmd {
    Validate(validate::CliArgs),
}

impl SubCmd {
    pub async fn run(&self, args: CliArgs) -> ExitCode {
        match self {
            SubCmd::Validate(cli_args) => validate::run(args, cli_args).await,
        }
    }
}
