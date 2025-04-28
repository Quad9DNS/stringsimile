//! JaroWinkler-Winkler rule implementation

use std::io::Error;

use serde::{Deserialize, Serialize};
use strsim::jaro_winkler;

use crate::{
    MatcherResult,
    rule::{MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

/// Rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JaroWinklerRule {
    /// Minimum match percentage for this rule to be considered a match
    pub match_percent: f64,
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JaroWinklerMetadata {
    #[allow(unused)]
    match_percent: f64,
}

// TODO replace with custom error
impl MatcherRule for JaroWinklerRule {
    type OutputMetadata = JaroWinklerMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        let res = jaro_winkler(input_str, target_str);
        let metadata = JaroWinklerMetadata { match_percent: res };
        if res >= self.match_percent {
            MatcherResult::new_match(metadata)
        } else {
            MatcherResult::new_no_match(metadata)
        }
    }
}

impl RuleMetadata for JaroWinklerMetadata {
    const RULE_NAME: &str = "jaro_winkler";
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
        let rule = JaroWinklerRule {
            match_percent: 0.85,
        };

        let result = rule.match_rule("example", "exemple");
        assert!(result.is_match());
        round_assert_eq(result.into_metadata().match_percent, 0.92);
    }
}
