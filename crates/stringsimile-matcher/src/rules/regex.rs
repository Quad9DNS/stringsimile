//! Regex rule implementation

use std::{fmt::Debug, io::Error};

use regex::Regex;
use regex_syntax::hir::Hir;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

use crate::{
    MatcherResult,
    rule::{
        EstimationResult, InputStringInfluence, MatcherResultRuleMetadataExt, MatcherRule,
        RuleMetadata,
    },
};

/// Rule
#[derive(Debug, Clone)]
pub struct RegexRule {
    regex: Regex,
    regex_hir: Hir,
}

/// Metaphone rule errors
#[derive(Debug, Clone, Snafu)]
#[snafu(visibility(pub))]
pub enum RegexBuildError {
    /// Regex rule compilation error
    #[snafu(display("Regex patten compilation failed for Regex rule: {}", source))]
    RegexCompilationError {
        /// Regex error.
        source: regex::Error,
    },

    /// Regex rule parsing error
    #[snafu(display("Regex patten parsing failed for Regex rule: {}", source))]
    RegexParsingError {
        /// Regex parsing error.
        source: regex_syntax::Error,
    },
}

impl RegexRule {
    /// Creates a new instance of [`RegexRule`], with compiled pattern.
    #[allow(clippy::result_large_err)]
    pub fn new(pattern: String) -> Result<Self, RegexBuildError> {
        // TODO: Since we are accepting untrusted patterns, maybe regex size should be limited?
        // TODO: https://docs.rs/regex/latest/regex/#untrusted-input provides more info about regex
        Ok(Self {
            regex: Regex::new(&pattern).context(RegexCompilationSnafu)?,
            regex_hir: regex_syntax::parse(&pattern).context(RegexParsingSnafu)?,
        })
    }
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegexMetadata;

impl MatcherRule for RegexRule {
    type OutputMetadata = RegexMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        _target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        if self.regex.is_match(input_str) {
            MatcherResult::new_match(RegexMetadata)
        } else {
            MatcherResult::new_no_match(RegexMetadata)
        }
    }

    fn estimate(&self, _target_str: &str) -> EstimationResult {
        // Basing estimations based on regex crate docs
        //
        // The implementation is limited and doesn't support some features that can't be implemented
        // efficiently, but then has a worst case search of O(m*n) where m is proportional to regex
        // size and n is proportional to the input string
        //
        // But, there are some special optimizations, of which a notable one is presence of literal
        // If there is a literal in the pattern, the library can quickly look for it using SIMD
        // instructions and then look for the rest of the pattern around it - this can reduce/remove
        // scaling on the input string when literals are present
        //
        // Other than that, we try to figure out relative size of the regex using the parsed HIR
        let props = self.regex_hir.properties().clone();
        let mut has_literal = false;
        let pattern_size = calculate_hir_size(self.regex_hir.clone());
        visit_hir(self.regex_hir.clone(), |hir| {
            if matches!(hir.kind(), regex_syntax::hir::HirKind::Literal(_)) {
                has_literal = true;
            }
        });
        EstimationResult {
            // Absolute minimum is 1, but it kind of scales with target str
            min: Some(if has_literal {
                1
            } else {
                props
                    .minimum_len()
                    .map(|m| m * pattern_size)
                    .unwrap_or(((pattern_size as f64 * 0.1) as usize).max(1))
            }),
            max: props
                .maximum_len()
                .map(|l| ((l as f64 * 0.1) as usize * pattern_size).max(1)),
            // TODO: figure out regex complexity
            calculated: ((pattern_size as f64 * 0.1) as usize).max(1),
            // If literal is present, regex can be optimized to quickly locate the literal in the
            // string and then after that we don't have to scan the whole string, but just as much
            // as the pattern requires
            input_string_influence: if has_literal {
                InputStringInfluence::None
            } else {
                InputStringInfluence::Linear(pattern_size as f64 * 0.1)
            },
        }
    }
}

fn visit_hir(hir: Hir, mut visitor: impl FnMut(&Hir)) {
    visitor(&hir);
    match hir.into_kind() {
        regex_syntax::hir::HirKind::Concat(hirs)
        | regex_syntax::hir::HirKind::Alternation(hirs) => hirs.iter().for_each(visitor),
        _ => (),
    }
}

fn calculate_hir_size(hir: Hir) -> usize {
    match hir.into_kind() {
        regex_syntax::hir::HirKind::Empty => 0,
        regex_syntax::hir::HirKind::Literal(literal) => literal.0.len(),
        regex_syntax::hir::HirKind::Class(class) => match class {
            regex_syntax::hir::Class::Unicode(class_unicode) => class_unicode.ranges().len(),
            regex_syntax::hir::Class::Bytes(class_bytes) => class_bytes.ranges().len(),
        },
        regex_syntax::hir::HirKind::Look(_) => 1,
        regex_syntax::hir::HirKind::Repetition(repetition) => {
            (repetition.min as usize) * calculate_hir_size(*repetition.sub.clone())
        }
        regex_syntax::hir::HirKind::Capture(capture) => calculate_hir_size(*capture.sub.clone()),
        regex_syntax::hir::HirKind::Concat(hirs) => hirs.into_iter().map(calculate_hir_size).sum(),
        // TODO: Assuming these can somehow be run in parallel, but we need to penalize it a bit more
        regex_syntax::hir::HirKind::Alternation(hirs) => {
            hirs.into_iter().map(calculate_hir_size).max().unwrap_or(1)
        }
    }
}

impl RuleMetadata for RegexMetadata {
    const RULE_NAME: &str = "regex";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    #[test]
    fn simple_example() {
        let rule = RegexRule::new(r#"netflix\.com\.$"#.to_string()).unwrap();

        let result = rule.match_rule("netflix.com.", "netflix.com.");
        assert!(result.is_match());
        let result = rule.match_rule("netflix.com", "netflix.com.");
        assert!(!result.is_match());
        let result = rule.match_rule("neftlix.com.", "netflix.com.");
        assert!(!result.is_match());
    }

    #[test]
    fn complex_pattern_example() {
        let rule = RegexRule::new(r#".*n.*t.*f.*"#.to_string()).unwrap();

        let result = rule.match_rule("netflix.com.", "netflix.com.");
        assert!(result.is_match());
        let result = rule.match_rule("netflix.com", "netflix.com.");
        assert!(result.is_match());
        let result = rule.match_rule("neftlix.com.", "netflix.com.");
        assert!(!result.is_match());
    }
}
