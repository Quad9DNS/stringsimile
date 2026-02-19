use std::{collections::HashSet, fs::File, path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use stringsimile_matcher::Error;
use tracing::Level;

use crate::{
    cli::CliArgs,
    error::{ConfigYamlParsingSnafu, FileReadSnafu},
    field_access::FieldAccessorConfig,
    inputs::Input,
    metrics_exporters::{FileExporterConfig, MetricsExporter, StdoutExporterConfig},
    outputs::Output,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBasedConfig {
    #[serde(default)]
    input: InputConfig,
    #[serde(default)]
    output: OutputConfig,
    #[serde(default)]
    metrics: MetricsConfig,
    #[serde(default)]
    matcher: MatcherConfig,
    #[serde(default)]
    process: ProcessConfig,
}

impl FileBasedConfig {
    pub fn build(&self) -> crate::Result<ServiceConfig> {
        Ok(ServiceConfig {
            inputs: self.input.build()?,
            outputs: self.output.build()?,
            metrics: self.metrics.build()?,
            matcher: self.matcher.clone(),
            process: self.process.build()?,
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct InputConfig {
    #[serde(default)]
    file_path: Option<PathBuf>,
    #[serde(default)]
    pipe_path: Option<PathBuf>,
    #[serde(default)]
    stdin: bool,
    #[cfg(feature = "inputs-kafka")]
    #[serde(default)]
    kafka: Option<crate::inputs::KafkaInputConfig>,
}

impl InputConfig {
    pub fn build(&self) -> crate::Result<HashSet<Input>> {
        let mut result = HashSet::default();
        if let Some(file_path) = &self.file_path {
            result.insert(Input::File(file_path.clone()));
        }
        if let Some(pipe_path) = &self.pipe_path {
            result.insert(Input::Pipe(pipe_path.clone()));
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
    kafka: Option<crate::outputs::KafkaOutputConfig>,
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
struct MetricsConfig {
    #[serde(default)]
    file: Option<FileExporterConfig>,
    #[serde(default)]
    stdout: Option<StdoutExporterConfig>,
    #[serde(default = "default_metrics_prefix")]
    name_prefix: String,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            file: Some(FileExporterConfig {
                file_path: "/var/lib/node-exporter/stringsimile.prom".into(),
                export_interval_secs: 15,
                mode: 0o644,
            }),
            stdout: Default::default(),
            name_prefix: default_metrics_prefix(),
        }
    }
}

impl MetricsConfig {
    pub fn build(&self) -> crate::Result<ValidatedMetricsConfig> {
        let mut exporters = HashSet::default();
        if let Some(config) = &self.file {
            exporters.insert(MetricsExporter::File(config.clone()));
        }
        if let Some(config) = &self.stdout {
            exporters.insert(MetricsExporter::Stdout(config.clone()));
        }
        Ok(ValidatedMetricsConfig {
            exporters,
            prefix: self.name_prefix.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatcherConfig {
    #[serde(default = "default_rules_path")]
    pub rules_path: PathBuf,
    #[serde(default = "default_field_name")]
    pub input_field: FieldAccessorConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    #[serde(default = "default_thread_count")]
    threads: usize,
    #[serde(default = "default_log_level")]
    log_level: String,
    #[serde(default = "default_shutdown_duration_ms")]
    shutdown_timeout_ms: usize,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            threads: default_thread_count(),
            log_level: default_log_level(),
            shutdown_timeout_ms: default_shutdown_duration_ms(),
        }
    }
}

impl ProcessConfig {
    pub fn build(&self) -> crate::Result<ValidatedProcessConfig> {
        Ok(ValidatedProcessConfig {
            threads: self.threads,
            log_level: self.log_level.parse()?,
            shutdown_timeout: Duration::from_millis(self.shutdown_timeout_ms.try_into()?),
        })
    }
}

fn default_rules_path() -> PathBuf {
    "/var/lib/stringsimile"
        .parse()
        .expect("Invalid default rules path")
}

fn default_field_name() -> FieldAccessorConfig {
    FieldAccessorConfig(".domain_name".to_string())
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_thread_count() -> usize {
    std::thread::available_parallelism()
        .map(|r| r.get())
        .unwrap_or(1)
}

fn default_shutdown_duration_ms() -> usize {
    60 * 1000
}

fn default_shutdown_duration() -> Duration {
    Duration::from_secs(60)
}

fn default_metrics_prefix() -> String {
    "stringsimile".to_string()
}

/// Parsed and validated process configuration for the stringsimile service.
#[derive(Debug, Clone)]
pub struct ValidatedProcessConfig {
    /// Number of threads to use
    pub threads: usize,
    /// Internal logging level
    pub log_level: Level,
    /// Graceful shutdown timeout. When shutdown is requested (SIGINT), the process will wait for
    /// processing to complete for the given duration and will resort to forceful shutdown
    /// afterwards.
    pub shutdown_timeout: Duration,
}

impl ValidatedProcessConfig {
    pub fn merge(self, other: Self) -> Self {
        Self {
            threads: if other.threads == default_thread_count() {
                self.threads
            } else {
                other.threads
            },
            log_level: self.log_level.max(other.log_level),
            shutdown_timeout: if other.shutdown_timeout == default_shutdown_duration() {
                self.shutdown_timeout
            } else {
                other.shutdown_timeout
            },
        }
    }
}

/// Configuration for stringsimile metrics.
#[derive(Debug, Clone)]
pub struct ValidatedMetricsConfig {
    /// List of metrics exporters to export metrics with.
    pub exporters: HashSet<MetricsExporter>,
    /// Prefix to apply to all metrics names.
    pub prefix: String,
}

impl ValidatedMetricsConfig {
    pub fn merge(self, other: Self) -> Self {
        Self {
            exporters: self.exporters.into_iter().chain(other.exporters).collect(),
            prefix: if other.prefix == default_metrics_prefix() {
                self.prefix
            } else {
                other.prefix
            },
        }
    }
}

/// Parsed and validated configuration for the stringsimile service.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// List of inputs to read input data from.
    pub inputs: HashSet<Input>,
    /// List of outputs to write output data to.
    pub outputs: HashSet<Output>,
    /// Configuration for metrics.
    pub metrics: ValidatedMetricsConfig,
    /// Configuration for matcher, defining rules source and field to consider when matching.
    pub matcher: MatcherConfig,
    /// Configuration for the process.
    pub process: ValidatedProcessConfig,
}

impl ServiceConfig {
    pub fn merge(self, other: Self) -> Self {
        Self {
            inputs: self.inputs.into_iter().chain(other.inputs).collect(),
            outputs: self.outputs.into_iter().chain(other.outputs).collect(),
            metrics: self.metrics.merge(other.metrics),
            matcher: self.matcher.merge(other.matcher),
            process: self.process.merge(other.process),
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
        let current_log_level = base_config.process.log_level.into_u8();
        let new_log_level = Level::from_u8(current_log_level.saturating_add(log_level_increase));

        let matcher_config = MatcherConfig {
            rules_path: value.rules_path.clone(),
            input_field: FieldAccessorConfig(value.input_field.clone()),
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

        if let Some(input_pipe) = &value.input_pipe {
            new_inputs.insert(Input::Pipe(input_pipe.clone()));
            // TODO: For now allow just one pipe config, maybe it would be okay to have multiple?
            base_config.inputs.retain(|i| !matches!(i, Input::Pipe(_)));
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

        let process_config = ValidatedProcessConfig {
            threads: value.threads.unwrap_or(default_thread_count()),
            // Any default for now, will be replaced with the calculated level
            log_level: Level::INFO,
            shutdown_timeout: default_shutdown_duration(),
        };

        let mut new_metrics = HashSet::default();

        if value.metrics_to_stdout {
            new_metrics.insert(MetricsExporter::Stdout(StdoutExporterConfig {
                export_interval_secs: 15,
            }));
        }

        if let Some(metrics_file) = &value.metrics_file {
            new_metrics.insert(MetricsExporter::File(FileExporterConfig {
                file_path: metrics_file.clone(),
                export_interval_secs: 15,
                mode: 0o644,
            }));
            // TODO: For now allow just one file config, maybe it would be okay to have multiple?
            base_config
                .metrics
                .exporters
                .retain(|i| !matches!(i, MetricsExporter::File(_)));
        }

        let metrics_config = ValidatedMetricsConfig {
            exporters: new_metrics,
            prefix: value.metrics_name_prefix,
        };

        let cli_config = ServiceConfig {
            inputs: new_inputs,
            outputs: new_outputs,
            metrics: metrics_config,
            matcher: matcher_config,
            process: process_config,
        };

        let mut config = base_config.merge(cli_config);
        config.process.log_level = new_log_level;

        Ok(config)
    }
}
