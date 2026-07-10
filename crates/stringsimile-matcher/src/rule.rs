//! stringsimile matcher rules

use std::fmt::Debug;

use serde::Serialize;
use serde_json::Map;
use tracing::{debug, trace_span};

use crate::{GenericMatcherResult, MatchResult, MatcherResult};

/// Helper functions for rule matchers results
pub trait MatcherResultExt<T: Debug, E: Debug> {
    /// Creates a new error
    fn new_error(err: E) -> Self;

    /// Returns true only if result is successful and a match
    fn is_match(&self) -> bool;
    /// Returns internal metadata of the match result.
    ///
    /// Panics if the result is not successful or not a match
    #[must_use]
    fn into_metadata(self) -> T;
}

/// Helper functions for creating rule matchers results
pub trait MatcherResultRuleMetadataExt<T: RuleMetadata, E> {
    /// Creates a new successful match result
    fn new_match(metadata: T) -> Self;
    /// Creates a new successful no match result
    fn new_no_match(metadata: T) -> Self;
}

/// Helper functions for creating optimized rule matchers results, when metadata is not required
pub trait MatcherResultRuleOptionMetadataExt<T: RuleMetadata, E> {
    /// Creates a new successful no match result, without metadata
    fn new_no_match_no_metadata() -> Self;
}

impl<T: Debug, E: Debug> MatcherResultExt<T, E> for MatcherResult<T, E> {
    fn new_error(err: E) -> Self {
        Self::Err(err)
    }

    fn is_match(&self) -> bool {
        matches!(self, Ok(MatchResult { matched: true, .. }))
    }

    fn into_metadata(self) -> T {
        self.unwrap().metadata
    }
}

impl<T: RuleMetadata, E: Debug> MatcherResultRuleMetadataExt<T, E> for MatcherResult<T, E> {
    fn new_match(metadata: T) -> Self {
        Self::Ok(MatchResult::new_match(T::RULE_NAME.to_string(), metadata))
    }

    fn new_no_match(metadata: T) -> Self {
        Self::Ok(MatchResult::new_no_match(
            T::RULE_NAME.to_string(),
            metadata,
        ))
    }
}

impl<T: RuleMetadata, E: Debug> MatcherResultRuleMetadataExt<T, E> for MatcherResult<Option<T>, E> {
    fn new_match(metadata: T) -> Self {
        Self::Ok(MatchResult::new_match(
            T::RULE_NAME.to_string(),
            Some(metadata),
        ))
    }

    fn new_no_match(metadata: T) -> Self {
        Self::Ok(MatchResult::new_no_match(
            T::RULE_NAME.to_string(),
            Some(metadata),
        ))
    }
}

impl<T: RuleMetadata, E: Debug> MatcherResultRuleOptionMetadataExt<T, E>
    for MatcherResult<Option<T>, E>
{
    fn new_no_match_no_metadata() -> Self {
        Self::Ok(MatchResult::new_no_match(T::RULE_NAME.to_string(), None))
    }
}

/// Interface of a matcher rule.
/// It defines outputs of the matcher and the actual implementation, since different matchers can
/// produce different metadata, to give additional information on the match.
pub trait MatcherRule: 'static {
    /// Additional data for positive matches.
    type OutputMetadata: RuleMetadata;
    /// Error type for matcher (negative matches should not be treated as errors).
    type Error: std::error::Error + 'static;

    /// Tries to match input string to target string using this rule.
    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error>;

    /// Estimates the resource cost of the rule
    fn estimate(&self, target_str: &str) -> EstimationResult;
}

/// Result of rule cost estimation.
///
/// Represents the expected resource cost of running the rule, as well as expected cost bounds and
/// cost scaling with input string size.
#[derive(Debug)]
pub struct EstimationResult {
    /// Represents the minimum possible cost
    pub min: Option<usize>,
    /// Represents the maximum possible cost
    pub max: Option<usize>,
    /// Represents the calculated cost
    pub calculated: usize,
    /// Represents the influence of the input string size to the cost
    pub input_string_influence: InputStringInfluence,
}

impl std::ops::AddAssign for EstimationResult {
    fn add_assign(&mut self, rhs: Self) {
        self.min = self.min.map(|m| m + rhs.min.unwrap_or(0)).or(rhs.min);
        self.max = if self.max.is_none() || rhs.max.is_none() {
            None
        } else {
            self.max.map(|m| m + rhs.max.unwrap_or(0)).or(rhs.max)
        };
        self.calculated += rhs.calculated;
        self.input_string_influence =
            match (&self.input_string_influence, &rhs.input_string_influence) {
                (InputStringInfluence::None, other) | (other, InputStringInfluence::None) => {
                    other.clone()
                }
                (InputStringInfluence::Linear(l), InputStringInfluence::Linear(r)) => {
                    // A bit incorrect, but ok - it is an estimate after all
                    InputStringInfluence::Linear(l + r)
                }
                (InputStringInfluence::Log, other) | (other, InputStringInfluence::Log) => {
                    other.clone()
                }
                (_, InputStringInfluence::Quadratic) | (InputStringInfluence::Quadratic, _) => {
                    InputStringInfluence::Quadratic
                }
            }
    }
}

