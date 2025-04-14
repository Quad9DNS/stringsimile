//! stringsimile matcher rules

use std::fmt::Debug;

/// Matching result
#[derive(Debug)]
pub enum MatcherResult<T: Debug, E: Debug> {
    /// Positive match
    Match(T),
    /// Negative match
    NoMatch,
    /// Error in the matching process
    Error(E),
}

impl<T: Debug, E: Debug> MatcherResult<T, E> {
    #[allow(unused)]
    fn into_option_result(self) -> Result<Option<T>, E> {
        match self {
            MatcherResult::Match(res) => Ok(Some(res)),
            MatcherResult::NoMatch => Ok(None),
            MatcherResult::Error(err) => Err(err),
        }
    }
}

/// Rule
pub trait MatcherRule {
    /// Additional data for positive matches
    type OutputMetadata: Debug;
    /// Error type for matcher (negative matches should not be treated as errors)
    type Error: Debug;

    /// Tries to match target string to this rule
    fn match_rule(&self, target_str: &str) -> MatcherResult<Self::OutputMetadata, Self::Error>;
}

#[cfg(test)]
/// Example matcher
pub struct ExampleRule {
    target_match: String,
}

#[cfg(test)]
impl MatcherRule for ExampleRule {
    type OutputMetadata = ();
    type Error = ();

    fn match_rule(&self, target_str: &str) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        if target_str.contains(&self.target_match) {
            MatcherResult::Match(())
        } else {
            MatcherResult::NoMatch
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_rule_test_match() {
        let rule = ExampleRule {
            target_match: "test".to_string(),
        };

        assert!(matches!(
            rule.match_rule("test string"),
            MatcherResult::Match(())
        ));
        assert!(matches!(
            rule.match_rule("some other string"),
            MatcherResult::NoMatch
        ));
    }
}
