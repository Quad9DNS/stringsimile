//! Rulesets configuration

use serde::{Deserialize, Serialize};
use stringsimile_matcher::{
    Error,
    preprocessors::{Preprocessor, SplitTargetConfig},
    ruleset::{RuleSet, StringGroup},
};

use crate::rules::RuleConfig;

/// Configuration for a rule set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSetConfig {
    name: String,
    string_match: String,
    #[serde(default)]
    preprocessors: Vec<PreprocessorConfig>,
    match_rules: Vec<RuleConfig>,
}

impl RuleSetConfig {
    /// Convert into RuleSet that can be used for matching
    ///
    /// `ignore_mismatch_metadata` flag can be enabled to potentially speed up some rules, at the
    /// cost of missing metadata for mismatches.
    pub fn into_rule_set(self, ignore_mismatch_metadata: bool) -> Result<RuleSet, Error> {
        Ok(RuleSet {
            name: self.name,
            preprocessors: self
                .preprocessors
                .iter()
                .map(|p| p.build())
                .collect::<Result<Vec<_>, _>>()?,
            rules: self
                .match_rules
                .iter()
                .map(|r| r.build(&self.string_match, ignore_mismatch_metadata))
                .collect::<Result<Vec<_>, _>>()?,
            string_match: self.string_match,
        })
    }
}

/// Configuration for a string group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringGroupConfig {
    name: String,
    rule_sets: Vec<RuleSetConfig>,
}

impl StringGroupConfig {
    /// Convert into StringGroup that can be used for matching
    ///
    /// `ignore_mismatch_metadata` flag can be enabled to potentially speed up some rules, at the
    /// cost of missing metadata for mismatches.
    pub fn into_string_group(self, ignore_mismatch_metadata: bool) -> Result<StringGroup, Error> {
        Ok(StringGroup::new(
            self.name,
            self.rule_sets
                .into_iter()
                .map(|s| s.into_rule_set(ignore_mismatch_metadata))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

/// Configuration for preprocessors
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "preprocessor_type", rename_all = "snake_case")]
pub enum PreprocessorConfig {
    /// Configuration for the split target preprocessor
    SplitTarget {
        /// If set to true, will ignore TLD part of the split string
        #[serde(default)]
        ignore_tld: bool,
    },
}

impl PreprocessorConfig {
    fn build(&self) -> Result<Preprocessor, Error> {
        match self {
            PreprocessorConfig::SplitTarget { ignore_tld } => {
                Ok(Preprocessor::SplitTarget(SplitTargetConfig {
                    ignore_tld: *ignore_tld,
                }))
            }
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
                "name": "Wikimedia",
                "rule_sets": [
                    {
                        "name": "wikipedia main brand name",
                        "string_match": "wikipedia",
                        "match_rules": [
                            {
                                "rule_type": "levenshtein",
                                "values": {
                                    "maximum_distance": 3
                                }
                            },
                            {
                                "rule_type": "jaro",
                                "values": {
                                    "match_percent_threshold": 85
                                }
                            }
                        ]
                    },
                    {
                        "name": "wikipedia learning brand name",
                        "string_match": "wikilearning",
                        "match_rules": [
                            {
                                "rule_type": "hamming",
                                "values": {
                                    "maximum_distance": 3
                                }
                            },
                            {
                                "rule_type": "jaro_winkler",
                                "values": {
                                    "match_percent_threshold": 85
                                }
                            }
                        ]
                    }
                ]
            }
            "#;

        let wikimedia_group: StringGroupConfig = serde_json::from_str(json).unwrap();

        assert_eq!("Wikimedia", &wikimedia_group.name);
        assert_eq!(2, wikimedia_group.rule_sets.len());

        let set_1 = &wikimedia_group.rule_sets[0];
        assert_eq!("wikipedia main brand name", &set_1.name);
        assert_eq!("wikipedia", &set_1.string_match);
        assert_eq!(2, set_1.match_rules.len());

        let RuleConfig::Levenshtein(set_1_rule_1) = &set_1.match_rules[0] else {
            panic!("Expected levenshtein rule");
        };
        assert_eq!(3, set_1_rule_1.maximum_distance);

        let RuleConfig::Jaro(set_1_rule_2) = &set_1.match_rules[1] else {
            panic!("Expected jaro rule");
        };
        assert_eq!(85.0, set_1_rule_2.match_percent_threshold);

        let set_2 = &wikimedia_group.rule_sets[1];
        assert_eq!("wikipedia learning brand name", &set_2.name);
        assert_eq!("wikilearning", &set_2.string_match);
        assert_eq!(2, set_2.match_rules.len());

        let RuleConfig::Hamming(set_2_rule_1) = &set_2.match_rules[0] else {
            panic!("Expected hamming rule");
        };
        assert_eq!(3, set_2_rule_1.maximum_distance);
        let RuleConfig::JaroWinkler(set_2_rule_2) = &set_2.match_rules[1] else {
            panic!("Expected jaro winkler rule");
        };
        assert_eq!(85.0, set_2_rule_2.match_percent_threshold);
    }

    #[test]
    fn test_parse_preprocessors() {
        let json = r#"
            {
                "name": "Wikimedia",
                "rule_sets": [
                    {
                        "name": "wikipedia main brand name",
                        "string_match": "wikipedia",
                        "preprocessors": [
                            {
                                "preprocessor_type": "split_target",
                                "ignore_tld": true
                            },
                            {
                                "preprocessor_type": "split_target"
                            }
                        ],
                        "match_rules": [
                            {
                                "rule_type": "levenshtein",
                                "values": {
                                    "maximum_distance": 3
                                }
                            },
                            {
                                "rule_type": "jaro",
                                "values": {
                                    "match_percent_threshold": 85
                                }
                            }
                        ]
                    }
                ]
            }
            "#;

        let wikimedia_group: StringGroupConfig = serde_json::from_str(json).unwrap();

        assert_eq!("Wikimedia", &wikimedia_group.name);

        let set_1 = &wikimedia_group.rule_sets[0];
        assert_eq!("wikipedia main brand name", &set_1.name);
        assert_eq!("wikipedia", &set_1.string_match);
        assert_eq!(2, set_1.preprocessors.len());

        let PreprocessorConfig::SplitTarget { ignore_tld } = &set_1.preprocessors[0];
        assert!(ignore_tld);

        let PreprocessorConfig::SplitTarget { ignore_tld } = &set_1.preprocessors[1];
        assert!(!ignore_tld);
    }
}
