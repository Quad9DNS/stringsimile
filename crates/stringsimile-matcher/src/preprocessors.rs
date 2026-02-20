//! stringsimile matcher preprocessors

use std::{fmt::Debug, iter::Peekable};

use serde_json::{Map, Value};

/// Preprocessor - prepares data before executing rules
#[derive(Debug, Clone)]
pub enum Preprocessor {
    /// Split target preprocessor
    ///
    /// Splits input string on '.' character and optionally ignores last part.
    /// Useful for domain names.
    SplitTarget(SplitTargetConfig),
}

struct IgnoreLastIterator<I: Iterator> {
    iter: Peekable<I>,
    ignore_last: bool,
}

trait IntoIgnoreLastIterator: Iterator + Sized {
    fn split_target(self, ignore_last: bool) -> IgnoreLastIterator<Self>;
}

impl<I: Iterator> IntoIgnoreLastIterator for I {
    fn split_target(self, ignore_last: bool) -> IgnoreLastIterator<Self> {
        IgnoreLastIterator {
            iter: self.peekable(),
            ignore_last,
        }
    }
}

impl<I: Iterator> Iterator for IgnoreLastIterator<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(item) => {
                if self.ignore_last && self.iter.peek().is_none() {
                    None
                } else {
                    Some(item)
                }
            }
            None => None,
        }
    }
}

impl Preprocessor {
    /// Processes iterator of input data producing another iterator with modified data
    pub fn process<'a>(
        &self,
        input: Box<dyn Iterator<Item = &'a str> + 'a>,
    ) -> Box<dyn Iterator<Item = &'a str> + 'a> {
        match self {
            Preprocessor::SplitTarget(config) => {
                let ignore_tld = config.ignore_tld;
                Box::new(input.flat_map(move |i| i.split('.').split_target(ignore_tld)))
            }
        }
    }

    /// Adds metadata to the matched result, based on this preprocessor
    pub fn add_metadata(&self, metadata: &mut Map<String, Value>, (index, part): (usize, &str)) {
        match self {
            Preprocessor::SplitTarget(_) => {
                metadata.insert(
                    "split_string".to_string(),
                    Value::String(part.to_string().clone()),
                );
                metadata.insert("split_position".to_string(), Value::Number(index.into()));
            }
        }
    }
}

/// Configuration for the split target preprocessor
#[derive(Debug, Clone, Default)]
pub struct SplitTargetConfig {
    /// If set to true, will ignore TLD part of the split string
    pub ignore_tld: bool,
}
