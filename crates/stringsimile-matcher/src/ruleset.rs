//! Group of related rules

use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
};

use metrics::{Counter, counter};
use serde_json::{Map, Value};
use tracing::{debug, trace_span, warn};

use crate::{GenericMatchResult, rule::GenericMatcherRule};

/// Rule set
pub struct RuleSet {
    /// Name of the rule set
    pub name: String,
    /// String to match agains
    pub string_match: String,
    // TODO: extract this into something more generic, like a pre-processor
    /// If set to true, will split the string into domain parts before processing
    pub split_target: bool,
    /// If set to true, will ignore TLD part of the split string
    pub ignore_tld: bool,
    /// Rules to apply to this match
    pub rules: Vec<Box<dyn GenericMatcherRule>>,
}

impl Clone for RuleSet {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            string_match: self.string_match.clone(),
            split_target: self.split_target,
            ignore_tld: self.ignore_tld,
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
    metrics: HashMap<String, HashMap<String, RuleMetrics>>,
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

impl RuleSet {
    /// Matches the value to this rule set and generates matches with metadata
    fn generate_matches(
        &self,
        name: &str,
        metrics: &HashMap<String, RuleMetrics>,
        full_metadata_for_all: bool,
    ) -> Vec<GenericMatchResult> {
        let _ = trace_span!("ruleset", input = name, ruleset = self.name).enter();
        debug!(
            message = format!("Generating matches for rule set: {}", self.name),
            input = name,
            target = self.string_match
        );
        let mut matches: Vec<GenericMatchResult> = Vec::default();

        if self.split_target {
            for rule in &self.rules {
                let rule_metrics = metrics.get(rule.name()).expect("Missing metrics for rule");
                for it in name
                    .split('.')
                    .rev()
                    .skip(if self.ignore_tld { 1 } else { 0 })
                    .enumerate()
                {
                    self.generate_match(
                        &mut matches,
                        rule.deref(),
                        it,
                        rule_metrics,
                        full_metadata_for_all,
                    );
                }
            }
        } else {
            for rule in &self.rules {
                let rule_metrics = metrics.get(rule.name()).expect("Missing metrics for rule");
                self.generate_match(
                    &mut matches,
                    rule.deref(),
                    (0, name),
                    rule_metrics,
                    full_metadata_for_all,
                );
            }
        };

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
                    if self.split_target {
                        result.metadata.insert(
                            "split_string".to_string(),
                            Value::String(part.to_string().clone()),
                        );
                        result
                            .metadata
                            .insert("split_position".to_string(), Value::Number(index.into()));
                    }
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
        let metrics = rule_sets
            .iter()
            .map(|rs| {
                (
                    rs.name.clone(),
                    rs.rules
                        .iter()
                        .map(|rule| {
                            (
                                rule.name().to_string(),
                                RuleMetrics::new(&name, &rs.name, rule.name()),
                            )
                        })
                        .collect(),
                )
            })
            .collect();
        Self {
            name: name.clone(),
            rule_sets,
            metrics,
        }
    }

    /// Matches the value to this string group and generates matches with metadata
    pub fn generate_matches(
        &self,
        input: &str,
        full_metadata_for_all: bool,
    ) -> BTreeMap<String, Vec<GenericMatchResult>> {
        let _ = trace_span!("string group", input = input, group = self.name).enter();
        debug!(message = format!("Generating matches for string group: {}", self.name), input = ?input);
        let mut matches: BTreeMap<String, Vec<GenericMatchResult>> = BTreeMap::default();

        for rule_set in &self.rule_sets {
            let rule_set_matches = rule_set.generate_matches(
                input,
                self.metrics
                    .get(&rule_set.name)
                    .expect("Missing rule set metrics"),
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
