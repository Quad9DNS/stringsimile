use snafu::Snafu;
use stringsimile_matcher::Error;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum StringsimileServiceError {
    #[snafu(display("File not found: {:?}", source))]
    FileNotFound { source: std::io::Error },

    #[snafu(display("Parsing matcher rules failed: {:?}", source))]
    RuleParsing { source: Error },

    #[snafu(display("Parsing matcher rules JSON file failed: {:?}", source))]
    RuleJsonParsing { source: serde_json::Error },

    #[snafu(display("Parsing config YAML file failed: {:?}", source))]
    ConfigYamlParsing { source: serde_yaml::Error },

    #[snafu(display("Preparing input {} has failed: {:?}", input_name, source))]
    InputFail { input_name: String, source: Error },
}
