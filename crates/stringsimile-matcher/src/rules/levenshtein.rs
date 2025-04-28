//! Levenshtein rule implementation

use std::io::Error;

use serde::{Deserialize, Serialize};
use triple_accel::levenshtein_exp;

use crate::{
    MatcherResult,
    rule::{MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

/// Rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevenshteinRule {
    /// Maximum distance allowed for this rule to be considered matched
    pub maximum_distance: u32,
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevenshteinMetadata {
    #[allow(unused)]
    distance: u32,
}

// TODO: replace with custom errors
impl MatcherRule for LevenshteinRule {
    type OutputMetadata = LevenshteinMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
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
            maximum_distance: 2.try_into().unwrap(),
        };

        let result = rule.match_rule("test", "tset");
        assert!(result.is_match());
        assert_eq!(result.into_metadata().distance, 2);
    }
}
