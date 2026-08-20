use std::{collections::HashSet, io::Write, time::Duration};

use stringsimile_service::{
    config::{MatcherConfig, ServiceConfig, ValidatedMetricsConfig, ValidatedProcessConfig},
    field_access::FieldAccessorConfig,
    inputs::Input,
    outputs::Output,
    processor::StringProcessor,
};
use tempfile::NamedTempFile;
use tracing::Level;

const RULES_DATA: &[u8] = br#"
{ "name": "Example string group", "rule_sets": [ { "name": "Example rule set", "preprocessors": [ { "preprocessor_type": "split_target", "ignore_tld": true } ], "string_match": "example", "match_rules": [ { "rule_type": "levenshtein", "values": { "maximum_distance": 3 } } ] } ] }
{ "name": "Example string group", "rule_sets": [ { "name": "Example rule set 2", "preprocessors": [ { "preprocessor_type": "split_target", "ignore_tld": true } ], "string_match": "example", "match_rules": [ { "rule_type": "levenshtein", "values": { "maximum_distance": 3 } } ] } ] }
"#;

#[tokio::test]
async fn duplicate_string_group_test() {
    // Prepare files for inputs and rules
    let input_file = NamedTempFile::new().expect("Failed creating input file");
    let output_file = NamedTempFile::new().expect("Failed creating output file");
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
            report_all: true,
        },
        process: ValidatedProcessConfig {
            threads: 1,
            log_level: Level::INFO,
            shutdown_timeout: Duration::from_secs(60),
            enable_config_reload: false,
        },
    };

    let Err(err) = StringProcessor::load_rules(&config.matcher).await else {
        panic!("Expected loading rules to failed due to duplicate string group name");
    };

    assert_eq!(
        err.to_string(),
        "Parsing matcher rules failed: Found a duplicate string group name (\"Example string group\")."
    );
}
