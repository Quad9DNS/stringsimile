//! Group of related rules

use hashbrown::HashMap;
use std::{collections::BTreeMap, ops::Deref};

use metrics::{Counter, counter};
use serde_json::{Map, Value};
use tracing::{debug, trace_span, warn};

use crate::{GenericMatchResult, preprocessors::Preprocessor, rule::GenericMatcherRule};

/// Rule set
pub struct RuleSet {
    /// Name of the rule set
    pub name: String,
    /// String to match against
    pub string_match: String,
    /// Preprocessors to apply to input strings before passing them to rules
    pub preprocessors: Vec<Preprocessor>,
    /// Rules to apply to this match
    pub rules: Vec<Box<dyn GenericMatcherRule>>,
}

impl Clone for RuleSet {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            string_match: self.string_match.clone(),
            preprocessors: self.preprocessors.clone(),
            rules: self.rules.iter().map(|r| r.clone_dyn()).collect(),
        }
    }
}

/// String group
#[derive(Clone)]
pub struct StringGroup {
    /// Name of the group
    pub name: String,
    /// Rule sets that are in this group
    pub rule_sets: Vec<RuleSet>,
}

#[derive(Clone)]
struct RuleMetrics {
    matches: Counter,
    misses: Counter,
    errors: Counter,
}

impl RuleMetrics {
    fn new(string_group: &str, rule_set: &str, rule: &str) -> Self {
        Self {
            matches: counter!("rule_matches",
                "string_group" => string_group.to_string(),
                "rule_set" => rule_set.to_string(),
                "rule" => rule.to_string(),
            ),
            misses: counter!("rule_misses",
                "string_group" => string_group.to_string(),
                "rule_set" => rule_set.to_string(),
                "rule" => rule.to_string(),
            ),
            errors: counter!("rule_errors",
                "string_group" => string_group.to_string(),
                "rule_set" => rule_set.to_string(),
                "rule" => rule.to_string(),
            ),
        }
    }
}

/// Context of a ruleset (external state)
pub struct RulesetContext {
    metrics: HashMap<String, RuleMetrics>,
}

/// Context of a string group (external state)
pub struct StringGroupContext {
    rulesets: HashMap<String, RulesetContext>,
}

impl StringGroupContext {
    /// Creates a new string group context, with configured contextx for each rule set
    pub fn new(string_group: &StringGroup) -> Self {
        let name: &str = &string_group.name;
        let rule_sets: &[RuleSet] = &string_group.rule_sets;
        let rulesets = rule_sets
            .iter()
            .map(|rs| {
                (
                    rs.name.clone(),
                    RulesetContext {
                        metrics: rs
                            .rules
                            .iter()
                            .map(|rule| {
                                (
                                    rule.name().to_string(),
                                    RuleMetrics::new(name, &rs.name, rule.name()),
                                )
                            })
                            .collect(),
                    },
                )
            })
            .collect();
        Self { rulesets }
    }
}

impl RuleSet {
    /// Matches the value to this rule set and generates matches with metadata
    fn generate_matches(
        &self,
        name: &str,
        context: &RulesetContext,
        full_metadata_for_all: bool,
    ) -> Vec<GenericMatchResult> {
        let _ = trace_span!("ruleset", input = name, ruleset = self.name).enter();
        debug!(
            message = format!("Generating matches for rule set: {}", self.name),
            input = name,
            target = self.string_match
        );
        let mut matches: Vec<GenericMatchResult> = Vec::default();

        let input: Box<dyn Iterator<Item = &str>> = Box::new([name].into_iter());

        let input = self
            .preprocessors
            .iter()
            .fold(input, |acc, p| p.process(acc));

        for it in input.enumerate() {
            for rule in &self.rules {
                let rule_metrics = context
                    .metrics
                    .get(rule.name())
                    .expect("Missing metrics for rule");

                self.generate_match(
                    &mut matches,
                    rule.deref(),
                    it,
                    rule_metrics,
                    full_metadata_for_all,
                );
            }
        }

        matches
    }

    fn generate_match(
        &self,
        matches: &mut Vec<GenericMatchResult>,
        rule: &dyn GenericMatcherRule,
        (index, part): (usize, &str),
        rule_metrics: &RuleMetrics,
        full_metadata_for_all: bool,
    ) {
        match rule.match_rule_generic(part, &self.string_match, full_metadata_for_all) {
            Ok(mut result) => {
                if result.matched {
                    rule_metrics.matches.increment(1);
                } else {
                    rule_metrics.misses.increment(1);
                }
                if result.matched || full_metadata_for_all {
                    self.preprocessors
                        .iter()
                        .for_each(|p| p.add_metadata(&mut result.metadata, (index, part)));
                    matches.push(result.into_full_metadata());
                } else {
                    matches.push(result);
                }
            }
            Err(err) => {
                rule_metrics.errors.increment(1);
                warn!(message = "Matcher failed", error = ?err);
            }
        }
    }
}

impl StringGroup {
    /// Creates a new string group with given name and rule sets
    pub fn new(name: String, rule_sets: Vec<RuleSet>) -> Self {
        Self {
            name: name.clone(),
            rule_sets,
        }
    }

