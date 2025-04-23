//! stringsimile matcher rules

use std::fmt::Debug;

use serde::Serialize;
use serde_json::{Map, Value};

/// Type alias for rule matchers results.
pub type MatcherResult<T, E> = Result<Option<T>, E>;
/// Type alias for generic errors.
pub type Error = Box<dyn std::error::Error>;

/// Helper functions for rule matchers results
pub trait MatcherResultExt<T: Debug, E: Debug> {
    /// Creates a new successful match result
    fn new_match(metadata: T) -> Self;
    /// Creates a new successful no match result
    fn new_no_match() -> Self;
    /// Creates a new error
    fn new_error(err: E) -> Self;

    /// Returns true only if result is successful and a match
    fn is_match(&self) -> bool;
    /// Returns internal metadata of the match result.
    ///
    /// Panics if the result is not successful or not a match
    fn into_metadata(self) -> T;
}

impl<T: Debug, E: Debug> MatcherResultExt<T, E> for MatcherResult<T, E> {
    fn new_match(metadata: T) -> Self {
        Self::Ok(Some(metadata))
    }

    fn new_no_match() -> Self {
        Self::Ok(None)
    }

    fn new_error(err: E) -> Self {
        Self::Err(err)
    }

    fn is_match(&self) -> bool {
        matches!(self, Ok(Some(_)))
    }

    fn into_metadata(self) -> T {
        self.unwrap().unwrap()
    }
}

/// Interface of a matcher rule.
/// It defines outputs of the matcher and the actual implementation, since different matchers can
/// produce different metadata, to give additional information on the match.
pub trait MatcherRule {
    /// Additional data for positive matches.
    type OutputMetadata: Debug + Serialize;
    /// Error type for matcher (negative matches should not be treated as errors).
    type Error: std::error::Error + 'static;

    /// Tries to match input string to target string using this rule.
    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error>;
}

/// Conversion trait for turning matcher into generic matchers, to make it easier to use them in
/// collections.
pub trait IntoGenericMatcherRule {
    /// Converts this object into an implementation of GenericMatcherRule.
    fn into_generic_matcher(self) -> impl GenericMatcherRule;
}

/// Generic matcher rule. Works for all matchers by converting their metadata into JSON value.
pub trait GenericMatcherRule {
    /// Tries to match input string to target string using this rule, turning result into a generic
    /// value.
    fn match_rule_generic(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Map<String, Value>, Box<dyn std::error::Error>>;
}

impl<T> GenericMatcherRule for T
where
    T: MatcherRule,
{
    fn match_rule_generic(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Map<String, Value>, Box<dyn std::error::Error>> {
        self.match_rule(input_str, target_str)
            .map_err(Box::new)?
            .map(|metadata| {
                serde_json::to_value(metadata).map(|v| match v {
                    Value::Object(map) => map,
                    Value::Null | Value::Bool(_) => Map::default(),
                    _ => panic!("Expected rule metadata to serialize into object"),
                })
            })
            .transpose()
            .map_err(Into::into)
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
            MatcherResult::new_no_match()
        }
    }
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
                .match_rule_generic("test string", "test")
                .is_match()
        );
        assert!(
            !generic_rule
                .match_rule_generic("some other string", "test")
                .is_match()
        );
    }
}
