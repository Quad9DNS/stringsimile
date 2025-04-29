//! Group of related rules

use std::collections::BTreeMap;

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
    pub rules: Vec<Box<dyn GenericMatcherRule + 'static + Send>>,
}

/// String group
pub struct StringGroup {
    /// Name of the group
    pub name: String,
    /// Rule sets that are in this group
    pub rule_sets: Vec<RuleSet>,
}

impl RuleSet {
    /// Matches the value to this rule set and generates matches with metadata
    pub fn generate_matches(&self, name: &str) -> Vec<GenericMatchResult> {
        let _ = trace_span!("ruleset", input = name, ruleset = self.name).enter();
        debug!(
            message = format!("Generating matches for rule set: {}", self.name),
            input = name,
            target = self.string_match
        );
        let mut matches: Vec<GenericMatchResult> = Vec::default();

        let mut parts = Vec::default();
        if self.split_target {
            parts.extend(name.split(".").map(|s| s.to_string()));
            if self.ignore_tld {
                parts.remove(parts.len() - 1);
            }
        } else {
            parts.push(name.to_string());
        }

        for rule in &self.rules {
            for (index, part) in parts.iter().enumerate() {
                match rule.match_rule_generic(part, &self.string_match) {
                    Ok(mut result) => {
                        if self.split_target {
                            result
                                .metadata
                                .insert("split_string".to_string(), Value::String(part.clone()));
                            result
                                .metadata
                                .insert("split_position".to_string(), Value::Number(index.into()));
                        }
                        matches.push(result.into_full_metadata());
                    }
                    Err(err) => {
                        warn!(message = "Matcher failed", error = ?err);
                    }
                }
            }
        }

        matches
    }
}

impl StringGroup {
    /// Matches the value to this string group and generates matches with metadata
    pub fn generate_matches(&self, input: &str) -> BTreeMap<String, Vec<GenericMatchResult>> {
        let _ = trace_span!("string group", input = input, group = self.name).enter();
        debug!(message = format!("Generating matches for string group: {}", self.name), input = ?input);
        let mut matches: BTreeMap<String, Vec<GenericMatchResult>> = BTreeMap::default();

        for rule_set in &self.rule_sets {
            let rule_set_matches = rule_set.generate_matches(input);
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
