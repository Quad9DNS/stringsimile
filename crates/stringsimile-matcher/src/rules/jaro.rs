//! Jaro rule implementation

use serde::{Deserialize, Serialize};
use strsim::jaro;

use crate::rule::{MatcherResult, MatcherResultExt, MatcherRule};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Rule
pub struct JaroRule {
    /// Minimum match percentage for this rule to be considered a match
    pub match_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// metadata
pub struct JaroMetadata {
    #[allow(unused)]
    match_percent: f64,
}

impl MatcherRule for JaroRule {
    type OutputMetadata = JaroMetadata;
    type Error = ();

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        let res = jaro(input_str, target_str);
        if res >= self.match_percent {
            MatcherResult::new_match(JaroMetadata { match_percent: res })
        } else {
            MatcherResult::new_no_match()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_example() {
        let rule = JaroRule {
            match_percent: 0.85,
        };

        let result = rule.match_rule("test", "tset");
        assert!(result.is_match());
        assert_eq!(result.into_metadata().match_percent, 2.0);
    }
}
