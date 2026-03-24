//! Damerau DamerauLevenshtein rule implementation

use std::io::Error;

use serde::{Deserialize, Serialize};
use triple_accel::rdamerau_exp;

use crate::{
    MatcherResult,
    rule::{
        MatcherResultRuleMetadataExt, MatcherResultRuleOptionMetadataExt, MatcherRule, RuleMetadata,
    },
};

/// Rule
#[derive(Debug, Clone)]
pub struct DamerauLevenshteinRule {
    /// Maximum distance allowed for this rule to be considered matched
    pub maximum_distance: u32,
    /// Uses hardcoded metadata for mismatches
    /// Makes the matcher faster, but makes the metadata invalid for mismatches
    pub ignore_mismatch_metadata: bool,
}

/// Damerau-Levenshtein rule which accepts substring matches too (substrings matching target string
/// in length)
#[derive(Debug, Clone)]
pub struct DamerauLevenshteinSubstringRule {
    /// Maximum distance allowed for this rule to be considered matched
    pub maximum_distance: u32,
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamerauLevenshteinMetadata {
    #[allow(unused)]
    distance: u32,
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamerauLevenshteinSubstringMetadata {
    #[allow(unused)]
    distance: u32,
    substring: Option<String>,
}

impl MatcherRule for DamerauLevenshteinRule {
    type OutputMetadata = Option<DamerauLevenshteinMetadata>;
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

        let res = rdamerau_exp(input_str.as_bytes(), target_str.as_bytes());
        let metadata = DamerauLevenshteinMetadata { distance: res };
        if res <= self.maximum_distance {
            MatcherResult::new_match(metadata)
        } else {
            MatcherResult::new_no_match(metadata)
        }
    }
}

impl MatcherRule for DamerauLevenshteinSubstringRule {
    type OutputMetadata = Option<DamerauLevenshteinSubstringMetadata>;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        let res = rdamerau_exp(input_str.as_bytes(), target_str.as_bytes());
        let metadata = DamerauLevenshteinSubstringMetadata {
            distance: res,
            substring: None,
        };

        if res <= self.maximum_distance {
            return MatcherResult::new_match(metadata);
        }

        if input_str.len() > target_str.len() {
            let mut start = 0;
            let mut substring = &input_str[start..start + target_str.len()];
            let mut res = rdamerau_exp(substring.as_bytes(), target_str.as_bytes());
            while res > self.maximum_distance && start + target_str.len() < input_str.len() {
                let diff = (((res - self.maximum_distance) / 2) as usize)
                    .min(input_str.len() - target_str.len() - start - 1);
                if diff == 0 {
                    break;
                }

                start += diff;
                substring = &input_str[start..start + target_str.len()];
                res = rdamerau_exp(substring.as_bytes(), target_str.as_bytes());
            }

            let metadata = DamerauLevenshteinSubstringMetadata {
                distance: res,
                substring: Some(substring.to_string()),
            };
            if res <= self.maximum_distance {
                MatcherResult::new_match(metadata)
            } else {
                MatcherResult::new_no_match(metadata)
            }
        } else {
            MatcherResult::new_no_match(metadata)
        }
    }
}

impl RuleMetadata for DamerauLevenshteinMetadata {
    const RULE_NAME: &str = "damerau_levenshtein";
}

impl RuleMetadata for DamerauLevenshteinSubstringMetadata {
    const RULE_NAME: &str = "damerau_levenshtein_substring";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    #[test]
    fn simple_example() {
        let rule = DamerauLevenshteinRule {
            maximum_distance: 2,
            ignore_mismatch_metadata: true,
        };

        let result = rule.match_rule("test", "tset");
        assert!(result.is_match());
        assert_eq!(result.into_metadata().unwrap().distance, 1);
    }

    #[test]
    fn simple_example_ignore_mismatch_metadata() {
        let rule = DamerauLevenshteinRule {
            maximum_distance: 1,
            ignore_mismatch_metadata: true,
        };

        let result = rule.match_rule("test", "tsettest");
        assert!(!result.is_match());
        assert!(result.into_metadata().is_none());
    }

    #[test]
    fn simple_example_provide_mismatch_metadata() {
        let rule = DamerauLevenshteinRule {
            maximum_distance: 1,
            ignore_mismatch_metadata: false,
        };

        let result = rule.match_rule("test", "tsettest");
        assert!(!result.is_match());
        assert_eq!(result.into_metadata().unwrap().distance, 4);
    }

    #[test]
    fn simple_example_substring_match() {
        let rule = DamerauLevenshteinSubstringRule {
            maximum_distance: 1,
        };

        let result = rule.match_rule("confirmimonneftlixconfirmation", "netflix");
        assert!(result.is_match());
        let metadata = result.into_metadata().unwrap();
        assert_eq!(metadata.distance, 1);
        assert_eq!(metadata.substring, Some("neftlix".to_string()));
    }

    #[test]
    fn simple_example_substring_match_shorter_string() {
        let rule = DamerauLevenshteinSubstringRule {
            maximum_distance: 1,
        };

        let result = rule.match_rule("neftli", "netflix");
        assert!(!result.is_match());
        let metadata = result.into_metadata().unwrap();
        assert_eq!(metadata.distance, 2);
        assert_eq!(metadata.substring, None);
    }

    #[test]
    fn simple_example_substring_mismatch() {
        let rule = DamerauLevenshteinSubstringRule {
            maximum_distance: 1,
        };

        let result = rule.match_rule("somecompletelyotherstring", "netflix");
        assert!(!result.is_match());
    }
}
