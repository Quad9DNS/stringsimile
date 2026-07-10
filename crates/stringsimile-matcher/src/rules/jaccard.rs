//! Jaccard similarity rule implementation

use std::io::Error;

use hashbrown::HashSet;
use serde::{Deserialize, Serialize};

use crate::{
    MatcherResult,
    rule::{EstimationResult, MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

/// Rule
#[derive(Debug, Clone)]
pub struct JaccardRule {
    /// Minimum similarity for this rule to be considered a match
    pub minimum_similarity: f64,
    target_set: HashSet<char>,
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JaccardMetadata {
    #[allow(unused)]
    similarity: f64,
}

impl JaccardRule {
    /// Creates an instance of JaccardRule with pre-compute target string chars set
    pub fn new(minimum_similarity: f64, target_str: &str) -> Self {
        Self {
            minimum_similarity,
            target_set: target_str.chars().collect(),
        }
    }
}

impl MatcherRule for JaccardRule {
    type OutputMetadata = JaccardMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        _target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        let input_set = input_str.chars().collect::<HashSet<_>>();
        let res = input_set.intersection(&self.target_set).count() as f64
            / input_set.union(&self.target_set).count() as f64;
        let metadata = JaccardMetadata { similarity: res };
        if res >= self.minimum_similarity {
            MatcherResult::new_match(metadata)
        } else {
            MatcherResult::new_no_match(metadata)
        }
    }

    fn estimate(&self, _target_str: &str) -> EstimationResult {
        EstimationResult::linear(10 + self.target_set.len(), 2.0)
    }
}

impl RuleMetadata for JaccardMetadata {
    const RULE_NAME: &str = "jaccard";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    fn round_assert_eq(left: f64, right: f64) {
        assert_eq!(
            (left * 100.0).round() / 100.0,
            (right * 100.0).round() / 100.0
        );
    }

    #[test]
    fn simple_example() {
        let rule = JaccardRule::new(0.8, "example");

        let result = rule.match_rule("exemple", "example");
        assert!(result.is_match());
        round_assert_eq(result.into_metadata().similarity, 0.83);
    }
}
