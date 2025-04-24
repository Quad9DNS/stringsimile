use std::{collections::HashSet, path::PathBuf};

use serde::{Deserialize, Serialize};
use tracing::Level;

use crate::{inputs::Input, outputs::Output};

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
        Ok(result)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct OutputConfig {
    #[serde(default)]
    file_path: Option<PathBuf>,
    #[serde(default)]
    stdout: bool,
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
        Ok(result)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatcherConfig {
    #[serde(default = "default_rules_path")]
    pub rules_path: PathBuf,
    #[serde(default = "default_field_name")]
    pub input_field: String,
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
        }
    }
}

fn default_rules_path() -> PathBuf {
    "/var/lib/stringsimile"
        .parse()
        .expect("Invalid default rules path")
}

fn default_field_name() -> String {
    "domain_name".to_string()
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
            inputs: self
                .inputs
                .into_iter()
                .chain(other.inputs.into_iter())
                .collect(),
            outputs: self
                .outputs
                .into_iter()
                .chain(other.outputs.into_iter())
                .collect(),
            matcher: self.matcher.merge(other.matcher),
            log_level: self.log_level.max(other.log_level),
        }
    }
}

pub trait LevelInt {
    fn into_u8(self) -> u8;
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
