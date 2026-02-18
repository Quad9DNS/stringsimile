//! Rulesets configuration

use serde::{Deserialize, Serialize};
use stringsimile_matcher::{
    Error,
    ruleset::{RuleSet, StringGroup},
};

use crate::rules::RuleConfig;

/// Configuration for a rule set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSetConfig {
    name: String,
    string_match: String,
    // TODO: extract this into something more generic, like a pre-processor
    #[serde(default)]
    split_target: bool,
    #[serde(default)]
    ignore_tld: bool,
    match_rules: Vec<RuleConfig>,
}

impl RuleSetConfig {
    /// Convert into RuleSet that can be used for matching
    pub fn into_rule_set(self) -> Result<RuleSet, Error> {
        Ok(RuleSet {
            name: self.name,
            split_target: self.split_target,
            ignore_tld: self.ignore_tld,
            rules: self
                .match_rules
                .iter()
                .map(|c| c.build(&self.string_match))
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
    pub fn into_string_group(self) -> Result<StringGroup, Error> {
        Ok(StringGroup::new(
            self.name,
            self.rule_sets
                .into_iter()
                .map(RuleSetConfig::into_rule_set)
                .collect::<Result<Vec<_>, _>>()?,
        ))
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
}
