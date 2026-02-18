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
