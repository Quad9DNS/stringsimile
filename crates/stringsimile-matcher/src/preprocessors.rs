//! stringsimile matcher preprocessors

use std::{fmt::Debug, iter::Peekable, path::PathBuf};

use hashbrown::HashSet;
use serde_json::{Map, Value};

use crate::ruleset::{ExclusionSetMetrics, PreprocessorContext};

/// Preprocessor - prepares data before executing rules
#[derive(Debug, Clone)]
pub enum Preprocessor {
    /// Split target preprocessor
    ///
    /// Splits input string on '.' character and optionally ignores last part.
    /// Useful for domain names.
    SplitTarget(SplitTargetConfig),
    /// Exclusion set preprocessor
    ///
    /// Splits input string on '.' character and optionally ignores last part.
    /// Useful for domain names.
    ExclusionSet(ExclusionSetConfig),
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
    // TODO: Change ot use `Cow` so preprocessors can generate their own stuff
    /// Processes iterator of input data producing another iterator with modified data
    pub fn process<'a>(
        &self,
        input: Box<dyn Iterator<Item = &'a str> + 'a>,
        context: &'a PreprocessorContext,
    ) -> Box<dyn Iterator<Item = &'a str> + 'a> {
        match self {
            Preprocessor::SplitTarget(config) => {
                let ignore_tld = config.ignore_tld;
                Box::new(input.flat_map(move |i| {
                    i.split('.')
                        .filter(|s| !s.is_empty())
                        .split_target(ignore_tld)
                }))
            }
            Preprocessor::ExclusionSet(exclusion_set_config) => {
                exclusion_set_config.process(input, context)
            }
        }
    }

    /// Builds context for this preprocessor
    pub fn build_context(
        &self,
        string_group_name: &str,
        ruleset_name: &str,
        preprocessor_index: usize,
    ) -> PreprocessorContext {
        match self {
            Preprocessor::SplitTarget(_) => PreprocessorContext::Empty,
            Preprocessor::ExclusionSet(exclusion_set_config) => PreprocessorContext::ExclusionSet {
                metrics: ExclusionSetMetrics::new(
                    string_group_name,
                    ruleset_name,
                    preprocessor_index,
                ),
                context: match (exclusion_set_config.regex, &exclusion_set_config.source) {
                    (true, _) => ExclusionSetContext::RegexSet,
                    (false, ExclusionSetSource::File(_path_buf)) => {
                        ExclusionSetContext::ExactFileSet
                    }
                    (false, ExclusionSetSource::Static(items)) => {
                        ExclusionSetContext::ExactStaticSet(HashSet::from_iter(
                            items.iter().cloned(),
                        ))
                    }
                },
            },
        }
    }

    /// Preloads context for this preprocessor
    pub async fn preload_context(&self, context: &mut PreprocessorContext) {
        match self {
            Preprocessor::SplitTarget(_) => {}
            Preprocessor::ExclusionSet(exclusion_set_config) => {
                match (
                    &exclusion_set_config.regex,
                    &exclusion_set_config.source,
                    context,
                ) {
                    (
                        false,
                        ExclusionSetSource::File(_path_buf),
                        PreprocessorContext::ExclusionSet {
                            metrics: _metrics,
                            context: ExclusionSetContext::ExactFileSet,
                        },
                    ) => todo!(),
                    (
                        false,
                        ExclusionSetSource::Static(_),
                        PreprocessorContext::ExclusionSet {
                            metrics: _,
                            context: ExclusionSetContext::ExactStaticSet(_),
                        },
                    ) => {
                        // Data is already loaded in this set
                    }
                    (
                        true,
                        _source,
                        PreprocessorContext::ExclusionSet {
                            metrics: _metrics,
                            context: ExclusionSetContext::RegexSet,
                        },
                    ) => {}
                    (_, _, PreprocessorContext::Empty) => {
                        unreachable!("exclusion set preprocessor requires a context")
                    }
                    (_, _, _) => unreachable!("Context preprocessor mismatch"),
                }
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
            Preprocessor::ExclusionSet(_) => {}
        }
    }
}

/// Configuration for the split target preprocessor
#[derive(Debug, Clone, Default)]
pub struct SplitTargetConfig {
    /// If set to true, will ignore TLD part of the split string
    pub ignore_tld: bool,
}

/// Configuration for the exclusion set preprocessor source
#[derive(Debug, Clone)]
pub enum ExclusionSetSource {
    /// Path to a file containing exclusion set, one string per line
    File(PathBuf),
    /// Static list of strings to put in the exclusion set
    Static(Vec<String>),
}

impl Default for ExclusionSetSource {
    fn default() -> Self {
        Self::Static(Vec::default())
    }
}

/// Context for a specific exclusion set type
pub enum ExclusionSetContext {
    /// Context for static exact match exclusion sets
    ExactStaticSet(HashSet<String>),
    // TODO
    /// Context for file sourced exact match exclusion sets
    ExactFileSet,
    // TODO
    /// Context for regex exclusion sets (both file and static)
    RegexSet,
}

/// Configuration for the exclusion set preprocessor
#[derive(Debug, Clone, Default)]
pub struct ExclusionSetConfig {
    /// Source to take exclusion set from
    pub source: ExclusionSetSource,
    /// If set to true, all entries will be considered as regex patterns
    pub regex: bool,
}

impl ExclusionSetConfig {
    /// Processes iterator of input data producing another iterator with modified data
    pub fn process<'a>(
        &self,
        input: Box<dyn Iterator<Item = &'a str> + 'a>,
        context: &'a PreprocessorContext,
    ) -> Box<dyn Iterator<Item = &'a str> + 'a> {
        let PreprocessorContext::ExclusionSet { metrics, context } = context else {
            return input;
        };
        match &self.source {
            ExclusionSetSource::File(_) => todo!(),
            ExclusionSetSource::Static(_) => {
                let ExclusionSetContext::ExactStaticSet(set) = context else {
                    return input;
                };
                Box::new(input.filter(|p| {
                    let res = set.contains(*p);
                    if res {
                        metrics.exclusions.increment(1);
                    }
                    !res
                }))
            }
        }
    }
}
