use std::{
    fs::File,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use serde_json::Value;
use stringsimile_config::rulesets::StringGroupConfig;
use stringsimile_matcher::ruleset::StringGroup;
use tokio::{
    io::{self, AsyncBufReadExt, BufReader},
    sync::broadcast::Receiver,
};
// use tracing::trace;

use crate::{cli::CliArgs, signal::ServiceSignal};

pub struct StringProcessor {
    rules_path: PathBuf,
    rules: Arc<Mutex<Vec<StringGroup>>>,
}

impl StringProcessor {
    pub fn from_args(args: CliArgs) -> Self {
        Self {
            // TODO: validate elsewhere
            rules_path: args.rules_path.expect("missing rule path"),
            rules: Arc::default(),
        }
    }

    // TODO: turn into result and log errors
    pub async fn reload_rules(&mut self) {
        let file = File::open(self.rules_path.clone()).expect("reading failed");
        *self.rules.lock().expect("mutex poisoned") = serde_json::Deserializer::from_reader(file)
            .into_iter::<StringGroupConfig>()
            .map(|c| c.map(|c| c.into_string_group().expect("Failed converting group")))
            .collect::<Result<Vec<StringGroup>, _>>()
            .expect("Failed parsing rules");
    }

    pub async fn run(mut self, mut signals: Receiver<ServiceSignal>) {
        // Initialize rules
        self.reload_rules().await;

        // This does not properly handle cancellation - requires enter press after completion
        let stdin = io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        // let stdout = io::stdout();

        loop {
            tokio::select! {
                line = lines.next_line() => if let Some(line) = line.expect("reading failed") {
                    println!("length = {}", line.len());
                    let parsed: Value = serde_json::from_str(&line).expect("Parsing input failed");
                    println!("parsed = {}", parsed);
                    let mut matches = Vec::default();
                    let rules = self.rules.lock().expect("mutex poisoned");
                    for rule in rules.iter() {
                        if let Some(rule_match) = rule.generate_matches(&parsed) {
                            matches.push(rule_match);
                        }
                    }
                    println!("matches = {:?}", matches);
                },
                Ok(signal) = signals.recv() => match signal {
                    ServiceSignal::ReloadConfig => {
                        self.reload_rules().await;
                    },
                    ServiceSignal::Shutdown | ServiceSignal::Quit => break,
                }
            }
        }
    }
}
