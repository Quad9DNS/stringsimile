use std::{
    fs::File,
    io::{BufReader, Seek},
    panic,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use futures::{TryFutureExt, stream::FusedStream};
use serde_json::{Map, Value};
use snafu::ResultExt;
use stringsimile_config::rulesets::StringGroupConfig;
use stringsimile_matcher::ruleset::{StringGroup, StringGroupMatchResult};
use tokio::{
    sync::broadcast::{self, Receiver},
    task::JoinSet,
};
use tokio_stream::{StreamExt, StreamMap, wrappers::BroadcastStream};
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

use crate::{
    config::ServiceConfig,
    error::{
        FileReadSnafu, InputConfigSnafu, InputParsingSnafu, RuleParsingSnafu,
        StringsimileServiceError,
    },
    field_access::{FieldAccessor, UnwrappedFields},
    inputs::{InputBuilder, InputStreamBuilder},
    metrics::ExportMetrics,
    outputs::OutputStreamBuilder,
    signal::ServiceSignal,
};

pub struct StringProcessor {
    config: ServiceConfig,
    rules: Arc<Mutex<Vec<StringGroup>>>,
}

impl StringProcessor {
    pub fn from_config(config: ServiceConfig) -> Self {
        Self {
            config,
            rules: Arc::default(),
        }
    }

    async fn parse_file(file_path: PathBuf) -> crate::Result<Vec<StringGroupConfig>> {
        let file = File::open(file_path).context(FileReadSnafu)?;
        let mut reader = BufReader::new(file);
        let parsed_rules: Vec<StringGroupConfig> = match serde_json::from_reader(&mut reader) {
            Ok(rules) => rules,
            Err(err) => {
                reader.rewind().context(FileReadSnafu)?;
                serde_json::Deserializer::from_reader(&mut reader)
                    .into_iter::<StringGroupConfig>()
                    .collect::<Result<Vec<StringGroupConfig>, _>>()
                    .map_err(|jsonl_err| StringsimileServiceError::RuleJsonParsing {
                        source_json: err,
                        source_jsonl: jsonl_err,
                    })?
            }
        };
        Ok(parsed_rules)
    }

    pub async fn reload_rules(&mut self) -> crate::Result<()> {
        let parsed_rules = if self.config.matcher.rules_path.is_dir() {
            let mut parsed_rules: Vec<StringGroupConfig> = Vec::new();
            for entry in WalkDir::new(self.config.matcher.rules_path.clone())
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    match Self::parse_file(entry.path().to_path_buf()).await {
                        Ok(rules) => parsed_rules.extend(rules),
                        Err(err) => {
                            warn!(
                                "Error while reading {}. Skipping. Error: {:?}",
                                entry.path().display(),
                                err
                            )
                        }
                    }
                }
            }
            parsed_rules
        } else {
            Self::parse_file(self.config.matcher.rules_path.clone()).await?
        };

        let built_rules = parsed_rules
            .into_iter()
            .map(|c| c.into_string_group())
            .collect::<Result<Vec<StringGroup>, _>>()
            .context(RuleParsingSnafu)?;
        built_rules.export_metrics();
        *self.rules.lock().expect("mutex poisoned") = built_rules;
        Ok(())
    }

    pub async fn run(mut self, mut signals: Receiver<ServiceSignal>) {
        // Initialize rules
        if let Err(err) = self.reload_rules().await {
            error!(message = "Loading rules has failed. Aborting...", error = %err);
            return;
        }

        let mut input_streams = StreamMap::with_capacity(self.config.inputs.len());

        for input in self.config.inputs.clone() {
            let input_name = input.name();
            let input_stream =
                match input
                    .into_stream()
                    .await
                    .map_err(|err| StringsimileServiceError::InputFail {
                        input_name: input_name.clone(),
                        source: err,
                    }) {
                    Ok(stream) => stream,
                    Err(err) => {
                        error!(message = "Input preparation failed!", error = %err);
                        return;
                    }
                };
            input_streams.insert(input_name, input_stream);
        }

        let input_field = match self
            .config
            .matcher
            .input_field
            .build()
            .context(InputConfigSnafu)
        {
            Ok(accessor) => accessor,
            Err(error) => {
                error!(message = "Configuration error!", error = %error);
                return;
            }
        };
        let rules = Arc::clone(&self.rules);
        let report_all = self.config.matcher.report_all;

        let (tx, _rx) = broadcast::channel(128);

        let mut output_tasks = JoinSet::new();

        for output in self.config.outputs.clone() {
            output_tasks.spawn(
                output
                    .consume_stream(Box::pin(
                        BroadcastStream::new(tx.subscribe()).map_while(|res| res.ok()),
                    ))
                    .map_err(|err| {
                        error!(message = "Output task has failed with an error: {}", err);
                    }),
            );
        }

        // let mut transform_futures = FuturesUnordered::new();
        //
        let mut transform_futures = futures::StreamExt::buffer_unordered(
            input_streams.map(|(input_name, (original_input, message))| {
                tokio::spawn(Self::process_input_data(
                    rules.lock().expect("mutex poisoned").clone(),
                    report_all,
                    input_field.clone(),
                    input_name,
                    original_input,
                    message,
                ))
            }),
            32,
        );

        loop {
            tokio::select! {
                Some(task) = output_tasks.join_next() => {
                    match task {
                        Ok(_t) => {
                            info!("Output task completed successfully.");
                        }
                        Err(err) if err.is_panic() => panic::resume_unwind(err.into_panic()),
                        Err(err) => {
                            error!(message = "Output task failed!", error = %err);
                        }
                    }
                },
                Some(result) = transform_futures.next(), if !transform_futures.is_terminated() => {
                    match result {
                        Ok(val) => {
                            if let Err(err) = tx.send(val) {
                                warn!(message = "Passing message to outputs failed.", error = %err);
                            }
                        }
                        Err(err) => {
                            warn!(message = "Rule matcher task failed.", error = %err);
                        }
                    }
                },
                Ok(signal) = signals.recv() => match signal {
                    ServiceSignal::ReloadConfig => {
                        if let Err(err) = self.reload_rules().await {
                            error!(message = "Reloading rules has failed! Keeping previous rules.", error = %err);
                        }
                    },
                    ServiceSignal::Shutdown | ServiceSignal::Quit => {
                        info!("Stopping strinsimile processor...");
                        break;
                    }
                }
            }
        }
    }

    async fn process_input_data(
        rules: Vec<StringGroup>,
        report_all: bool,
        input_field: FieldAccessor,
        input_name: String,
        original_input: String,
        message: Option<Value>,
    ) -> (String, Option<Value>) {
        let Some(message) = message else {
            warn!("Input data was not a JSON object!");
            return (original_input, None);
        };

        let UnwrappedFields {
            input_object_map: mut map,
            input_field_value: name,
        } = match input_field.access_field(message).context(InputParsingSnafu) {
            Ok(fields) => fields,
            Err(error) => {
                warn!(
                    "Input parsing error!\nError: {:?}\nOriginal input: {}",
                    error, original_input
                );
                return (original_input, None);
            }
        };

        debug!("Processing input from {}", input_name);
        let mut matches = Vec::default();
        {
            for rule in rules.iter() {
                let match_results = rule.generate_matches(&name);
                matches.push((rule.name.clone(), match_results));
            }
        }
        if !report_all
            && !matches
                .iter()
                .map(|(_name, results)| results)
                .any(StringGroupMatchResult::has_matches)
        {
            (original_input, None)
        } else {
            let mut inner_data = Map::default();
            inner_data.insert(
                "groups".to_string(),
                Value::Array(
                    matches
                        .into_iter()
                        .filter(|(_name, results)| report_all || results.has_matches())
                        .filter_map(|(name, mut result)| {
                            if !report_all {
                                for res in result.values_mut() {
                                    res.retain(|m| m.matched);
                                }
                                if !result.has_matches() {
                                    return None;
                                }
                            }

                            let mut json = result.to_json();
                            if let Some(obj) = json.as_object_mut() {
                                obj.insert("string_group_name".to_string(), Value::String(name));
                            }
                            Some(json)
                        })
                        .collect(),
                ),
            );
            map.insert("stringsimile".to_string(), Value::Object(inner_data));
            (original_input, Some(Value::Object(map)))
        }
    }
}
