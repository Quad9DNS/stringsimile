//! Group of related rules

use hashbrown::HashMap;
use std::{borrow::Cow, collections::BTreeMap, ops::Deref};

use metrics::{Counter, Unit, counter, describe_counter};
use serde_json::{Map, Value};
use tracing::{debug, trace_span, warn};

use crate::{
    GenericMatchResult,
    preprocessors::{BoxedTargetWithMetadataIter, ExclusionSetContext, Preprocessor},
    rule::{EstimationResult, GenericMatcherRule},
};

/// Rule set
pub struct RuleSet {
    /// Name of the rule set
    pub name: String,
    /// String to match against
    pub string_match: String,
    /// Preprocessors to apply to input strings before passing them to rules
    pub preprocessors: Vec<Preprocessor>,
    /// Rules to apply to this match
    pub rules: Vec<(CommonRuleConfig, Box<dyn GenericMatcherRule>)>,
}

impl Clone for RuleSet {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            string_match: self.string_match.clone(),
            preprocessors: self.preprocessors.clone(),
            rules: self
                .rules
                .iter()
                .map(|(c, r)| (c.clone(), r.clone_dyn()))
                .collect(),
        }
    }
}

/// Common configuration for rules
#[derive(Clone, Default)]
pub struct CommonRuleConfig {
    /// Whether match on this rule should result in early exit from ruleset
    pub exit_on_match: bool,
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
        describe_counter!(
            "rule_matches",
            Unit::Count,
            "Number of matches found by this rule"
        );
        describe_counter!(
            "rule_misses",
            Unit::Count,
            "Number of mismatches found by this rule"
        );
        describe_counter!(
            "rule_errors",
            Unit::Count,
            "Number of errors encountered by this rule"
        );
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

/// Metrics related to a single exclusion set preprocessor
#[derive(Clone)]
pub struct ExclusionSetMetrics {
    /// Number of items excluded by this set
    pub exclusions: Counter,
}

impl ExclusionSetMetrics {
    /// Creates a new metrics object based on provided names
    pub fn new(string_group: &str, rule_set: &str, preprocessor_index: usize) -> Self {
        describe_counter!(
            "exclusion_set_exclusions",
            Unit::Count,
            "Number of input objects (or parts) excluded by this set"
        );
        Self {
            exclusions: counter!("exclusion_set_exclusions",
                "string_group" => string_group.to_string(),
                "rule_set" => rule_set.to_string(),
                "preprocessor_index" => preprocessor_index.to_string(),
            ),
        }
    }
}

/// Context of a ruleset (external state)
pub struct RulesetContext {
    metrics: Vec<RuleMetrics>,
    preprocessors: Vec<PreprocessorContext>,
}

/// Context of a preprocessor (external state)
pub enum PreprocessorContext {
    /// Empty context for preprocessors that don't need it
    Empty,
    /// Context for exclusion set preprocessor
    ExclusionSet {
        /// Metrics for this exclusion set
        metrics: ExclusionSetMetrics,
        /// Specific context for this exclusion set
        context: ExclusionSetContext,
    },
}

/// Context of a string group (external state)
pub struct StringGroupContext {
    rulesets: HashMap<String, RulesetContext>,
}

impl StringGroupContext {
    /// Creates a new string group context, with configured context for each rule set
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
                            .map(|(_, rule)| RuleMetrics::new(name, &rs.name, rule.name()))
                            .collect(),
                        preprocessors: rs
                            .preprocessors
                            .iter()
                            .enumerate()
                            .map(|(index, p)| p.build_context(name, &rs.name, index))
                            .collect(),
                    },
                )
            })
            .collect();
        Self { rulesets }
    }

    /// Preloads data needed for the context that needs to be loaded asynchronously
    pub async fn preload_context(&mut self, rulesets: &Vec<RuleSet>) {
        for rs in rulesets {
            if let Some(rs_ctx) = self.rulesets.get_mut(&rs.name) {
                for (ctx, p) in rs_ctx.preprocessors.iter_mut().zip(rs.preprocessors.iter()) {
                    p.preload_context(ctx).await;
                }
            }
        }
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
        let span = trace_span!("ruleset", input = name, ruleset = self.name);
        let _entered = span.enter();
        debug!(
            message = format!("Generating matches for rule set: {}", self.name),
            input = name,
            target = self.string_match
        );
        let mut matches: Vec<GenericMatchResult> = Vec::default();

        let input: BoxedTargetWithMetadataIter<'_> =
            Box::new([(Cow::Borrowed(name), Cow::Owned(Map::default()))].into_iter());

        let input = self
            .preprocessors
            .iter()
            .enumerate()
            .fold(input, |acc, (index, p)| {
                p.process(acc, &context.preprocessors[index])
            });

        let mut matched_rules: Vec<bool> = self.rules.iter().map(|_| false).collect();

        for it in input {
            for (index, (config, rule)) in self.rules.iter().enumerate() {
                let span = trace_span!(
                    "ruleset_rule",
                    input = name,
                    ruleset = self.name,
                    index = index
                );
                let _entered = span.enter();
                let rule_metrics = context
                    .metrics
                    .get(index)
                    .expect("Missing metrics for rule");

                let matched = self.generate_match(
                    &mut matches,
                    rule.deref(),
                    &it,
                    rule_metrics,
                    full_metadata_for_all,
                );

                if matched {
                    matched_rules[index] = true;
                }

                if matched && config.exit_on_match {
                    matches
                        .last_mut()
                        .expect("Last match not found after generating it")
                        .metadata
                        .insert("early_match_exit".to_string(), true.into());
                    break;
                }
            }
        }

        for (index, matched) in matched_rules.iter().enumerate() {
            let rule_metrics = context
                .metrics
                .get(index)
                .expect("Missing metrics for rule");
            if *matched {
                rule_metrics.matches.increment(1);
            } else {
                rule_metrics.misses.increment(1);
            }
        }

        matches
    }

    fn generate_match(
        &self,
        matches: &mut Vec<GenericMatchResult>,
        rule: &dyn GenericMatcherRule,
        (part, extra_metadata): &(Cow<'_, str>, Cow<'_, Map<String, Value>>),
        rule_metrics: &RuleMetrics,
        full_metadata_for_all: bool,
    ) -> bool {
        match rule.match_rule_generic(part, &self.string_match, full_metadata_for_all) {
            Ok(mut result) => {
                let matched = result.matched;
                if result.matched || full_metadata_for_all {
                    extra_metadata.iter().for_each(|(k, v)| {
                        result.metadata.insert(k.to_string(), v.clone());
                    });
                    matches.push(result.into_full_metadata());
                }
                matched
            }
            Err(err) => {
                rule_metrics.errors.increment(1);
                warn!(message = "Matcher failed", error = ?err);
                false
            }
        }
    }

    fn estimate_cost(&self) -> EstimationResult {
        let mut total = EstimationResult::zero();
        let mut min = None;
        for (common, rule) in &self.rules {
            total += rule.estimate_generic(&self.string_match);
            if common.exit_on_match && min.is_none() {
                min = total.min;
            }
        }
        if min.is_some() {
            total.min = min;
        }
        total
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
        let span = trace_span!("string group", input = input, group = self.name);
        let _entered = span.enter();
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
            if !rule_set_matches.is_empty() || full_metadata_for_all {
                matches.insert(rule_set.name.clone(), rule_set_matches);
            }
        }

        matches
    }

    /// Estimates the resource cost of the string group
    pub fn estimate_cost(&self) -> EstimationResult {
        let mut total = EstimationResult::zero();
        for rule_set in &self.rule_sets {
            total += rule_set.estimate_cost();
        }
        total
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
    use std::fs;

    use tempfile::NamedTempFile;

    use crate::{
        preprocessors::{ExclusionSetConfig, ExclusionSetSource, SplitTargetConfig},
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
                    (
                        Default::default(),
                        Box::new(BitflipRule::new_dns("www.test.com", true).into_generic_matcher()),
                    ),
                    (
                        Default::default(),
                        Box::new(
                            LevenshteinRule {
                                maximum_distance: 3,
                                ignore_mismatch_metadata: false,
                            }
                            .into_generic_matcher(),
                        ),
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
                    (
                        Default::default(),
                        Box::new(BitflipRule::new_dns("test", true).into_generic_matcher()),
                    ),
                    (
                        Default::default(),
                        Box::new(
                            LevenshteinRule {
                                maximum_distance: 3,
                                ignore_mismatch_metadata: false,
                            }
                            .into_generic_matcher(),
                        ),
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

    #[tokio::test]
    async fn basic_example_exclusion_set_preprocessing() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "is\ncom\n").unwrap();
        let group = StringGroup::new(
            "test".to_string(),
            vec![
                RuleSet {
                    name: "test_ruleset".to_string(),
                    string_match: "test".to_string(),
                    preprocessors: vec![
                        Preprocessor::SplitTarget(SplitTargetConfig { ignore_tld: false }),
                        Preprocessor::ExclusionSet(ExclusionSetConfig {
                            source: ExclusionSetSource::File(file.path().to_path_buf()),
                            regex: false,
                        }),
                    ],
                    rules: vec![
                        (
                            Default::default(),
                            Box::new(BitflipRule::new_dns("test", true).into_generic_matcher()),
                        ),
                        (
                            Default::default(),
                            Box::new(
                                LevenshteinRule {
                                    maximum_distance: 3,
                                    ignore_mismatch_metadata: false,
                                }
                                .into_generic_matcher(),
                            ),
                        ),
                    ],
                },
                RuleSet {
                    name: "test_ruleset_2".to_string(),
                    string_match: "test".to_string(),
                    preprocessors: vec![
                        Preprocessor::ExclusionSet(ExclusionSetConfig {
                            source: ExclusionSetSource::File(file.path().to_path_buf()),
                            regex: false,
                        }),
                        Preprocessor::SplitTarget(SplitTargetConfig { ignore_tld: true }),
                    ],
                    rules: vec![(
                        Default::default(),
                        Box::new(BitflipRule::new_dns("fdsfdsfs", true).into_generic_matcher()),
                    )],
                },
            ],
        );
        let mut context = StringGroupContext::new(&group);
        context.preload_context(&group.rule_sets).await;

        let matches = group.generate_matches("www.tset.com", &context, true);
        let result = matches.get("test_ruleset").unwrap();

        println!(
            "{:?}",
            result
                .iter()
                .map(|r| r.metadata.clone())
                .collect::<Vec<_>>()
        );
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
