use std::process::ExitCode;

use clap::Subcommand;

use crate::cli::CliArgs;

mod estimate;
mod evaluate;
mod validate;

#[derive(Subcommand, Clone)]
pub enum SubCmd {
    Validate(validate::CliArgs),
    Estimate(estimate::CliArgs),
    Evaluate(evaluate::CliArgs),
}

impl SubCmd {
    pub async fn run(&self, args: CliArgs) -> ExitCode {
        match self {
            SubCmd::Validate(cli_args) => validate::run(args, cli_args).await,
            SubCmd::Estimate(cli_args) => estimate::run(args, cli_args).await,
            _ => (exitcode::USAGE as u8).into(),
        }
    }

    pub fn run_sync(&self, args: CliArgs) -> ExitCode {
        match self {
            SubCmd::Evaluate(cli_args) => evaluate::run(args, cli_args),
            _ => (exitcode::USAGE as u8).into(),
        }
    }

    pub fn is_async(&self) -> bool {
        match self {
            SubCmd::Validate(_) | SubCmd::Estimate(_) => true,
            SubCmd::Evaluate(_) => false,
        }
    }
}
