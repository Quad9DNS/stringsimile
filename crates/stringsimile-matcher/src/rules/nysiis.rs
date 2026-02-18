//! NYSIIS rule implementation

use std::{fmt::Debug, io::Error};

use rphonetic::{Encoder, Nysiis};
use serde::{Deserialize, Serialize};

use crate::{
    MatcherResult,
    rule::{MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

/// Rule
#[derive(Debug, Clone)]
pub struct NysiisRule {
    /// Built NYSIIS matcher
    nysiis: Nysiis,
    /// Pre-encoded target string
    encoded_target: String,
}

impl NysiisRule {
    /// Creates a new NysiisRule with given strict mode
    pub fn new(strict: bool, target_str: &str) -> Self {
        let nysiis = Nysiis::new(strict);
        Self {
            nysiis,
            encoded_target: nysiis.encode(target_str),
        }
    }
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NysiisMetadata {
    encoded: String,
}

impl MatcherRule for NysiisRule {
    type OutputMetadata = NysiisMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        _target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        let encoded = self.nysiis.encode(input_str);
        let result = encoded == self.encoded_target;
        let metadata = NysiisMetadata { encoded };
        if result {
            MatcherResult::new_match(metadata)
        } else {
            MatcherResult::new_no_match(metadata)
        }
    }
}

impl RuleMetadata for NysiisMetadata {
    const RULE_NAME: &str = "nysiis";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    #[test]
    fn simple_example_normal() {
        let rule = NysiisRule::new(true, "Brown");

        let result = rule.match_rule("Brian", "Brown");
        assert!(result.is_match());
        let metadata = result.into_metadata();
        assert_eq!(metadata.encoded, "BRAN");
    }
}