/// Influence of input string on the cost.
///
/// Represents expected scaling of cost with size of the input string.
#[derive(Default, Debug, Clone)]
pub enum InputStringInfluence {
    /// None - the input string size has none or minimal effect on the cost.
    #[default]
    None,
    /// Linear cost scaling - cost scales linearly with the input string size, with the provided
    /// factor.
    Linear(f64),
    /// Logarithmic cost scaling - cost scales logaritmically with the input string size.
    Log,
    /// Quadratic cost scaling - cost scales quadratically with the input string size.
    Quadratic,
}

impl EstimationResult {
    /// Helper for zero cost rule.
    ///
    /// Useful as a starting value when collecting multiple costs. No rule is expected to have a
    /// zero cost.
    pub fn zero() -> Self {
        Self {
            min: None,
            max: None,
            calculated: 0,
            input_string_influence: InputStringInfluence::None,
        }
    }

    /// Helper for static cost rule.
    ///
    /// Represents a rule that has an exact same cost (or very close to it) every time.
    pub fn static_cost(cost: usize) -> Self {
        Self {
            min: Some(cost),
            max: Some(cost),
            calculated: cost,
            input_string_influence: InputStringInfluence::None,
        }
    }

    /// Helper for linear cost rule.
    pub fn linear(cost: usize, factor: f64) -> Self {
        Self {
            min: None,
            max: None,
            calculated: cost,
            input_string_influence: InputStringInfluence::Linear(factor),
        }
    }
}

/// Trait for all metadata objects for MatcherRules
pub trait RuleMetadata: Debug + Serialize {
    /// Name of the rule
    const RULE_NAME: &str;
}

impl<T: RuleMetadata> RuleMetadata for Option<T> {
    const RULE_NAME: &str = T::RULE_NAME;
}

/// Conversion trait for turning matcher into generic matchers, to make it easier to use them in
/// collections.
pub trait IntoGenericMatcherRule {
    /// Converts this object into an implementation of GenericMatcherRule.
    #[must_use]
    fn into_generic_matcher(self) -> impl GenericMatcherRule;
}

/// Generic matcher rule. Works for all matchers by converting their metadata into JSON value.
pub trait GenericMatcherRule: Send + Sync + 'static {
    /// Name of the rule
    fn name(&self) -> &str;

    /// Tries to match input string to target string using this rule, turning result into a generic
    /// value.
    fn match_rule_generic(
        &self,
        input_str: &str,
        target_str: &str,
        full_metadata_for_all: bool,
    ) -> GenericMatcherResult;

    /// Estimates the resource cost of the rule.
    fn estimate_generic(&self, target_str: &str) -> EstimationResult;

    /// Clones this generic matcher
    fn clone_dyn(&self) -> Box<dyn GenericMatcherRule>;
}

impl<T> GenericMatcherRule for T
where
    T: MatcherRule + Clone + Send + Sync,
{
    fn match_rule_generic(
        &self,
        input_str: &str,
        target_str: &str,
        full_metadata_for_all: bool,
    ) -> GenericMatcherResult {
        let _ = trace_span!(
            "rule",
            input = input_str,
            target = target_str,
            rule = T::OutputMetadata::RULE_NAME
        )
        .enter();
        debug!(
            message = "Matching rule",
            input = input_str,
            target = target_str,
            rule = T::OutputMetadata::RULE_NAME
        );
        let result = self.match_rule(input_str, target_str).map_err(Box::new)?;
        if result.matched || full_metadata_for_all {
            result.try_into_generic_result()
        } else {
            Ok(MatchResult {
                rule_type: result.rule_type,
                matched: result.matched,
                metadata: Map::default(),
            })
        }
    }

    fn estimate_generic(&self, target_str: &str) -> EstimationResult {
        self.estimate(target_str)
    }

    fn name(&self) -> &str {
        T::OutputMetadata::RULE_NAME
    }

    fn clone_dyn(&self) -> Box<dyn GenericMatcherRule> {
        Box::new(self.clone())
    }
}

impl<T> IntoGenericMatcherRule for T
where
    T: GenericMatcherRule,
    T: MatcherRule,
{
    fn into_generic_matcher(self) -> impl GenericMatcherRule {
        self
    }
}

#[cfg(test)]
/// Example matcher
#[derive(Clone)]
pub struct ExampleRule;

#[cfg(test)]
impl MatcherRule for ExampleRule {
    type OutputMetadata = ();
    type Error = std::io::Error;

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        if input_str.contains(target_str) {
            MatcherResult::new_match(())
        } else {
            MatcherResult::new_no_match(())
        }
    }

    fn estimate(&self, _target_str: &str) -> EstimationResult {
        EstimationResult::zero()
    }
}

#[cfg(test)]
impl RuleMetadata for () {
    const RULE_NAME: &str = "example";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_rule_test_match() {
        let rule = ExampleRule;

        assert!(rule.match_rule("test string", "test").is_match());
        assert!(!rule.match_rule("some other string", "test").is_match());
    }

    #[test]
    fn example_rule_generic_test_match() {
        let rule = ExampleRule;
        let generic_rule = rule.into_generic_matcher();

        assert!(
            generic_rule
                .match_rule_generic("test string", "test", false)
                .is_match()
        );
        assert!(
            !generic_rule
                .match_rule_generic("some other string", "test", false)
                .is_match()
        );
    }
}
