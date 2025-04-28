//! Configuration for rules
use serde::{Deserialize, Serialize};
use stringsimile_matcher::{
    rule::{GenericMatcherRule, IntoGenericMatcherRule},
    rules::{confusables::ConfusablesRule, jaro::JaroRule, levenshtein::LevenshteinRule},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "rule_type", rename_all = "snake_case", content = "values")]
/// Configuration for rules
pub enum RuleConfig {
    /// Configuration for Levenshtein rule
    Levenshtein(LevenshteinConfig),
    /// Configuration for Confusables rule
    Confusables,
    /// Configuration for Jaro rule
    Jaro(JaroConfig),
}

impl RuleConfig {
    /// Generates a rule implementation from this config
    pub fn build(&self) -> Box<dyn GenericMatcherRule + 'static + Send> {
        match self {
            RuleConfig::Levenshtein(levenshtein_config) => {
                Box::new(levenshtein_config.build().into_generic_matcher())
            }
            RuleConfig::Confusables => Box::new(ConfusablesConfig.build().into_generic_matcher()),
            RuleConfig::Jaro(jaro_config) => Box::new(jaro_config.build().into_generic_matcher()),
        }
    }
}

/// Configuration for Levenshtein rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevenshteinConfig {
    /// Maximum distance
    pub maximum_distance: u32,
}

impl LevenshteinConfig {
    fn build(&self) -> LevenshteinRule {
        LevenshteinRule {
            maximum_distance: self.maximum_distance,
        }
    }
}

/// Configuration for Confusables rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfusablesConfig;

impl ConfusablesConfig {
    fn build(&self) -> ConfusablesRule {
        ConfusablesRule
    }
}

/// Configuration for Jaro rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JaroConfig {
    /// Maximum distance
    pub match_percent_threshold: f64,
}

impl JaroConfig {
    fn build(&self) -> JaroRule {
        JaroRule {
            match_percent: self.match_percent_threshold,
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
        assert_eq!(3, config.maximum_distance);
    }

    #[test]
    fn test_parse_jaro() {
        let json = r#"
        {
            "rule_type": "jaro",
            "values": {
                "match_percent_threshold": 0.4
            }
        }
            "#;

        let RuleConfig::Jaro(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Jaro config");
        };
        assert_eq!(0.4, config.match_percent_threshold);
    }

    #[test]
    fn test_parse_confusables() {
        let json = r#"
        {
            "rule_type": "confusables"
        }
            "#;

        let RuleConfig::Confusables = serde_json::from_str(json).unwrap() else {
            panic!("Expected Confusables config");
        };
    }
}
