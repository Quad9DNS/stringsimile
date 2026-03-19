//! Regex rule implementation

use std::{fmt::Debug, io::Error};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    MatcherResult,
    rule::{MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

/// Rule
#[derive(Debug, Clone)]
pub struct RegexRule {
    pattern: Regex,
}

impl RegexRule {
    /// Creates a new instance of [`RegexRule`], with compiled pattern.
    pub fn new(pattern: Regex) -> Self {
        Self { pattern }
    }
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegexMetadata;

impl MatcherRule for RegexRule {
    type OutputMetadata = RegexMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        _target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        if self.pattern.is_match(input_str) {
            MatcherResult::new_match(RegexMetadata)
        } else {
            MatcherResult::new_no_match(RegexMetadata)
        }
    }
}

impl RuleMetadata for RegexMetadata {
    const RULE_NAME: &str = "regex";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    #[test]
    fn simple_example() {
        let rule = RegexRule::new(Regex::new(r#"netflix\.com\.$"#).unwrap());

        let result = rule.match_rule("netflix.com.", "netflix.com.");
        assert!(result.is_match());
        let result = rule.match_rule("netflix.com", "netflix.com.");
        assert!(!result.is_match());
        let result = rule.match_rule("neftlix.com.", "netflix.com.");
        assert!(!result.is_match());
    }

    #[test]
    fn complex_pattern_example() {
        let rule = RegexRule::new(Regex::new(r#".*n.*t.*f.*"#).unwrap());

        let result = rule.match_rule("netflix.com.", "netflix.com.");
        assert!(result.is_match());
        let result = rule.match_rule("netflix.com", "netflix.com.");
        assert!(result.is_match());
        let result = rule.match_rule("neftlix.com.", "netflix.com.");
        assert!(!result.is_match());
    }
}
