use crate::{
    cli,
    config::ServiceConfig,
    error::{InputConfigSnafu, InputParsingSnafu, StringsimileServiceError},
    inputs::{InputBuilder, InputStreamBuilder},
    processor::StringProcessor,
};
use std::{
    hint::black_box,
    process::ExitCode,
    time::{Duration, Instant},
};

use clap::{Parser, ValueEnum};
use futures::StreamExt;
use snafu::ResultExt;
use stringsimile_matcher::{
    rule::GenericMatcherRule,
    rules::bitflip::BitflipRule,
    ruleset::{StringGroup, StringGroupContext},
};
use tokio::signal::unix::{SignalKind, signal};
use tokio_stream::StreamMap;
use tracing::{debug, error, info, warn};

#[derive(Parser, Clone)]
#[command(rename_all = "kebab-case")]
pub struct CliArgs {
    /// Number of iterations to evaluate for. Higher count should give more precise results.
    #[clap(long, default_value_t = 100)]
    iterations: u32,

    /// Number of items to take from input as a sample data set. The total number of iterations run
    /// for each rule will be iterations * sample_size.
    #[clap(long, default_value_t = 100)]
    sample_size: usize,

    /// Granularity at which to run the evaluation.
    #[clap(long, default_value_t = Granularity::Total)]
    #[arg(value_enum)]
    granularity: Granularity,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Granularity {
    /// Get the cost of the whole rules file.
    Total,
    /// Get the cost of each string group.
    StringGroup,
    /// Get the cost of each ruleset.
    Ruleset,
    /// Get the cost of each rule.
    Rule,
}

impl Granularity {
    async fn execute_evaluation(
        &self,
        config: &ServiceConfig,
        iters: u32,
        input_data: &[String],
        rules_contexts: &[(&StringGroup, StringGroupContext)],
        base_cost: Duration,
    ) {
        fn format_duration(duration: Duration, max_len: usize) -> String {
            let duration_formatted = format!("{:?}", duration);

            let index_of_suffix = duration_formatted.find(|c| ['m', 'n', 's', 'μ'].contains(&c));
            let (duration, suffix) =
                duration_formatted.split_at(index_of_suffix.unwrap_or(duration_formatted.len()));

            format!(
                "{}{}",
                &duration[..(max_len - suffix.len()).min(duration.len())],
                suffix
            )
        }
        match self {
            Granularity::Total => {
                let start = Instant::now();
                for _ in 0..iters {
                    for input in input_data {
                        for (sg, context) in rules_contexts {
                            let _ = sg.generate_matches(input, context, config.matcher.report_all);
                        }
                    }
                }
                let total_duration = start.elapsed() / iters;

                println!("Total duration: {:?}", total_duration);
                println!(
                    "Total cost: {:?}",
                    total_duration.as_nanos() / base_cost.as_nanos()
                );
            }
            Granularity::StringGroup => {
                fn format_string_group(sg: &str, duration: Duration, cost: u128) {
                    println!(
                        "|{:20}|{:10}|{:8}|",
                        &sg[..sg.len().min(20)],
                        format_duration(duration, 10),
                        cost.max(1)
                    );
                }
                println!("|{:20}|{:10}|{:8}|", "String group", "Duration", "Cost");
                println!("+{:-<20}+{:-<10}+{:-<8}+", "", "", "");
                let mut total_duration = Duration::ZERO;
                for (sg, context) in rules_contexts {
                    let start = Instant::now();
                    for _ in 0..iters {
                        for input in input_data {
                            let _ = sg.generate_matches(input, context, config.matcher.report_all);
                        }
                    }
                    let sg_duration = start.elapsed() / iters;
                    format_string_group(
                        &sg.name,
                        sg_duration,
                        sg_duration.as_nanos() / base_cost.as_nanos(),
                    );
                    total_duration += sg_duration;
                }

                format_string_group(
                    "-",
                    total_duration,
                    total_duration.as_nanos() / base_cost.as_nanos(),
                );
            }
            Granularity::Ruleset => {
                fn format_ruleset(sg: &str, rs: &str, duration: Duration, cost: u128) {
                    println!(
                        "|{:20}|{:20}|{:10}|{:8}|",
                        &sg[..sg.len().min(20)],
                        &rs[..rs.len().min(20)],
                        format_duration(duration, 10),
                        cost.max(1)
                    );
                }
                println!(
                    "|{:20}|{:20}|{:10}|{:8}|",
                    "String group", "Ruleset", "Duration", "Cost"
                );
                println!("+{:-<20}+{:-<20}+{:-<10}+{:-<8}+", "", "", "", "");
                let mut total_duration = Duration::ZERO;
                for (sg, context) in rules_contexts {
                    let mut sg_duration = Duration::ZERO;
                    for rs in &sg.rule_sets {
                        let start = Instant::now();
                        for _ in 0..iters {
                            for input in input_data {
                                let Some(rs_context) = context.ruleset_context(&rs.name) else {
                                    continue;
                                };
                                let _ = rs.generate_matches(
                                    input,
                                    rs_context,
                                    config.matcher.report_all,
                                );
                            }
                        }
                        let rs_duration = start.elapsed() / iters;
                        format_ruleset(
                            &sg.name,
                            &rs.name,
                            rs_duration,
                            rs_duration.as_nanos() / base_cost.as_nanos(),
                        );
                        sg_duration += rs_duration;
                    }
                    format_ruleset(
                        &sg.name,
                        "-",
                        sg_duration,
                        sg_duration.as_nanos() / base_cost.as_nanos(),
                    );
                    total_duration += sg_duration;
                }

                format_ruleset(
                    "-",
                    "-",
                    total_duration,
                    total_duration.as_nanos() / base_cost.as_nanos(),
                );
            }
            Granularity::Rule => {
                fn format_rule(sg: &str, rs: &str, rule: &str, duration: Duration, cost: u128) {
                    println!(
                        "|{:20}|{:20}|{:20}|{:10}|{:8}|",
                        &sg[..sg.len().min(20)],
                        &rs[..rs.len().min(20)],
                        &rule[..rule.len().min(20)],
                        format_duration(duration, 10),
                        cost.max(1)
                    );
                }
                println!(
                    "|{:20}|{:20}|{:20}|{:10}|{:8}|",
                    "String group", "Ruleset", "Rule", "Duration", "Cost"
                );
                println!(
                    "+{:-<20}+{:-<20}+{:-<20}+{:-<10}+{:-<8}+",
                    "", "", "", "", ""
                );
                let mut total_duration = Duration::ZERO;
                for (sg, _) in rules_contexts {
                    let mut sg_duration = Duration::ZERO;
                    for rs in &sg.rule_sets {
                        let mut rs_duration = Duration::ZERO;
                        for (index, (_, rule)) in rs.rules.iter().enumerate() {
                            let start = Instant::now();
                            for _ in 0..iters {
                                for input in input_data {
                                    let _ = rule.match_rule_generic(
                                        input,
                                        &rs.string_match,
                                        config.matcher.report_all,
                                    );
                                }
                            }
                            let rule_duration = start.elapsed() / iters;
                            format_rule(
                                &sg.name,
                                &rs.name,
                                &format!("{} ({})", index, rule.name()),
                                rule_duration,
                                rule_duration.as_nanos() / base_cost.as_nanos(),
                            );
                            rs_duration += rule_duration;
                        }
                        format_rule(
                            &sg.name,
                            &rs.name,
                            "-",
                            rs_duration,
                            rs_duration.as_nanos() / base_cost.as_nanos(),
                        );
                        sg_duration += rs_duration;
                    }
                    format_rule(
                        &sg.name,
                        "-",
                        "-",
                        sg_duration,
                        sg_duration.as_nanos() / base_cost.as_nanos(),
                    );
                    total_duration += sg_duration;
                }

                format_rule(
                    "-",
                    "-",
                    "-",
                    total_duration,
                    total_duration.as_nanos() / base_cost.as_nanos(),
                );
            }
        }
    }
}

pub async fn run(args: cli::CliArgs, evaluate_args: &CliArgs) -> ExitCode {
    let config = match ServiceConfig::try_from(args) {
        Ok(config) => config,
        Err(err) => {
            warn!(message = "Invalid configuration, can't evaluate rule costs...", error = %err);
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
    let mut contexts = Vec::new();
    for (sg, mut context) in rules.iter().map(|sg| (sg, StringGroupContext::new(sg))) {
        context.preload_context(&sg.rule_sets).await;
        contexts.push(context);
    }

    let rules_contexts = rules.iter().zip(contexts).collect::<Vec<_>>();

    let (input_shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);
    let mut input_streams = StreamMap::with_capacity(config.inputs.len());

    for input in config.inputs.clone() {
        let input_name = input.name();
        let input_stream = match input
            .into_stream(input_shutdown_tx.subscribe())
            .await
            .map_err(|err| StringsimileServiceError::InputFail {
                input_name: input_name.clone(),
                source: err,
            }) {
            Ok(stream) => stream,
            Err(err) => {
                error!(message = "Input preparation failed!", error = %err);
                return (exitcode::CONFIG as u8).into();
            }
        };
        input_streams.insert(input_name, input_stream);
    }

    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to set up SIGINT handler.");
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to set up SIGTERM handler.");
    let mut sigquit = signal(SignalKind::quit()).expect("Failed to set up SIGQUIT handler.");

    let input_field = match config.matcher.input_field.build().context(InputConfigSnafu) {
        Ok(accessor) => accessor,
        Err(error) => {
            error!(message = "Configuration error!", error = %error);
            return (exitcode::CONFIG as u8).into();
        }
    };

    let mut limited_input_streams = input_streams.take(evaluate_args.sample_size);
    let mut input_data = Vec::new();
    let mut input_shutdown_tx = Some(input_shutdown_tx);

    loop {
        tokio::select! {
            _ = sigint.recv() => {
                info!(message = "Signal received.", signal = "SIGINT");
                if let Some(input_shutdown_tx) = input_shutdown_tx.take() {
                    info!("Starting graceful shutdown. ({} ms)", config.process.shutdown_timeout.as_millis());
                    let _ = input_shutdown_tx.send(());
                }  else {
                    info!("Forceful shutdown.");
                    break;
                }
            }

            _ = sigterm.recv() => {
                info!(message = "Signal received.", signal = "SIGTERM");
                if let Some(input_shutdown_tx) = input_shutdown_tx.take() {
                    info!("Starting graceful shutdown. ({} ms)", config.process.shutdown_timeout.as_millis());
                    let _ = input_shutdown_tx.send(());
                }  else {
                    info!("Forceful shutdown.");
                    break;
                }
            }

            _ = sigquit.recv() => {
                info!(message = "Signal received.", signal = "SIGQUIT");
                break;
            }

            next = limited_input_streams.next() => {
                let Some((_, message)) = next else {
                    info!("Inputs done!");
                    break;
                };
                let (original_input, message) = message.into_parts();
                let Some(message) = message else {
                    debug!("Input data was not a JSON object!");
                    continue;
                };

                let name = match input_field
                    .access_field(&message)
                    .context(InputParsingSnafu)
                {
                    Ok(fields) => fields,
                    Err(error) => {
                        debug!(
                            "Input parsing error!\nError: {:?}\nOriginal input: {}",
                            error, original_input
                        );
                        continue;
                    }
                };
                input_data.push(name.to_string());
            }
        }
    }

    let bitflip_rule = BitflipRule::new_dns("test_string", true);

    let iters = evaluate_args.iterations;
    let start = Instant::now();
    for _ in 0..iters {
        for input in &input_data {
            let _ = black_box(bitflip_rule.match_rule_generic(input, "test_string", false));
        }
    }
    let base_cost = start.elapsed() / iters;

    println!("Base cost duration: {:?}", base_cost);

    evaluate_args
        .granularity
        .execute_evaluation(&config, iters, &input_data, &rules_contexts, base_cost)
        .await;

    (exitcode::OK as u8).into()
}
