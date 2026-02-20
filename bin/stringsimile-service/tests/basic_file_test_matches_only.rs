use std::{
    collections::HashSet,
    io::{Seek, Write},
    time::Duration,
};

use serde_json::{Map, Value};
use stringsimile_service::{
    config::{MatcherConfig, ServiceConfig, ValidatedMetricsConfig, ValidatedProcessConfig},
    field_access::FieldAccessorConfig,
    inputs::Input,
    outputs::Output,
    service::Service,
};
use tempfile::NamedTempFile;
use tracing::Level;

const INPUT_DATA: &[u8] =
    br#"{"name":"bot-upload.s3.amazonaws.com.","timestamp":"2025-05-06T20:19:23.356Z"}
{"name":"webapp.dtes.mh.gob.sv.","timestamp":"2025-05-06T20:19:24.171Z"}
{"name":"binance.org.","timestamp":"2025-05-06T20:19:24.172Z"}
{"name":"akmstatic.ml.youngjoygame.com.","timestamp":"2025-05-06T20:19:24.172Z"}
"#;

const RULES_DATA: &[u8] = br#"
{ "name": "Example string group", "rule_sets": [ { "name": "Test rule set", "string_match": "test long", "preprocessors": [ { "preprocessor_type": "split_target", "ignore_tld": true } ], "match_rules": [ { "rule_type": "levenshtein", "values": { "maximum_distance": 3 } }, { "rule_type": "hamming", "values": { "maximum_distance": 3 } }, { "rule_type": "soundex", "values": { "minimum_similarity": 3 } }, { "rule_type": "metaphone", "values": { "max_code_length": 3 } }, { "rule_type": "nysiis", "values": { "strict": true } }, { "rule_type": "jaro", "values": { "match_percent_threshold": 0.85 } }, { "rule_type": "jaro_winkler", "values": { "match_percent_threshold": 0.85 } }, { "rule_type": "confusables" }, { "rule_type": "match_rating" }, { "rule_type": "damerau_levenshtein", "values": { "maximum_distance": 3 } } ] }, { "name": "Example rule set", "split_target": true, "ignore_tld": true, "string_match": "example", "match_rules": [ { "rule_type": "levenshtein", "values": { "maximum_distance": 3 } }, { "rule_type": "jaro", "values": { "match_percent_threshold": 0.85 } } ] } ] }
"#;

#[test]
fn basic_file_test_report_matches_only() {
    // Prepare files for inputs and rules
    let mut input_file = NamedTempFile::new().expect("Failed creating input file");
    input_file
        .write_all(INPUT_DATA)
        .expect("Failed writing input data");
    let mut output_file = NamedTempFile::new().expect("Failed creating output file");
    let mut rules_file = NamedTempFile::new().expect("Failed creating rules file");
    rules_file
        .write_all(RULES_DATA)
        .expect("Failed writing rules data");

    // Set up the service
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
            report_all: false,
        },
        process: ValidatedProcessConfig {
            threads: 1,
            log_level: Level::INFO,
            shutdown_timeout: Duration::from_secs(60),
        },
    };

    // Run the service
    let (runtime, service) = Service::prepare_from_config(config).expect("building service failed");
    let service = service
        .start(runtime.handle())
        .expect("starting service failed");
    let exit_status = runtime.block_on(service.run());

    // Assert results
    assert_eq!(exit_status.code().unwrap(), exitcode::OK);

    output_file
        .as_file()
        .rewind()
        .expect("Failed rewinding the output file");

    let results = serde_json::Deserializer::from_reader(&mut output_file)
        .into_iter::<Value>()
        .map(|r| {
            r.expect("Failed parsing output")
                .as_object()
                .expect("Expected JSON object")
                .clone()
        })
        .collect::<Vec<Map<String, Value>>>();

    let input_data_parsed = serde_json::Deserializer::from_slice(INPUT_DATA)
        .into_iter::<Value>()
        .map(|r| {
            r.expect("Failed parsing output")
                .as_object()
                .expect("Expected JSON object")
                .clone()
        })
        .collect::<Vec<Map<String, Value>>>();

    assert_eq!(results.len(), input_data_parsed.len());

    // Ensure original data was not modified
    for i in 0..input_data_parsed.len() {
        let input_row = &input_data_parsed[i];
        let results_row = &results[i];
        for (k, v) in input_row {
            assert_eq!(results_row[k], *v);
        }

        println!("{:?}", results_row);
        assert!(!results_row.contains_key("stringsimile"));
    }
}
