//! Match Rating rule implementation

use std::{fmt::Debug, io::Error};

use rphonetic::{Encoder, MatchRatingApproach};
use serde::{Deserialize, Serialize};

use crate::{
    MatcherResult,
    rule::{MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

/// Rule
#[derive(Debug, Clone)]
pub struct MatchRatingRule {
    /// Pre-encoded target string
    encoded_target: String,
}

impl MatchRatingRule {
    /// Creates an instance of match rating rule with pre-computed target string encoding
    pub fn new(target_str: &str) -> Self {
        Self {
            encoded_target: MatchRatingApproach.encode(target_str),
        }
    }
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRatingMetadata;

impl MatcherRule for MatchRatingRule {
    type OutputMetadata = MatchRatingMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        let matches = 'block: {
            if !input_str.trim().is_empty() && input_str.trim().len() != 1 {
                let input = MatchRatingApproach.encode(input_str);
                if input.len().abs_diff(self.encoded_target.len()) >= 3 {
                    break 'block false;
                }

                let sum_length = input.len() + self.encoded_target.len();

                let min_rating = get_minimum_rating(sum_length);
                let count =
                    left_to_right_then_right_to_left_processing(input, &self.encoded_target);

                count >= min_rating
            } else {
                input_str == target_str
            }
        };

        if matches {
            MatcherResult::new_match(MatchRatingMetadata)
        } else {
            MatcherResult::new_no_match(MatchRatingMetadata)
        }
    }
}

// Taken from rphonetic::MatchRatingApproach to allow us to use our pre-encoded target
fn get_minimum_rating(sum_length: usize) -> usize {
    match sum_length {
        0..=4 => 5,
        5..=7 => 4,
        8..=11 => 3,
        12 => 2,
        _ => 1,
    }
}

fn left_to_right_then_right_to_left_processing(name1: String, name2: &str) -> usize {
    let mut n1: Vec<char> = name1.chars().collect();
    let mut n2: Vec<char> = name2.chars().collect();

    for i in 0..n1.len() {
        if i >= n2.len() {
            break;
        }

        let c1: &char = n1.get(i).unwrap();
        let c2: &char = n2.get(i).unwrap();
        if c1 == c2 {
            n1[i] = ' ';
            n2[i] = ' ';
        }
    }

    n1.retain(|c| *c != ' ');
    n2.retain(|c| *c != ' ');

    let n1len = n1.len() - 1;
    let n2len = n2.len() - 1;

    for i in 0..n1.len() {
        if i >= n2.len() {
            break;
        }

        let c1: &char = n1.get(n1len - i).unwrap();
        let c2: &char = n2.get(n2len - i).unwrap();
        if c1 == c2 {
            n1[n1len - i] = ' ';
            n2[n2len - i] = ' ';
        }
    }

    let r1 = n1.iter().filter(|c| c != &&' ').count();
    let r2 = n2.iter().filter(|c| c != &&' ').count();

    if r1 > r2 {
        6usize.abs_diff(r1)
    } else {
        6usize.abs_diff(r2)
    }
}

impl RuleMetadata for MatchRatingMetadata {
    const RULE_NAME: &str = "match_rating";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    #[test]
    fn simple_example_normal() {
        let rule = MatchRatingRule::new("Frances");

        let result = rule.match_rule("Franciszek", "Frances");
        assert!(result.is_match());
    }
}
