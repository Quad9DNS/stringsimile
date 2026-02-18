//! Jaro rule implementation

use std::io::Error;

use serde::{Deserialize, Serialize};
use strsim::jaro;

use crate::{
    MatcherResult,
    rule::{MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

/// Rule
#[derive(Debug, Clone)]
pub struct JaroRule {
    /// Minimum match percentage for this rule to be considered a match
    pub match_percent: f64,
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JaroMetadata {
    #[allow(unused)]
    match_percent: f64,
}

impl MatcherRule for JaroRule {
    type OutputMetadata = JaroMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        let res = jaro(input_str, target_str);
        let metadata = JaroMetadata { match_percent: res };
        if res >= self.match_percent {
            MatcherResult::new_match(metadata)
        } else {
            MatcherResult::new_no_match(metadata)
        }
    }
}

impl RuleMetadata for JaroMetadata {
    const RULE_NAME: &str = "jaro";
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
        let rule = JaroRule {
            match_percent: 0.85,
        };

        let result = rule.match_rule("example", "exemple");
        assert!(result.is_match());
        round_assert_eq(result.into_metadata().match_percent, 0.9);
    }
}
