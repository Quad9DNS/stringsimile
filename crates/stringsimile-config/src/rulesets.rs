//! Rulesets configuration

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use stringsimile_matcher::{
    Error,
    preprocessors::{
        ExclusionSetConfig, ExclusionSetSource, Preprocessor, PunycodeConfig, SplitTargetConfig,
    },
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

/// Configuration for exclusion set preprocessors
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "exclusion_set_source", rename_all = "snake_case")]
pub enum ExclusionSetPreprocessorConfig {
    /// Configuration for the file exclusion set preprocessor source
    File {
        /// Path to the file containing exclusion set - one entry per line
        path: PathBuf,
    },
    /// Configuration for the static list exclusion set preprocessor source
    List {
        /// Static list of exclusion set entries
        list: Vec<String>,
    },
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
    /// Configuration for the exclusion set preprocessor
    ExclusionSet {
        /// Specific exclusion set type config
        #[serde(flatten)]
        inner: ExclusionSetPreprocessorConfig,
        /// If set to true, values will be treated as regex patterns, rather than exact values
        #[serde(default)]
        regex: bool,
    },
    /// Configuration for the punycode preprocessor
    Punycode {
        /// If set to true, non-ascii strings will be punycode encoded
        #[serde(default = "default_true")]
        encode: bool,
        /// If set to true, punycode encoded strings will be decoded
        #[serde(default = "default_true")]
        decode: bool,
        /// If set to true, the original input value will be kept alongside the encoded/decoded
        /// value
        #[serde(default)]
        keep_both: bool,
    },
}

const fn default_true() -> bool {
    true
}

impl PreprocessorConfig {
    fn build(&self) -> Result<Preprocessor, Error> {
        match self {
            PreprocessorConfig::SplitTarget { ignore_tld } => {
                Ok(Preprocessor::SplitTarget(SplitTargetConfig {
                    ignore_tld: *ignore_tld,
                }))
            }
            PreprocessorConfig::ExclusionSet { inner, regex } => {
                Ok(Preprocessor::ExclusionSet(ExclusionSetConfig {
                    source: match inner {
                        ExclusionSetPreprocessorConfig::File { path } => {
                            ExclusionSetSource::File(path.clone())
                        }
                        ExclusionSetPreprocessorConfig::List { list } => {
                            ExclusionSetSource::Static(list.clone())
                        }
                    },
                    regex: *regex,
                }))
            }
            PreprocessorConfig::Punycode {
                encode,
                decode,
                keep_both,
            } => Ok(Preprocessor::Punycode(PunycodeConfig {
                encode: *encode,
                decode: *decode,
                keep_both: *keep_both,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::RuleTypeConfig;

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
                                "exit_on_match": true,
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

        let RuleConfig {
            common,
            rule_type: RuleTypeConfig::Levenshtein(set_1_rule_1),
        } = &set_1.match_rules[0]
        else {
            panic!("Expected levenshtein rule");
        };
        assert!(common.exit_on_match);
        assert_eq!(3, set_1_rule_1.maximum_distance);

        let RuleConfig {
            common: _,
            rule_type: RuleTypeConfig::Jaro(set_1_rule_2),
        } = &set_1.match_rules[1]
        else {
            panic!("Expected jaro rule");
        };
        assert_eq!(85.0, set_1_rule_2.match_percent_threshold);

        let set_2 = &wikimedia_group.rule_sets[1];
        assert_eq!("wikipedia learning brand name", &set_2.name);
        assert_eq!("wikilearning", &set_2.string_match);
        assert_eq!(2, set_2.match_rules.len());

        let RuleConfig {
            common: _,
            rule_type: RuleTypeConfig::Hamming(set_2_rule_1),
        } = &set_2.match_rules[0]
        else {
            panic!("Expected hamming rule");
        };
        assert_eq!(3, set_2_rule_1.maximum_distance);
        let RuleConfig {
            common: _,
            rule_type: RuleTypeConfig::JaroWinkler(set_2_rule_2),
        } = &set_2.match_rules[1]
        else {
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
                            },
                            {
                                "preprocessor_type": "exclusion_set",
                                "exclusion_set_source": "list",
                                "list": [ "www" ]
                            },
                            {
                                "preprocessor_type": "punycode",
                                "encode": true,
                                "decode": false
                            },
                            {
                                "preprocessor_type": "punycode"
                            },
                            {
                                "preprocessor_type": "punycode",
                                "keep_both": true
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
        assert_eq!(6, set_1.preprocessors.len());

        let PreprocessorConfig::SplitTarget { ignore_tld } = &set_1.preprocessors[0] else {
            panic!("Expected split target preprocessor");
        };
        assert!(ignore_tld);

        let PreprocessorConfig::SplitTarget { ignore_tld } = &set_1.preprocessors[1] else {
            panic!("Expected split target preprocessor");
        };
        assert!(!ignore_tld);

        let PreprocessorConfig::ExclusionSet {
            inner: ExclusionSetPreprocessorConfig::List { list },
            regex: false,
        } = &set_1.preprocessors[2]
        else {
            panic!("Expected list exclusion set preprocessor");
        };
        assert_eq!(list, &vec!["www".to_string()]);

        let PreprocessorConfig::Punycode {
            encode,
            decode,
            keep_both,
        } = &set_1.preprocessors[3]
        else {
            panic!("Expected punycode set preprocessor");
        };
        assert!(encode);
        assert!(!decode);
        assert!(!keep_both);

        let PreprocessorConfig::Punycode {
            encode,
            decode,
            keep_both,
        } = &set_1.preprocessors[4]
        else {
            panic!("Expected punycode set preprocessor");
        };
        assert!(encode);
        assert!(decode);
        assert!(!keep_both);

        let PreprocessorConfig::Punycode {
            encode,
            decode,
            keep_both,
        } = &set_1.preprocessors[5]
        else {
            panic!("Expected punycode set preprocessor");
        };
        assert!(encode);
        assert!(decode);
        assert!(keep_both);
    }
}
