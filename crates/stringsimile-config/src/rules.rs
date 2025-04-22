//! Configuration for rules
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use stringsimile_matcher::rules::levenshtein::LevenhsteinRule;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "rule_type", rename_all = "snake_case", content = "values")]
/// Configuration for rules
pub enum RuleConfig {
    /// Configuration for  Levenshtein rule
    Levenshtein(LevenshteinConfig),
    /// TODO remove
    Remove,
}

/// Configuration for Levenshtein rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevenshteinConfig {
    /// Maximum distance
    pub maximum_distance: NonZeroU32,
}

impl LevenshteinConfig {
    #[allow(unused)]
    fn build(&self) -> LevenhsteinRule {
        LevenhsteinRule {
            maximum_distance: self.maximum_distance,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let json = r#"
        {
            "rule_type": "levenshtein",
            "values": {
                "maximum_distance": 3
            }
        }
            "#;

        let RuleConfig::Levenshtein(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Levenshtein config");
        };
        assert_eq!(3, config.maximum_distance.get());
    }
}
