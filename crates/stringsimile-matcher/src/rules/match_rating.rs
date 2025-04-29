//! Match Rating rule implementation

use std::{fmt::Debug, io::Error};

use rphonetic::{Encoder, MatchRatingApproach};
use serde::{Deserialize, Serialize};

use crate::{
    MatcherResult,
    rule::{MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

/// Rule
#[derive(Debug, Clone)]
pub struct MatchRatingRule;

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRatingMetadata;

// TODO replace with custom error
impl MatcherRule for MatchRatingRule {
    type OutputMetadata = MatchRatingMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        if MatchRatingApproach.is_encoded_equals(input_str, target_str) {
            MatcherResult::new_match(MatchRatingMetadata)
        } else {
            MatcherResult::new_no_match(MatchRatingMetadata)
        }
    }
}

impl RuleMetadata for MatchRatingMetadata {
    const RULE_NAME: &str = "match_rating";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    #[test]
    fn simple_example_normal() {
        let rule = MatchRatingRule;

        let result = rule.match_rule("Franciszek", "Frances");
        assert!(result.is_match());
    }
}
