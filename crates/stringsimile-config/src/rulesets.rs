//! Rulesets configuration
use serde::{Deserialize, Serialize};

use crate::rules::RuleConfig;

/// Configuration for a rule set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    name: String,
    string_match: String,
    match_rules: Vec<RuleConfig>,
}

/// Configuration for a string group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringGroup {
    name: String,
    rule_sets: Vec<RuleSet>,
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
                ]
            }
            "#;

        let wikimedia_group: StringGroup = serde_json::from_str(json).unwrap();

        assert_eq!("Wikimedia", &wikimedia_group.name);
        assert_eq!(2, wikimedia_group.rule_sets.len());

        let set_1 = &wikimedia_group.rule_sets[0];
        assert_eq!("wikipedia main brand name", &set_1.name);
        assert_eq!("wikipedia", &set_1.string_match);
        assert_eq!(2, set_1.match_rules.len());

        let RuleConfig::Levenshtein(set_1_rule_1) = &set_1.match_rules[0] else {
            panic!("Expected levenshtein rule");
        };
        assert_eq!(3, set_1_rule_1.maximum_distance.get());

        // let set_2 = wikimedia_group.rule_sets[1];
    }
}
