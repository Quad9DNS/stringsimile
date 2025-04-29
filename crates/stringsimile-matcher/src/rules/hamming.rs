//! Hamming rule implementation

use std::io::Error;

use serde::{Deserialize, Serialize};
use triple_accel::hamming;

use crate::{
    MatcherResult,
    rule::{MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

/// Rule
#[derive(Debug, Clone)]
pub struct HammingRule {
    /// Maximum distance allowed for this rule to be considered matched
    pub maximum_distance: u32,
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HammingMetadata {
    #[allow(unused)]
    distance: Option<u32>,
}

// TODO: replace with custom errors
impl MatcherRule for HammingRule {
    type OutputMetadata = HammingMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        if input_str.len() != target_str.len() {
            return MatcherResult::new_no_match(HammingMetadata { distance: None });
        }

        let res = hamming(input_str.as_bytes(), target_str.as_bytes());
        let metadata = HammingMetadata {
            distance: Some(res),
        };
        if res <= self.maximum_distance {
            MatcherResult::new_match(metadata)
        } else {
            MatcherResult::new_no_match(metadata)
        }
    }
}

impl RuleMetadata for HammingMetadata {
    const RULE_NAME: &str = "hamming";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    #[test]
    fn simple_example() {
        let rule = HammingRule {
            maximum_distance: 2,
        };

        let result = rule.match_rule("test", "tset");
        assert!(result.is_match());
        assert_eq!(result.into_metadata().distance, Some(2));
    }

    #[test]
    fn different_length_example_example() {
        let rule = HammingRule {
            maximum_distance: 2,
        };

        let result = rule.match_rule("test", "longer string");
        assert!(!result.is_match());
        assert_eq!(result.into_metadata().distance, None);
    }
}