    /// Matches the value to this string group and generates matches with metadata
    pub fn generate_matches(
        &self,
        input: &str,
        context: &StringGroupContext,
        full_metadata_for_all: bool,
    ) -> BTreeMap<String, Vec<GenericMatchResult>> {
        let _ = trace_span!("string group", input = input, group = self.name).enter();
        debug!(message = format!("Generating matches for string group: {}", self.name), input = ?input);
        let mut matches: BTreeMap<String, Vec<GenericMatchResult>> = BTreeMap::default();

        for rule_set in &self.rule_sets {
            let rule_set_matches = rule_set.generate_matches(
                input,
                context
                    .rulesets
                    .get(&rule_set.name)
                    .expect("Missing rule set context"),
                full_metadata_for_all,
            );
            matches.insert(rule_set.name.clone(), rule_set_matches);
        }

        matches
    }
}

/// Trait for results of StringGroup
pub trait StringGroupMatchResult {
    /// Returns true if there were any matches in this group
    fn has_matches(&self) -> bool;

    /// Converts this match result into JSON value
    fn to_json(self) -> Value;
}

impl StringGroupMatchResult for BTreeMap<String, Vec<GenericMatchResult>> {
    fn has_matches(&self) -> bool {
        self.iter().flat_map(|(_name, res)| res).any(|m| m.matched)
    }

    fn to_json(self) -> Value {
        let mut map: Map<String, Value> = Map::default();
        map.insert(
            "rule_sets".to_string(),
            Value::Object(Map::from_iter(self.into_iter().map(
                |(rule_set_name, results)| {
                    (
                        rule_set_name,
                        Value::Array(
                            results
                                .into_iter()
                                .map(|r| Value::Object(r.metadata))
                                .collect(),
                        ),
                    )
                },
            ))),
        );
        Value::Object(map)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocessors::SplitTargetConfig,
        rule::IntoGenericMatcherRule,
        rules::{bitflip::BitflipRule, levenshtein::LevenshteinRule},
    };

    use super::*;

    #[test]
    fn basic_example_no_preprocessing() {
        let group = StringGroup::new(
            "test".to_string(),
            vec![RuleSet {
                name: "test_ruleset".to_string(),
                string_match: "www.test.com".to_string(),
                preprocessors: Vec::default(),
                rules: vec![
                    Box::new(BitflipRule::new_dns("www.test.com", true).into_generic_matcher()),
                    Box::new(
                        LevenshteinRule {
                            maximum_distance: 3,
                            ignore_mismatch_metadata: false,
                        }
                        .into_generic_matcher(),
                    ),
                ],
            }],
        );

        let matches =
            group.generate_matches("wwwntest.com", &StringGroupContext::new(&group), false);
        let result = matches.get("test_ruleset").unwrap();

        assert_eq!(result.len(), 2);
        assert!(result[0].matched);
        assert_eq!(result[0].rule_type, "bitflip");
        assert!(result[1].matched);
        assert_eq!(result[1].rule_type, "levenshtein");
    }

    #[test]
    fn basic_example_split_target_preprocessing() {
        let group = StringGroup::new(
            "test".to_string(),
            vec![RuleSet {
                name: "test_ruleset".to_string(),
                string_match: "test".to_string(),
                preprocessors: vec![Preprocessor::SplitTarget(SplitTargetConfig {
                    ignore_tld: true,
                })],
                rules: vec![
                    Box::new(BitflipRule::new_dns("test", true).into_generic_matcher()),
                    Box::new(
                        LevenshteinRule {
                            maximum_distance: 3,
                            ignore_mismatch_metadata: false,
                        }
                        .into_generic_matcher(),
                    ),
                ],
            }],
        );

        let matches =
            group.generate_matches("www.tset.com", &StringGroupContext::new(&group), true);
        let result = matches.get("test_ruleset").unwrap();

        assert_eq!(result.len(), 4);
        assert!(!result[0].matched);
        assert_eq!(result[0].rule_type, "bitflip");
        assert_eq!(
            result[0]
                .metadata
                .get("split_string")
                .unwrap()
                .as_str()
                .unwrap(),
            "www"
        );
        assert_eq!(
            result[0]
                .metadata
                .get("split_position")
                .unwrap()
                .as_u64()
                .unwrap(),
            0
        );
        assert!(!result[1].matched);
        assert_eq!(result[1].rule_type, "levenshtein");
        assert_eq!(
            result[1]
                .metadata
                .get("split_string")
                .unwrap()
                .as_str()
                .unwrap(),
            "www"
        );
        assert_eq!(
            result[1]
                .metadata
                .get("split_position")
                .unwrap()
                .as_u64()
                .unwrap(),
            0
        );
        assert!(!result[2].matched);
        assert_eq!(result[2].rule_type, "bitflip");
        assert_eq!(
            result[2]
                .metadata
                .get("split_string")
                .unwrap()
                .as_str()
                .unwrap(),
            "tset"
        );
        assert_eq!(
            result[2]
                .metadata
                .get("split_position")
                .unwrap()
                .as_u64()
                .unwrap(),
            1
        );
        assert!(result[3].matched);
        assert_eq!(result[3].rule_type, "levenshtein");
        assert_eq!(
            result[3]
                .metadata
                .get("split_string")
                .unwrap()
                .as_str()
                .unwrap(),
            "tset"
        );
        assert_eq!(
            result[3]
                .metadata
                .get("split_position")
                .unwrap()
                .as_u64()
                .unwrap(),
            1
        );
    }
}
