use std::{
    fs::File,
    sync::{Arc, Mutex},
};

use serde_json::{Map, Value};
use stringsimile_config::rulesets::StringGroupConfig;
use stringsimile_matcher::ruleset::StringGroup;
use tokio::{
    io::{self, AsyncWriteExt},
    sync::broadcast::Receiver,
};
use tokio_stream::{StreamExt, StreamMap};
use tracing::{debug, info, warn};

use crate::{config::ServiceConfig, signal::ServiceSignal};

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

    // TODO: turn into result and log errors
    pub async fn reload_rules(&mut self) {
        let file = File::open(self.config.matcher.rules_path.clone()).expect("reading failed");
        *self.rules.lock().expect("mutex poisoned") = serde_json::Deserializer::from_reader(file)
            .into_iter::<StringGroupConfig>()
            .map(|c| c.map(|c| c.into_string_group().expect("Failed converting group")))
            .collect::<Result<Vec<StringGroup>, _>>()
            .expect("Failed parsing rules");
    }

    pub async fn run(mut self, mut signals: Receiver<ServiceSignal>) {
        // Initialize rules
        self.reload_rules().await;

        let mut input_streams = StreamMap::with_capacity(self.config.inputs.len());

        for input in self.config.inputs.clone() {
            input_streams.insert(input.name(), input.into_stream());
        }

        let mut stdout = io::stdout();

        // TODO: abstract away inputs, pre-processors, transformers and outpus
        // Also, don't let any errors stop the processing!
        loop {
            tokio::select! {
                Some((input_name, (original_input, message))) = input_streams.next() => {
                    debug!(message = "Processing input from {}", input_name);
                    let mut matches = Vec::default();
                    {
                        let rules = self.rules.lock().expect("mutex poisoned");
                        for rule in rules.iter() {
                            if let Some(rule_match) = rule.generate_matches(&message) {
                                matches.push(rule_match);
                            }
                        }
                    }
                    if matches.is_empty() {
                        stdout.write_all(original_input.as_bytes()).await.expect("Write failed");
                    } else if let Value::Object(mut map) = message {
                        let mut inner_data = Map::default();
                        inner_data.insert("matches".to_string(), Value::Array(matches.into_iter().map(Value::Object).collect()));
                        map.insert("stringsimile".to_string(), Value::Object(inner_data));
                        stdout.write_all(&serde_json::to_vec(&Value::Object(map)).expect("Serialization failed")).await.expect("Write failed");
                    } else {
                        warn!("Input data was not a JSON object!");
                        stdout.write_all(original_input.as_bytes()).await.expect("Write failed");
                    }
                    stdout.write_all(b"\n").await.expect("Write failed");
                },
                Ok(signal) = signals.recv() => match signal {
                    ServiceSignal::ReloadConfig => {
                        self.reload_rules().await;
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
