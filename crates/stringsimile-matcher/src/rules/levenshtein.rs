//! Levenshtein rule implementation

use std::io::Error;

use serde::{Deserialize, Serialize};
use triple_accel::levenshtein_exp;

use crate::{
    MatcherResult,
    rule::{
        MatcherResultRuleMetadataExt, MatcherResultRuleOptionMetadataExt, MatcherRule, RuleMetadata,
    },
};

/// Rule
#[derive(Debug, Clone)]
pub struct LevenshteinRule {
    /// Maximum distance allowed for this rule to be considered matched
    pub maximum_distance: u32,
    /// Uses hardcoded metadata for mismatches
    /// Makes the matcher faster, but makes the metadata invalid for mismatches
    pub ignore_mismatch_metadata: bool,
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevenshteinMetadata {
    #[allow(unused)]
    distance: u32,
}

// TODO: replace with custom errors
impl MatcherRule for LevenshteinRule {
    type OutputMetadata = Option<LevenshteinMetadata>;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        if self.ignore_mismatch_metadata
            && input_str.len().abs_diff(target_str.len()) > self.maximum_distance as usize
        {
            return MatcherResult::new_no_match_no_metadata();
        }

        let res = levenshtein_exp(input_str.as_bytes(), target_str.as_bytes());
        let metadata = LevenshteinMetadata { distance: res };
        if res <= self.maximum_distance {
            MatcherResult::new_match(metadata)
        } else {
            MatcherResult::new_no_match(metadata)
        }
    }
}

impl RuleMetadata for LevenshteinMetadata {
    const RULE_NAME: &str = "levenshtein";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    #[test]
    fn simple_example() {
        let rule = LevenshteinRule {
            maximum_distance: 2,
            ignore_mismatch_metadata: true,
        };

        let result = rule.match_rule("test", "tset");
        assert!(result.is_match());
        assert_eq!(result.into_metadata().unwrap().distance, 2);
    }

    #[test]
    fn simple_example_ignore_mismatch_metadata() {
        let rule = LevenshteinRule {
            maximum_distance: 1,
            ignore_mismatch_metadata: true,
        };

        let result = rule.match_rule("test", "tsettest");
        assert!(!result.is_match());
        assert!(result.into_metadata().is_none());
    }

    #[test]
    fn simple_example_provide_mismatch_metadata() {
        let rule = LevenshteinRule {
            maximum_distance: 1,
            ignore_mismatch_metadata: false,
        };

        let result = rule.match_rule("test", "tsettest");
        assert!(!result.is_match());
        assert_eq!(result.into_metadata().unwrap().distance, 4);
    }
}
