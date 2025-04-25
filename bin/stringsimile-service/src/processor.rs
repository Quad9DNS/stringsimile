use std::{
    fs::File,
    panic,
    sync::{Arc, Mutex},
};

use futures::TryFutureExt;
use serde_json::{Map, Value};
use snafu::ResultExt;
use stringsimile_config::rulesets::StringGroupConfig;
use stringsimile_matcher::ruleset::StringGroup;
use tokio::{
    sync::broadcast::{self, Receiver},
    task::JoinSet,
};
use tokio_stream::{StreamExt, StreamMap, wrappers::BroadcastStream};
use tracing::{debug, error, info, warn};

use crate::{
    config::ServiceConfig,
    error::{FileNotFoundSnafu, RuleJsonParsingSnafu, RuleParsingSnafu, StringsimileServiceError},
    inputs::{InputBuilder, InputStreamBuilder},
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

    pub async fn reload_rules(&mut self) -> crate::Result<()> {
        let file = File::open(self.config.matcher.rules_path.clone()).context(FileNotFoundSnafu)?;
        *self.rules.lock().expect("mutex poisoned") = serde_json::Deserializer::from_reader(file)
            .into_iter::<StringGroupConfig>()
            .map(|c| c.map(|c| c.into_string_group()))
            .collect::<Result<Result<Vec<StringGroup>, _>, _>>()
            .context(RuleJsonParsingSnafu)?
            .context(RuleParsingSnafu)?;
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

        let input_field = self.config.matcher.input_field.clone();
        let rules = Arc::clone(&self.rules);

        let mut transformed_stream =
            input_streams.map(|(input_name, (original_input, message))| {
                let Some(message) = message else {
                    warn!("Input data was not a JSON object!");
                    return (original_input, None);
                };

                let Value::Object(ref map) = message else {
                    warn!("Expected JSON object, but found: {message}");
                    return (original_input, None);
                };

                let Some(field) = map.get(&input_field) else {
                    warn!("Specified key field ({}) not found in input.", input_field);
                    return (original_input, None);
                };

                let Value::String(name) = field else {
                    warn!("Expected string value in key field, but found: {:?}", field);
                    return (original_input, None);
                };

                debug!(message = "Processing input from {}", input_name);
                let mut matches = Vec::default();
                {
                    let rules = rules.lock().expect("mutex poisoned");
                    for rule in rules.iter() {
                        if let Some(rule_match) = rule.generate_matches(name) {
                            matches.push(rule_match);
                        }
                    }
                }
                if matches.is_empty() {
                    (original_input, None)
                } else if let Value::Object(mut map) = message {
                    let mut inner_data = Map::default();
                    inner_data.insert(
                        "matches".to_string(),
                        Value::Array(matches.into_iter().map(Value::Object).collect()),
                    );
                    map.insert("stringsimile".to_string(), Value::Object(inner_data));
                    (original_input, Some(Value::Object(map)))
                } else {
                    warn!("Input data was not a JSON object!");
                    (original_input, None)
                }
            });

        let (tx, _rx) = broadcast::channel(128);

        let mut output_tasks = JoinSet::new();

        for output in self.config.outputs.clone() {
            output_tasks.spawn(
                output
                    .consume_stream(Box::pin(
                        BroadcastStream::new(tx.subscribe()).map_while(|res| res.ok()),
                    ))
                    // TODO: do something with the error here
                    .map_err(|_err| ()),
            );
        }

        // TODO: abstract away inputs, pre-processors, transformers and outpus
        // Also, don't let any errors stop the processing!
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
                Some(val) = transformed_stream.next() => {
                    if let Err(err) = tx.send(val) {
                        warn!(message = "Passing message to outputs failed.", error = %err);
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
}
