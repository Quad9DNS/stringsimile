use std::{collections::HashSet, io::Write};

use criterion::{Criterion, criterion_group, criterion_main};
use stringsimile_service::{
    config::{MatcherConfig, ServiceConfig, ValidatedMetricsConfig, ValidatedProcessConfig},
    field_access::FieldAccessorConfig,
    inputs::Input,
    outputs::Output,
    processor::StringProcessor,
};
use tempfile::NamedTempFile;
use tokio::sync::broadcast::{self};
use tracing::Level;

const INPUT_DATA: &[u8] =
    br#"{"name":"bot-upload.s3.amazonaws.com.","timestamp":"2025-05-06T20:19:23.356Z"}
{"name":"webapp.dtes.mh.gob.sv.","timestamp":"2025-05-06T20:19:24.171Z"}
{"name":"binance.org.","timestamp":"2025-05-06T20:19:24.172Z"}
{"name":"akmstatic.ml.youngjoygame.com.","timestamp":"2025-05-06T20:19:24.172Z"}
"#;

const RULES_DATA: &[u8] = br#"
{ "name": "Example string group", "rule_sets": [ { "name": "Test rule set", "string_match": "test", "split_target": true, "ignore_tld": true, "match_rules": [ { "rule_type": "levenshtein", "values": { "maximum_distance": 3 } }, { "rule_type": "hamming", "values": { "maximum_distance": 3 } }, { "rule_type": "soundex", "values": { "minimum_similarity": 3 } }, { "rule_type": "metaphone", "values": { "max_code_length": 3 } }, { "rule_type": "nysiis", "values": { "strict": true } }, { "rule_type": "jaro", "values": { "match_percent_threshold": 0.85 } }, { "rule_type": "jaro_winkler", "values": { "match_percent_threshold": 0.85 } }, { "rule_type": "confusables" }, { "rule_type": "match_rating" }, { "rule_type": "damerau_levenshtein", "values": { "maximum_distance": 3 } } ] }, { "name": "Example rule set", "split_target": true, "ignore_tld": true, "string_match": "example", "match_rules": [ { "rule_type": "levenshtein", "values": { "maximum_distance": 3 } }, { "rule_type": "jaro", "values": { "match_percent_threshold": 0.85 } } ] } ] }
"#;

fn processor(c: &mut Criterion) {
    // Prepare files for inputs and rules
    let mut input_file = NamedTempFile::new().expect("Failed creating input file");
    input_file
        .write_all(INPUT_DATA)
        .expect("Failed writing input data");
    let output_file = NamedTempFile::new().expect("Failed creating output file");
    let mut rules_file = NamedTempFile::new().expect("Failed creating rules file");
    rules_file
        .write_all(RULES_DATA)
        .expect("Failed writing rules data");

    let config = ServiceConfig {
        inputs: HashSet::from_iter(vec![Input::File(input_file.path().to_path_buf())]),
        outputs: HashSet::from_iter(vec![Output::File(output_file.path().to_path_buf())]),
        metrics: ValidatedMetricsConfig {
            exporters: Default::default(),
            prefix: String::default(),
        },
        matcher: MatcherConfig {
            rules_path: rules_file.path().to_path_buf(),
            input_field: FieldAccessorConfig(".name".to_string()),
            report_all: true,
        },
        process: ValidatedProcessConfig {
            threads: 1,
            log_level: Level::INFO,
        },
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Building async runtime failed!");
    let (tx, _) = broadcast::channel(16);

    let mut group = c.benchmark_group("stringsimile_service");
    group.throughput(criterion::Throughput::Bytes(INPUT_DATA.len() as u64));

    group.bench_function("processor", |b| {
        b.iter(|| {
            let processor = StringProcessor::from_config(config.clone());
            runtime.spawn(processor.run(tx.subscribe()));
        });
    });

    group.finish();
}

criterion_group!(benches, processor);
criterion_main!(benches);
