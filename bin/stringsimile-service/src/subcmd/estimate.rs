use crate::{cli, config::ServiceConfig, processor::StringProcessor};
use std::process::ExitCode;

use clap::Parser;
use stringsimile_matcher::rule::EstimationResult;
use tracing::warn;

#[derive(Parser, Clone)]
#[command(rename_all = "kebab-case")]
pub struct CliArgs {}

pub async fn run(args: cli::CliArgs, _validate_args: &CliArgs) -> ExitCode {
    let config = match ServiceConfig::try_from(args) {
        Ok(config) => config,
        Err(err) => {
            warn!(message = "Invalid configuration, can't estimate rule costs...", error = %err);
            return (exitcode::CONFIG as u8).into();
        }
    };

    let rules = match StringProcessor::load_rules(&config.matcher).await {
        Ok(rules) => rules,
        Err(err) => {
            warn!(message = "Invalid rules, can't estimate rule costs...", error = %err);
            return (exitcode::CONFIG as u8).into();
        }
    };

    let mut total: EstimationResult = EstimationResult::zero();
    for group in &rules {
        for rule_set in &group.rule_sets {
            for (_, rule) in &rule_set.rules {
                println!(
                    "------ {} rule cost: {:?}",
                    rule.name(),
                    rule.estimate_generic(&rule_set.string_match)
                );
            }
            let cost = group.estimate_cost();
            println!("---- Total for ruleset {}: {:?}", rule_set.name, cost);
        }
        let cost = group.estimate_cost();
        println!("-- Total for group {}: {:?}", group.name, cost);
        total += cost;
    }
    println!("Total costs: {:?}", total);

    (exitcode::OK as u8).into()
}
