//! stringsimile matchers
//!
//! This crate provides basic building blocks of the stringsimile rule engine.
//! It provides rule matchers and their configuration.

#![deny(unreachable_pub)]
#![deny(unused_extern_crates)]
#![deny(unused_allocation)]
#![deny(unused_assignments)]
#![deny(unused_comparisons)]
#![deny(warnings)]
#![deny(missing_docs)]

use serde::Serialize;
use serde_json::{Map, Value};
pub mod preprocessors;
pub mod rule;
pub mod rules;
pub mod ruleset;

/// General match result
pub struct MatchResult<T> {
    /// Rule type used to generate this result
    pub rule_type: String,
    /// Whether the matcher has successfully matched the target string
    pub matched: bool,
    /// Metadata related to the specific rule
    pub metadata: T,
}

/// Type alias for generic match results
pub type GenericMatchResult = MatchResult<Map<String, Value>>;
/// Type alias for rule matchers results.
pub type MatcherResult<T, E> = std::result::Result<MatchResult<T>, E>;
/// Type alias for generic errors.
pub type Error = Box<dyn std::error::Error>;
/// Type alias for rule matchers results.
pub type GenericMatcherResult = std::result::Result<GenericMatchResult, Error>;

impl<T> MatchResult<T> {
    /// Creates a new successful match
    pub fn new_match(rule_type: String, metadata: T) -> Self {
        Self {
            rule_type,
            matched: true,
            metadata,
        }
    }

    /// Creates a new failed match
    pub fn new_no_match(rule_type: String, metadata: T) -> Self {
        Self {
            rule_type,
            matched: false,
            metadata,
        }
    }
}

impl<T> MatchResult<T>
where
    T: Serialize,
{
    fn try_into_generic_result(self) -> GenericMatcherResult {
        Ok(MatchResult {
            rule_type: self.rule_type,
            matched: self.matched,
            metadata: serde_json::to_value(self.metadata).map(|v| match v {
                Value::Object(map) => map,
                Value::Null | Value::Bool(_) => Map::default(),
                _ => panic!("Expected rule metadata to serialize into object"),
            })?,
        })
    }
}

impl GenericMatchResult {
    fn into_full_metadata(self) -> GenericMatchResult {
        let mut metadata = self.metadata;
        metadata.insert("match".to_string(), Value::Bool(self.matched));
        metadata.insert(
            "rule_type".to_string(),
            Value::String(self.rule_type.clone()),
        );
        MatchResult {
            rule_type: self.rule_type,
            matched: self.matched,
            metadata,
        }
    }
}
