use crate::{cli, config::ServiceConfig, processor::StringProcessor};
use std::process::ExitCode;

use clap::Parser;
use stringsimile_matcher::rule::EstimationResult;
use tracing::warn;

#[derive(Parser, Clone)]
#[command(rename_all = "kebab-case")]
pub struct CliArgs {}

pub async fn run(args: cli::CliArgs, _estimate_args: &CliArgs) -> ExitCode {
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

    fn format_rule(sg: &str, rs: &str, rule: &str, cost: &EstimationResult) {
        let formatted_influence = format!("{:?}", cost.input_string_influence);
        println!(
            "|{:20}|{:20}|{:20}|{:8}|{:8}|{:8}|{:20}|",
            &sg[..sg.len().min(20)],
            &rs[..rs.len().min(20)],
            &rule[..rule.len().min(20)],
            cost.min.map(|c| c.to_string()).unwrap_or("-".to_string()),
            cost.max.map(|c| c.to_string()).unwrap_or("-".to_string()),
            cost.calculated,
            &formatted_influence[..formatted_influence.len().min(20)]
        );
    }
    println!(
        "|{:20}|{:20}|{:20}|{:8}|{:8}|{:8}|{:20}|",
        "String group", "Ruleset", "Rule", "Min Cost", "Max Cost", "Estimate", "Input influence"
    );
    println!(
        "+{:-<20}+{:-<20}+{:-<20}+{:-<8}+{:-<8}+{:-<8}+{:-<20}+",
        "", "", "", "", "", "", ""
    );

    let mut total: EstimationResult = EstimationResult::zero();
    for group in &rules {
        for rule_set in &group.rule_sets {
            for (_, rule) in &rule_set.rules {
                format_rule(
                    &group.name,
                    &rule_set.name,
                    rule.name(),
                    &rule.estimate_generic(&rule_set.string_match),
                );
            }
            format_rule(&group.name, &rule_set.name, "-", &rule_set.estimate_cost());
        }
        let cost = group.estimate_cost();
        format_rule(&group.name, "-", "-", &cost);
        total += cost;
    }
    format_rule("-", "-", "-", &total);

    (exitcode::OK as u8).into()
}
