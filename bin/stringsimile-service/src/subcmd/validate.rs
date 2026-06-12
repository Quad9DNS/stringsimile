use crate::{cli, config::ServiceConfig, processor::StringProcessor};
use std::process::ExitCode;

use clap::Parser;
use tracing::{info, warn};

#[derive(Parser, Clone)]
#[command(rename_all = "kebab-case")]
pub struct CliArgs {}

pub async fn run(args: cli::CliArgs, _validate_args: &CliArgs) -> ExitCode {
    let conf_path = args.config.to_string_lossy().to_string();
    let config = match ServiceConfig::try_from(args) {
        Ok(config) => config,
        Err(err) => {
            warn!(message = "Invalid configuration", error = %err);
            return (exitcode::CONFIG as u8).into();
        }
    };
    info!("Configuration ({}) is valid.", conf_path);

    if let Err(err) = StringProcessor::load_rules(&config.matcher).await {
        warn!(message = "Invalid rules", error = %err);
        return (exitcode::CONFIG as u8).into();
    }
    info!(
        "Rules ({}) are valid.",
        config.matcher.rules_path.to_string_lossy()
    );

    (exitcode::OK as u8).into()
}
