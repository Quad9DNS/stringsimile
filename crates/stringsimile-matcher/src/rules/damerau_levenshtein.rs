//! Damerau DamerauLevenshtein rule implementation

use std::io::Error;

use serde::{Deserialize, Serialize};
use triple_accel::rdamerau_exp;

use crate::{
    MatcherResult,
    rule::{MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

/// Rule
#[derive(Debug, Clone)]
pub struct DamerauLevenshteinRule {
    /// Maximum distance allowed for this rule to be considered matched
    pub maximum_distance: u32,
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamerauLevenshteinMetadata {
    #[allow(unused)]
    distance: u32,
}

// TODO: replace with custom errors
impl MatcherRule for DamerauLevenshteinRule {
    type OutputMetadata = DamerauLevenshteinMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        let res = rdamerau_exp(input_str.as_bytes(), target_str.as_bytes());
        let metadata = DamerauLevenshteinMetadata { distance: res };
        if res <= self.maximum_distance {
            MatcherResult::new_match(metadata)
        } else {
            MatcherResult::new_no_match(metadata)
        }
    }
}

impl RuleMetadata for DamerauLevenshteinMetadata {
    const RULE_NAME: &str = "damerau_levenshtein";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    #[test]
    fn simple_example() {
        let rule = DamerauLevenshteinRule {
            maximum_distance: 2,
        };

        let result = rule.match_rule("test", "tset");
        assert!(result.is_match());
        assert_eq!(result.into_metadata().distance, 1);
    }
}
