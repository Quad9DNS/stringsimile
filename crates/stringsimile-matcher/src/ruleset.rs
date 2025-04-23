//! Group of related rules

use serde_json::{Map, Value};
use tracing::{debug, error, warn};

use crate::rule::GenericMatcherRule;

/// Rule set
pub struct RuleSet {
    /// Name of the rule set
    pub name: String,
    /// String to match agains
    pub string_match: String,
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
    pub fn generate_matches(&self, name: &str) -> Option<Vec<Map<String, Value>>> {
        debug!(
            message = "Generating matches for rule set: {}",
            self.name,
            input = name,
            target = self.string_match
        );
        let mut matches: Vec<Map<String, Value>> = Vec::default();

        for rule in &self.rules {
            match rule.match_rule_generic(name, &self.string_match) {
                Ok(Some(metadata)) => {
                    matches.push(metadata);
                }
                Ok(None) => {
                    debug!("No match");
                    continue;
                }
                Err(err) => {
                    error!(message = "Matcher failed", error = ?err);
                }
            }
        }

        if !matches.is_empty() {
            Some(matches)
        } else {
            None
        }
    }
}

impl StringGroup {
    /// Matches the value to this string group and generates matches with metadata
    pub fn generate_matches(&self, object: &Value) -> Option<Map<String, Value>> {
        debug!(message = "Generating matches for string group: {}", self.name, input = ?object);
        let mut matches: Map<String, Value> = Map::default();

        if let Value::Object(map) = object {
            // TODO: make key configurable
            let field = map.get("domain_name");
            match field {
                Some(Value::String(name)) => {
                    for rule_set in &self.rule_sets {
                        if let Some(rule_set_matches) = rule_set.generate_matches(name) {
                            matches.insert(
                                rule_set.name.clone(),
                                Value::Array(
                                    rule_set_matches.into_iter().map(Value::Object).collect(),
                                ),
                            );
                        }
                    }
                }
                Some(other) => {
                    warn!("Expected string value in key field, but found: {other}");
                }
                None => {
                    warn!("Specified key field (domain_name) not found in input.");
                }
            }
        } else {
            warn!("Expected JSON object, but found: {object}");
        };

        if !matches.is_empty() {
            Some(matches)
        } else {
            None
        }
    }
}
