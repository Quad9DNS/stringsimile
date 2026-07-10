//! IDN confusables rule implementation

use std::io::Error;

use confusables::Confusable;
use serde::{Deserialize, Serialize};

use crate::{
    MatcherResult,
    rule::{EstimationResult, MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

/// Rule
#[derive(Debug, Clone)]
pub struct ConfusablesRule;

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfusablesMetadata;

impl MatcherRule for ConfusablesRule {
    type OutputMetadata = ConfusablesMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        let metadata = ConfusablesMetadata;
        let confusable = input_str.is_confusable_with(target_str);
        if confusable && target_str != input_str {
            MatcherResult::new_match(metadata)
        } else {
            MatcherResult::new_no_match(metadata)
        }
    }

    fn estimate(&self, _target_str: &str) -> EstimationResult {
        EstimationResult::linear(20, 1.0)
    }
}

impl RuleMetadata for ConfusablesMetadata {
    const RULE_NAME: &str = "confusables";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    #[test]
    fn simple_example() {
        let rule = ConfusablesRule;

        let result = rule.match_rule("t℮st", "test");
        assert!(result.is_match());
    }

    #[test]
    fn test_rules_file_example() {
        let rule = ConfusablesRule;

        let result = rule.match_rule("𝓗℮𝐥1೦", "Hello");
        assert!(result.is_match());
    }

    #[test]
    fn exact_match_example() {
        let rule = ConfusablesRule;

        let result = rule.match_rule("hello", "hello");
        assert!(!result.is_match());
    }

    #[test]
    fn unrelated_strings() {
        let rule = ConfusablesRule;

        let result = rule.match_rule("𝓗℮𝐥1೦", "test");
        assert!(!result.is_match());
    }
}
