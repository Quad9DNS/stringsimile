//! Soundex rule implementation

use std::{fmt::Debug, io::Error};

use rphonetic::{RefinedSoundex, Soundex, SoundexCommons};
use serde::{Deserialize, Serialize};

use crate::{
    MatcherResult,
    rule::{MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

/// Rule
#[derive(Debug, Clone)]
pub struct SoundexRule {
    /// Type of soundex (normal or refined)
    soundex_type: SoundexRuleType,
    /// Minimum similarity value to consider rule matched
    minimum_similarity: usize,
    /// Pre-encoded target string
    encoded_target: String,
}

impl SoundexRule {
    /// Creates an instance of soundex rule with pre-computed target string encoding
    pub fn new(soundex_type: SoundexRuleType, minimum_similarity: usize, target_str: &str) -> Self {
        Self {
            soundex_type,
            minimum_similarity,
            encoded_target: soundex_type.build_soundex().encode(target_str),
        }
    }
}

/// Type of Soundex (normal or refined)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SoundexRuleType {
    /// Normal
    #[default]
    Normal,
    /// Refined
    Refined,
}

impl SoundexRuleType {
    fn as_str(&self) -> &'static str {
        match self {
            SoundexRuleType::Normal => "normal",
            SoundexRuleType::Refined => "refined",
        }
    }

    fn build_soundex(&self) -> Box<dyn SoundexCommons> {
        match self {
            SoundexRuleType::Normal => Box::new(Soundex::default()),
            SoundexRuleType::Refined => Box::new(RefinedSoundex::default()),
        }
    }
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundexMetadata {
    similarity: usize,
    soundex_type: &'static str,
}

// TODO replace with custom error
impl MatcherRule for SoundexRule {
    type OutputMetadata = SoundexMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        _target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        let mut res = 0usize;
        if !self.encoded_target.is_empty() {
            let soundex = self.soundex_type.build_soundex();
            let input = soundex.encode(input_str);
            if !input.is_empty() {
                res = input
                    .chars()
                    .zip(self.encoded_target.chars())
                    .filter(|(l, r)| l == r)
                    .count();
            }
        };

        let metadata = SoundexMetadata {
            similarity: res,
            soundex_type: self.soundex_type.as_str(),
        };
        if res >= self.minimum_similarity {
            MatcherResult::new_match(metadata)
        } else {
            MatcherResult::new_no_match(metadata)
        }
    }
}

impl RuleMetadata for SoundexMetadata {
    const RULE_NAME: &str = "soundex";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    #[test]
    fn simple_example_normal() {
        let rule = SoundexRule::new(SoundexRuleType::Normal, 3, "Smythers");

        let result = rule.match_rule("Smithers", "Smythers");
        assert!(result.is_match());
        let metadata = result.into_metadata();
        assert_eq!(metadata.similarity, 4);
        assert_eq!(metadata.soundex_type, "normal");
    }

    #[test]
    fn simple_example_refined() {
        let rule = SoundexRule::new(SoundexRuleType::Refined, 3, "Smythers");

        let result = rule.match_rule("Smithers", "Smythers");
        assert!(result.is_match());
        let metadata = result.into_metadata();
        assert_eq!(metadata.similarity, 8);
        assert_eq!(metadata.soundex_type, "refined");
    }
}
