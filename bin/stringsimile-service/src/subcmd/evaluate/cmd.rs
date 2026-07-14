use crate::{
    cli,
    config::ServiceConfig,
    processor::StringProcessor,
    service::Service,
    subcmd::evaluate::tracing_layer::{EvaluationLayer, RuleId},
};
use std::{
    collections::HashMap,
    io,
    process::ExitCode,
    time::{Duration, Instant},
};

use clap::Parser;
use stringsimile_matcher::{rule::GenericMatcherRule, rules::bitflip::BitflipRule};
use tracing::warn;
use tracing_subscriber::{
    Registry,
    fmt::{self, writer::MakeWriterExt},
    layer::SubscriberExt,
};

#[derive(Parser, Clone)]
#[command(rename_all = "kebab-case")]
pub struct CliArgs {}

pub fn run(args: cli::CliArgs, _validate_args: &CliArgs) -> ExitCode {
    let config = match ServiceConfig::try_from(args) {
        Ok(mut config) => {
            // Forcing 1 thread for correct calculation
            config.process.threads = 1;
            config
        }
        Err(err) => {
            warn!(message = "Invalid configuration, can't evaluate rule costs...", error = %err);
            return (exitcode::CONFIG as u8).into();
        }
    };
    let matcher = config.matcher.clone();

    let fmt_layer = fmt::layer()
        .with_file(false)
        .with_target(false)
        .with_writer(io::stderr.with_max_level(config.process.log_level));

    let eval_layer = EvaluationLayer::new();
    let subscriber = Registry::default()
        .with(fmt_layer)
        .with(eval_layer.share_layer());
    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");

    let (runtime, app) = match Service::prepare_from_config(config, true) {
        Ok(res) => res,
        Err(code) => {
            std::process::exit(code);
        }
    };

    let app = match app.start(runtime.handle()) {
        Ok(app) => app,
        Err(code) => {
            std::process::exit(code);
        }
    };

    let res = runtime.block_on(app.run());

    let rules = runtime.block_on(async {
        match StringProcessor::load_rules(&matcher).await {
            Ok(rules) => rules,
            Err(err) => {
                warn!(message = "Invalid rules, can't evaluate rule costs...", error = %err);
                Default::default()
            }
        }
    });

    let durations: HashMap<RuleId, Vec<(Instant, Option<Instant>)>> = eval_layer
        .spans_durations
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect();

    let bitflip_rule = BitflipRule::new_dns("test_string", true);

    let iters = 1000000;
    let start = Instant::now();
    for _ in 0..iters {
        let _ = bitflip_rule.match_rule_generic("test_string_2", "test_string", false);
    }
    let base_cost = start.elapsed() / iters;

    println!("Base cost duration: {:?}", base_cost);

    let full_start = Instant::now();
    let mut total_costs = Duration::default();
    for group in &rules {
        let mut group_costs = Duration::default();
        for rule_set in &group.rule_sets {
            let mut ruleset_costs = Duration::default();
            for (index, (_, rule)) in rule_set.rules.iter().enumerate() {
                let rule_id = RuleId {
                    group_name: group.name.clone(),
                    rule_set_name: rule_set.name.clone(),
                    rule_index: index,
                    rule_name: rule.name().to_string(),
                };
                if let Some(blocks) = durations.get(&rule_id) {
                    let durations = blocks
                        .iter()
                        .filter_map(|(l, r)| r.map(|r| r - *l))
                        .collect::<Vec<_>>();
                    let avg_duration = durations.iter().sum::<Duration>() / durations.len() as u32;
                    println!(
                        "------ {} rule: average duration: {:?} (cost: {})",
                        rule.name(),
                        avg_duration,
                        avg_duration.as_nanos() / base_cost.as_nanos()
                    );
                    ruleset_costs += avg_duration;
                } else {
                    println!("------ {} rule average duration: unknown", rule.name());
                }
            }
            println!(
                "---- Total duration for ruleset {}: {:?} (cost: {})",
                rule_set.name,
                ruleset_costs,
                ruleset_costs.as_nanos() / base_cost.as_nanos()
            );
            group_costs += ruleset_costs;
        }
        println!(
            "-- Total duration for group {}: {:?} (cost: {})",
            group.name,
            group_costs,
            group_costs.as_nanos() / base_cost.as_nanos()
        );
        total_costs += group_costs;
    }
    println!(
        "Total duration: {:?} (cost: {}) - process duration: {:?}",
        total_costs,
        total_costs.as_nanos() / base_cost.as_nanos(),
        full_start.elapsed()
    );

    if let Some(exit) = res {
        return (exit.code().unwrap_or(exitcode::UNAVAILABLE) as u8).into();
    }

    (exitcode::OK as u8).into()
}
