//! Metaphone rule implementation

use std::fmt::Debug;

use rphonetic::{DoubleMetaphone, Encoder, Metaphone};
use serde::{Deserialize, Serialize};
use snafu::Snafu;

use crate::{
    MatcherResult,
    rule::{MatcherResultExt, MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

/// Rule
#[derive(Debug, Clone)]
pub struct MetaphoneRule {
    /// Type of soundex (normal or refined)
    pub metaphone_type: MetaphoneRuleType,
    /// Max code length to generate using metaphone (None for unlimited)
    pub max_code_length: Option<usize>,
}

/// Type of Metaphone (normal or double)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetaphoneRuleType {
    /// Normal
    #[default]
    Normal,
    /// Double
    Double,
}

/// Metaphone rule errors
#[derive(Debug, Clone, Snafu)]
#[snafu(visibility(pub))]
pub enum MetaphoneError {
    /// Used when input string is not ASCII, since it can't be used with metaphone
    #[snafu(display("Metaphone matcher failed. Non-ASCII input: {}", input))]
    NonAsciiInput {
        /// The value of the input string
        input: String,
    },

    /// Used when target string is not ASCII, since it can't be used with metaphone
    #[snafu(display("Metaphone matcher failed. Non-ASCII target: {}", target))]
    NonAsciiTarget {
        /// The value of the target string
        target: String,
    },
}

impl MetaphoneRuleType {
    fn as_str(&self) -> &'static str {
        match self {
            MetaphoneRuleType::Normal => "normal",
            MetaphoneRuleType::Double => "double",
        }
    }
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaphoneMetadata {
    max_code_length: Option<usize>,
    metaphone_type: &'static str,
    primary_code: String,
    alternate_code: Option<String>,
}

impl MatcherRule for MetaphoneRule {
    type OutputMetadata = MetaphoneMetadata;
    type Error = MetaphoneError;

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        if !input_str.is_ascii() {
            return MatcherResult::new_error(MetaphoneError::NonAsciiInput {
                input: input_str.to_string(),
            });
        }

        if !target_str.is_ascii() {
            return MatcherResult::new_error(MetaphoneError::NonAsciiTarget {
                target: target_str.to_string(),
            });
        }

        let (metadata, result) = match self.metaphone_type {
            MetaphoneRuleType::Normal => {
                let metaphone = Metaphone::new(self.max_code_length);
                let res_code = metaphone.encode(input_str);
                let target_code = metaphone.encode(target_str);
                let result = res_code == target_code;
                (
                    MetaphoneMetadata {
                        max_code_length: self.max_code_length,
                        metaphone_type: self.metaphone_type.as_str(),
                        primary_code: res_code,
                        alternate_code: None,
                    },
                    result,
                )
            }
            MetaphoneRuleType::Double => {
                let metaphone = DoubleMetaphone::new(self.max_code_length);
                let primary_code = metaphone.encode(input_str);
                let alternate_code = metaphone.encode_alternate(input_str);
                let target_primary = metaphone.encode(target_str);
                let target_alternate = metaphone.encode_alternate(target_str);
                let result = primary_code == target_primary || alternate_code == target_alternate;
                (
                    MetaphoneMetadata {
                        max_code_length: self.max_code_length,
                        metaphone_type: self.metaphone_type.as_str(),
                        primary_code,
                        alternate_code: Some(alternate_code),
                    },
                    result,
                )
            }
        };

        if result {
            MatcherResult::new_match(metadata)
        } else {
            MatcherResult::new_no_match(metadata)
        }
    }
}

impl RuleMetadata for MetaphoneMetadata {
    const RULE_NAME: &str = "metaphone";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    #[test]
    fn simple_example_normal() {
        let rule = MetaphoneRule {
            metaphone_type: MetaphoneRuleType::Normal,
            max_code_length: Some(4),
        };

        let result = rule.match_rule("Selena", "Selina");
        assert!(result.is_match());
        let metadata = result.into_metadata();
        assert_eq!(metadata.max_code_length, Some(4));
        assert_eq!(metadata.metaphone_type, "normal");
        assert_eq!(metadata.primary_code, "SLN");
        assert_eq!(metadata.alternate_code, None);
    }

    #[test]
    fn simple_example_double() {
        let rule = MetaphoneRule {
            metaphone_type: MetaphoneRuleType::Double,
            max_code_length: Some(4),
        };

        let result = rule.match_rule("Brian", "Bryan");
        assert!(result.is_match());
        let metadata = result.into_metadata();
        assert_eq!(metadata.max_code_length, Some(4));
        assert_eq!(metadata.metaphone_type, "double");
        assert_eq!(metadata.primary_code, "PRN");
        assert_eq!(metadata.alternate_code, Some("PRN".to_string()));
    }
}
