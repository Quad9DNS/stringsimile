use std::{collections::HashSet, fs::File, path::PathBuf};

use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use stringsimile_matcher::Error;
use tracing::Level;

use crate::{
    cli::CliArgs,
    error::{ConfigYamlParsingSnafu, FileReadSnafu},
    inputs::Input,
    outputs::Output,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBasedConfig {
    #[serde(default)]
    input: InputConfig,
    #[serde(default)]
    output: OutputConfig,
    #[serde(default)]
    matcher: MatcherConfig,
    #[serde(default = "default_log_level")]
    log_level: String,
}

impl FileBasedConfig {
    pub fn build(&self) -> crate::Result<ServiceConfig> {
        Ok(ServiceConfig {
            inputs: self.input.build()?,
            outputs: self.output.build()?,
            matcher: self.matcher.clone(),
            log_level: self.log_level.parse()?,
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct InputConfig {
    #[serde(default)]
    file_path: Option<PathBuf>,
    #[serde(default)]
    stdin: bool,
    #[cfg(feature = "inputs-kafka")]
    #[serde(default)]
    kafka: Option<crate::inputs::kafka::KafkaInputConfig>,
}

impl InputConfig {
    pub fn build(&self) -> crate::Result<HashSet<Input>> {
        let mut result = HashSet::default();
        if let Some(file_path) = &self.file_path {
            result.insert(Input::File(file_path.clone()));
        }
        if self.stdin {
            result.insert(Input::Stdin);
        }
        #[cfg(feature = "inputs-kafka")]
        if let Some(kafka_config) = &self.kafka {
            result.insert(Input::Kafka(kafka_config.clone()));
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct OutputConfig {
    #[serde(default)]
    file_path: Option<PathBuf>,
    #[serde(default)]
    stdout: bool,
    #[cfg(feature = "outputs-kafka")]
    #[serde(default)]
    kafka: Option<crate::outputs::kafka::KafkaOutputConfig>,
}

impl OutputConfig {
    pub fn build(&self) -> crate::Result<HashSet<Output>> {
        let mut result = HashSet::default();
        if let Some(file_path) = &self.file_path {
            result.insert(Output::File(file_path.clone()));
        }
        if self.stdout {
            result.insert(Output::Stdout);
        }
        #[cfg(feature = "outputs-kafka")]
        if let Some(kafka_config) = &self.kafka {
            result.insert(Output::Kafka(kafka_config.clone()));
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatcherConfig {
    #[serde(default = "default_rules_path")]
    pub rules_path: PathBuf,
    #[serde(default = "default_field_name")]
    pub input_field: String,
    #[serde(default)]
    pub report_all: bool,
}

impl Default for MatcherConfig {
    fn default() -> Self {
        Self {
            rules_path: default_rules_path(),
            input_field: default_field_name(),
            report_all: false,
        }
    }
}

impl MatcherConfig {
    pub fn merge(self, other: Self) -> Self {
        Self {
            rules_path: if other.rules_path == default_rules_path() {
                self.rules_path
            } else {
                other.rules_path
            },
            input_field: if other.input_field == default_field_name() {
                self.input_field
            } else {
                other.input_field
            },
            report_all: self.report_all || other.report_all,
        }
    }
}

fn default_rules_path() -> PathBuf {
    "/var/lib/stringsimile"
        .parse()
        .expect("Invalid default rules path")
}

fn default_field_name() -> String {
    ".domain_name".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

/// Parsed and validated configuration for the stringsimile service.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// List of inputs to read input data from.
    pub inputs: HashSet<Input>,
    /// List of outputs to write output data to.
    pub outputs: HashSet<Output>,
    /// Configuration for matcher, defining rules source and field to consider when matching.
    pub matcher: MatcherConfig,
    /// Internal logging level.
    pub log_level: Level,
}

impl ServiceConfig {
    pub fn merge(self, other: Self) -> Self {
        Self {
            inputs: self.inputs.into_iter().chain(other.inputs).collect(),
            outputs: self.outputs.into_iter().chain(other.outputs).collect(),
            matcher: self.matcher.merge(other.matcher),
            log_level: self.log_level.max(other.log_level),
        }
    }
}

pub trait LevelInt {
    #[must_use]
    fn into_u8(self) -> u8;
    #[must_use]
    fn from_u8(level: u8) -> Self;
}

impl LevelInt for Level {
    fn into_u8(self) -> u8 {
        match self {
            Level::ERROR => 1,
            Level::WARN => 2,
            Level::INFO => 3,
            Level::DEBUG => 4,
            Level::TRACE => 5,
        }
    }

    fn from_u8(level: u8) -> Self {
        match level {
            0 | 1 => Level::ERROR,
            2 => Level::WARN,
            3 => Level::INFO,
            4 => Level::DEBUG,
            _ => Level::TRACE,
        }
    }
}

impl TryFrom<CliArgs> for ServiceConfig {
    type Error = Error;

    fn try_from(value: CliArgs) -> crate::Result<ServiceConfig> {
        let file_config: FileBasedConfig =
            serde_yaml::from_reader(File::open(value.config.clone()).context(FileReadSnafu)?)
                .context(ConfigYamlParsingSnafu)?;
        let mut base_config = file_config.build()?;

        let log_level_increase = value.verbose - value.quiet;
        let current_log_level = base_config.log_level.into_u8();
        let new_log_level = Level::from_u8(current_log_level.saturating_add(log_level_increase));

        let matcher_config = MatcherConfig {
            rules_path: value.rules_path.clone(),
            input_field: value.input_field.clone(),
            report_all: value.report_all,
        };

        let mut new_inputs = HashSet::default();

        if value.input_from_stdin {
            new_inputs.insert(Input::Stdin);
        }

        if let Some(input_file) = &value.input_file {
            new_inputs.insert(Input::File(input_file.clone()));
            // TODO: For now allow just one file config, maybe it would be okay to have multiple?
            base_config.inputs.retain(|i| !matches!(i, Input::File(_)));
        }

        let mut new_outputs = HashSet::default();

        if value.output_to_stdout {
            new_outputs.insert(Output::Stdout);
        }

        if let Some(output_file) = &value.output_file {
            new_outputs.insert(Output::File(output_file.clone()));
            // TODO: For now allow just one file config, maybe it would be okay to have multiple?
            base_config
                .outputs
                .retain(|i| !matches!(i, Output::File(_)));
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
