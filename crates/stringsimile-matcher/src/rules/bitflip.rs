//! Bitflip rule implementation

use lazy_static::lazy_static;
use std::{collections::HashMap, fmt::Debug, io::Error};

use serde::{Deserialize, Serialize};

use crate::{
    MatcherResult,
    rule::{MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

lazy_static! {
    static ref VALID_CHARS: Vec<u8> = {
        let mut chars = Vec::new();
        for i in 0..u8::MAX {
            let c = char::from(i);
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                chars.push(i);
            }
        }
        chars
    };
    static ref BITFLIP_PATTERNS: Vec<u8> = {
        let mut patterns = Vec::new();
        for i in 0..8 {
            patterns.push(1 << i);
        }
        patterns
    };
    static ref BITFLIPS: HashMap<u8, Vec<u8>> = {
        let mut flips = HashMap::<u8, Vec<u8>>::new();
        for c in VALID_CHARS.iter() {
            for p in BITFLIP_PATTERNS.iter() {
                let flipped = c ^ p;
                if VALID_CHARS.contains(&flipped) {
                    flips.entry(*c).or_default().push(flipped);
                }
            }
        }
        flips
    };
}

/// Rule
#[derive(Debug, Clone)]
pub struct BitflipRule {
    matches_cache: Vec<String>,
    for_target: String,
}

impl BitflipRule {
    /// Creates a new instance of [`BitflipRule`], with cached bitflips for the target string
    pub fn new(target_str: &str) -> Self {
        let cache = Self::matches_for_target(target_str).collect();
        Self {
            matches_cache: cache,
            for_target: target_str.to_string(),
        }
    }

    fn matches_for_target(target_str: &str) -> impl Iterator<Item = String> {
        target_str
            .chars()
            .enumerate()
            .filter(|(_i, c)| c.is_ascii())
            .flat_map(|(i, c)| {
                let string = target_str.to_owned();
                BITFLIPS
                    .get(&(c.try_into().unwrap()))
                    .into_iter()
                    .flatten()
                    .map(move |c| {
                        let mut new_str = string.clone();
                        new_str.replace_range(i..=i, &char::from(*c).to_string());
                        new_str
                    })
            })
    }
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitflipMetadata;

impl MatcherRule for BitflipRule {
    type OutputMetadata = BitflipMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        let matches = if target_str == self.for_target {
            self.matches_cache.iter().any(|f| f == input_str)
        } else {
            Self::matches_for_target(target_str).any(|f| f == input_str)
        };
        if matches {
            MatcherResult::new_match(BitflipMetadata)
        } else {
            MatcherResult::new_no_match(BitflipMetadata)
        }
    }
}

impl RuleMetadata for BitflipMetadata {
    const RULE_NAME: &str = "bitflip";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    #[test]
    fn simple_example() {
        let rule = BitflipRule::new("www.google.com");

        let result = rule.match_rule("wwwngoogle.com", "www.google.com");
        assert!(result.is_match());
        let result = rule.match_rule("licrosoft", "microsoft");
        assert!(result.is_match());
    }

    #[test]
    fn mismatch() {
        let rule = BitflipRule::new("test");

        let result = rule.match_rule("tset", "test");
        assert!(!result.is_match());
        let result = rule.match_rule("unrelated", "microsoft");
        assert!(!result.is_match());
    }
}
