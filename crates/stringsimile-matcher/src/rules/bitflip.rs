//! Bitflip rule implementation

use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    io::Error,
};

use serde::{Deserialize, Serialize};

use crate::{
    MatcherResult,
    rule::{MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

const BITFLIP_PATTERNS: [u8; 8] = [
    0b10000000, 0b01000000, 0b00100000, 0b00010000, 0b00001000, 0b00000100, 0b00000010, 0b00000001,
];

/// Rule
#[derive(Debug, Clone)]
pub struct BitflipRule {
    matches_cache: HashSet<String>,
    case_sensitive: bool,
}

impl BitflipRule {
    /// Creates a new instance of [`BitflipRule`], with cached bitflips for the target string.
    ///
    /// Bitflips are limited to provided valid characters.
    pub fn new(valid_chars: &Vec<u8>, target_str: &str, case_sensitive: bool) -> Self {
        let bitflips = Self::calculate_bitflips_for_chars(valid_chars);
        let cache = Self::matches_for_target(target_str, &bitflips, case_sensitive).collect();
        Self {
            matches_cache: cache,
            case_sensitive,
        }
    }

    /// Creates a new instance of [`BitflipRule`], with cached bitflips for the target string.
    ///
    /// Bitflips are limited to chars valid in DNS context (a-z,A-Z,0-9,-,.).
    pub fn new_dns(target_str: &str, case_sensitive: bool) -> Self {
        let valid_chars = ('a'..='z')
            .chain('A'..='Z')
            .chain('0'..='9')
            .chain(['-', '.'])
            .map(|c| u8::try_from(c).expect("DNS character out of ASCII range"))
            .collect();
        Self::new(&valid_chars, target_str, case_sensitive)
    }

    /// Creates a new instance of [`BitflipRule`], with cached bitflips for the target string.
    ///
    /// Bitflips are limited to printable ASCII characters.
    pub fn new_ascii_printable(target_str: &str, case_sensitive: bool) -> Self {
        let valid_chars = (0x21..=0x7E).collect();
        Self::new(&valid_chars, target_str, case_sensitive)
    }

    fn calculate_bitflips_for_chars(valid_chars: &Vec<u8>) -> HashMap<u8, Vec<u8>> {
        let mut flips = HashMap::<u8, Vec<u8>>::new();
        for c in valid_chars {
            for p in BITFLIP_PATTERNS.iter() {
                let flipped = c ^ p;
                if valid_chars.contains(&flipped) {
                    flips.entry(*c).or_default().push(flipped);
                }
            }
        }
        flips
    }

    fn matches_for_target(
        target_str: &str,
        bitflips: &HashMap<u8, Vec<u8>>,
        case_sensitive: bool,
    ) -> impl Iterator<Item = String> {
        target_str
            .chars()
            .enumerate()
            .filter(|(_i, c)| c.is_ascii())
            .flat_map(move |(i, c)| {
                let string = target_str.to_owned();
                bitflips
                    .get(&(c.try_into().unwrap()))
                    .into_iter()
                    .flatten()
                    .map(move |c| {
                        let mut new_str = string.clone();
                        new_str.replace_range(i..=i, &char::from(*c).to_string());
                        if !case_sensitive {
                            new_str = new_str.to_lowercase();
                        }
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
        _target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        let matches = if self.case_sensitive {
            self.matches_cache.contains(input_str)
        } else {
            self.matches_cache.contains(&input_str.to_lowercase())
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
        let rule = BitflipRule::new_dns("www.google.com", true);

        let result = rule.match_rule("wwwngoogle.com", "www.google.com");
        assert!(result.is_match());
        let result = rule.match_rule("licrosoft", "microsoft");
        assert!(result.is_match());
    }

    #[test]
    fn simple_example_case_insensitive() {
        let rule = BitflipRule::new_dns("www.google.com", false);

        let result = rule.match_rule("WWWNGOOGLE.COM", "www.google.com");
        assert!(result.is_match());
        let result = rule.match_rule("licrosoft", "microsoft");
        assert!(result.is_match());
    }

    #[test]
    fn simple_example_custom_charset() {
        let rule = BitflipRule::new(&vec![b'.', b'n'], "www.google.com", true);

        let result = rule.match_rule("wwwngoogle.com", "www.google.com");
        assert!(result.is_match());
        let result = rule.match_rule("licrosoft", "microsoft");
        assert!(!result.is_match());
    }

    #[test]
    fn mismatch() {
        let rule = BitflipRule::new_dns("test", true);

        let result = rule.match_rule("tset", "test");
        assert!(!result.is_match());
        let result = rule.match_rule("unrelated", "microsoft");
        assert!(!result.is_match());
    }
}
