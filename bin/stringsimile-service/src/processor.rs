use std::{fs::File, path::PathBuf};

use serde_json::Value;
use stringsimile_config::rulesets::StringGroup;
use tokio::{
    io::{self, AsyncBufReadExt, BufReader},
    sync::broadcast::Receiver,
};

use crate::{cli::CliArgs, signal::ServiceSignal};

pub struct StringProcessor {
    rules_path: PathBuf,
    rules: Vec<StringGroup>,
}

impl StringProcessor {
    pub fn from_args(args: CliArgs) -> Self {
        Self {
            // TODO: validate elsewhere
            rules_path: args.rules_path.expect("missing rule path"),
            rules: Vec::default(),
        }
    }

    pub async fn reload_rules(&mut self) {
        let file = File::open(self.rules_path.clone()).expect("reading failed");
        self.rules = serde_json::Deserializer::from_reader(file)
            .into_iter::<StringGroup>()
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

        loop {
            tokio::select! {
                line = lines.next_line() => if let Some(line) = line.expect("reading failed") {
                    println!("length = {}", line.len());
                    let parsed: Value = serde_json::from_str(&line).expect("Parsing input failed");
                    println!("parsed = {}", parsed);
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
