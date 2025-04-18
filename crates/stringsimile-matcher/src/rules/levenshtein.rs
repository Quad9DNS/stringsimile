use std::num::NonZeroU32;

use triple_accel::levenshtein_exp;

use crate::rule::{MatcherResult, MatcherResultExt, MatcherRule};

#[derive(Debug)]
struct LevenhsteinRule {
    maximum_distance: NonZeroU32,
}

#[derive(Debug)]
struct LevenhsteinMetadata {
    #[allow(unused)]
    distance: NonZeroU32,
}

impl MatcherRule for LevenhsteinRule {
    type OutputMetadata = LevenhsteinMetadata;
    type Error = ();

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        let res = levenshtein_exp(input_str.as_bytes(), target_str.as_bytes());
        if res <= self.maximum_distance.get() {
            MatcherResult::new_match(LevenhsteinMetadata {
                distance: NonZeroU32::try_from(res).map_err(|_| ())?,
            })
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
        let rule = LevenhsteinRule {
            maximum_distance: 2.try_into().unwrap(),
        };

        let result = rule.match_rule("test", "tset");
        assert!(result.is_match());
        assert_eq!(result.into_metadata().distance, 2.try_into().unwrap());
    }
}
