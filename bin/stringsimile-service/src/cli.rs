use std::{collections::HashSet, fs::File, path::PathBuf};

use clap::{ArgAction, Parser};
use tracing::Level;

use crate::{
    config::{FileBasedConfig, LevelInt, MatcherConfig, ServiceConfig},
    inputs::Input,
    outputs::Output,
};

#[derive(Parser)]
#[command(version, about, long_about = None, rename_all = "kebab-case")]
pub struct CliArgs {
    /// Path to the directory (or file) containing rules
    #[clap(short, long, default_value = "/var/lib/stringsimile")]
    rules_path: PathBuf,

    /// Increase log verbosity. May be repeated for further increase.
    #[clap(short, long, action = ArgAction::Count)]
    verbose: u8,

    /// Decrease log verbosity. May be repeated for further decrease.
    #[clap(short, long, action = ArgAction::Count)]
    quiet: u8,

    /// Path to configuration file.
    #[clap(short, long, default_value = "/etc/stringsimile/stringsimile.yaml")]
    config: PathBuf,

    /// Optionally set file to read input data from.
    #[clap(long)]
    input_file: Option<PathBuf>,

    /// Set to true to read input data from stdin.
    #[clap(long, default_value_t = false)]
    input_from_stdin: bool,

    /// Optionally set file to write output data to.
    #[clap(long)]
    output_file: Option<PathBuf>,

    /// Set to true to write output data to stdout.
    #[clap(long, default_value_t = false)]
    output_to_stdout: bool,

    /// Field to take from input JSON object, to match against rules.
    #[clap(long, default_value = ".domain_name")]
    input_field: String,
}

impl CliArgs {
    pub fn build(&self) -> crate::Result<ServiceConfig> {
        let file_config: FileBasedConfig =
            serde_yaml::from_reader(File::open(self.config.clone())?)?;
        let mut base_config = file_config.build()?;

        let log_level_increase = self.verbose - self.quiet;
        let current_log_level = base_config.log_level.into_u8();
        let new_log_level = Level::from_u8(current_log_level.saturating_add(log_level_increase));

        let matcher_config = MatcherConfig {
            rules_path: self.rules_path.clone(),
            input_field: self.input_field.clone(),
        };

        let mut new_inputs = HashSet::default();

        if self.input_from_stdin {
            new_inputs.insert(Input::Stdin);
        }

        if let Some(input_file) = &self.input_file {
            new_inputs.insert(Input::File(input_file.clone()));
            // TODO: For now allow just one file config, maybe it would be okay to have multiple?
            base_config.inputs = base_config
                .inputs
                .into_iter()
                .filter(|i| !matches!(i, Input::File(_)))
                .collect();
        }

        let mut new_outputs = HashSet::default();

        if self.output_to_stdout {
            new_outputs.insert(Output::Stdout);
        }

        if let Some(output_file) = &self.output_file {
            new_outputs.insert(Output::File(output_file.clone()));
            // TODO: For now allow just one file config, maybe it would be okay to have multiple?
            base_config.outputs = base_config
                .outputs
                .into_iter()
                .filter(|i| !matches!(i, Output::File(_)))
                .collect();
        }

        let cli_config = ServiceConfig {
            inputs: new_inputs,
            outputs: new_outputs,
            matcher: matcher_config,
            // Any default for now, will be replaced with the calculated level
            log_level: Level::INFO,
        };

        let mut config = base_config.merge(cli_config);
        config.log_level = new_log_level;

        Ok(config)
    }
}
