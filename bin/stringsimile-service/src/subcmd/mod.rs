use std::process::ExitCode;

use clap::Subcommand;

use crate::cli::CliArgs;

mod estimate;
mod validate;

#[derive(Subcommand, Clone)]
pub enum SubCmd {
    Validate(validate::CliArgs),
    Estimate(estimate::CliArgs),
}

impl SubCmd {
    pub async fn run(&self, args: CliArgs) -> ExitCode {
        match self {
            SubCmd::Validate(cli_args) => validate::run(args, cli_args).await,
            SubCmd::Estimate(cli_args) => estimate::run(args, cli_args).await,
        }
    }
}
