//! stringsimile matcher rules

use std::fmt::Debug;

/// Type alias for rule matchers results.
pub type MatcherResult<T, E> = Result<Option<T>, E>;

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

/// Rule
pub trait MatcherRule {
    /// Additional data for positive matches
    type OutputMetadata: Debug;
    /// Error type for matcher (negative matches should not be treated as errors)
    type Error: Debug;

    /// Tries to match input string to target string using this rule
    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error>;
}

#[cfg(test)]
/// Example matcher
pub struct ExampleRule;

#[cfg(test)]
impl MatcherRule for ExampleRule {
    type OutputMetadata = ();
    type Error = ();

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
}
