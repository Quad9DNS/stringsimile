//! stringsimile matcher preprocessors

use std::{
    fmt::Debug,
    fs::{self, File},
    hash::DefaultHasher,
    io::{BufRead, BufReader},
    iter::Peekable,
    path::PathBuf,
};

use hashbrown::HashSet;
use hyperscan::{BlockDatabase, Builder, Matching, Pattern, Patterns};
use serde_json::{Map, Value};
use xorf::{Filter, HashProxy, Xor32};

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
    /// Excludes known strings from being processed by the rules in the ruleset.
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
                    (true, ExclusionSetSource::File(_)) => {
                        ExclusionSetContext::RegexSet { db: None }
                    }
                    (true, ExclusionSetSource::Static(items)) => {
                        let patterns = items
                            .iter()
                            .map(Pattern::new)
                            .collect::<Result<Vec<_>, _>>()
                            .unwrap();
                        let db = Patterns::from_iter(patterns).build().unwrap();

                        ExclusionSetContext::RegexSet { db: Some(db) }
                    }
                    (false, ExclusionSetSource::File(path_buf)) => {
                        ExclusionSetContext::ExactFileSet {
                            path: path_buf.clone(),
                            filter: None,
                        }
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
                            context: ExclusionSetContext::ExactFileSet { path, filter },
                        },
                    ) => {
                        let set: Vec<String> = fs::read_to_string(path)
                            .unwrap()
                            .lines()
                            .map(|l| l.to_string())
                            .collect::<Vec<_>>();
                        *filter = Some(HashProxy::from(&set));
                    }
                    (_, ExclusionSetSource::Static(_), _) => {
                        // Data is already loaded in this set
                    }
                    (
                        true,
                        ExclusionSetSource::File(path_buf),
                        PreprocessorContext::ExclusionSet {
                            metrics: _,
                            context: ExclusionSetContext::RegexSet { db },
                        },
                    ) => {
                        let lines = BufReader::new(File::open(path_buf).unwrap()).lines();
                        *db = Some(
                            Patterns::from(
                                lines
                                    .map(|l| l.map(|l| Pattern::new(l).unwrap()))
                                    .collect::<Result<Vec<_>, _>>()
                                    .unwrap(),
                            )
                            .build()
                            .unwrap(),
                        )
                    }
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
    /// Context for file sourced exact match exclusion sets
    ExactFileSet {
        /// The path to the file
        path: PathBuf,
        /// Probabilistic filter for checking if there is a need to look into the file
        filter: Option<HashProxy<String, DefaultHasher, Xor32>>,
    },
    /// Context for regex exclusion sets (both file and static)
    RegexSet {
        /// The compiled hyperscan database, if available
        db: Option<BlockDatabase>,
    },
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
        if self.regex {
            let ExclusionSetContext::RegexSet { db } = context else {
                return input;
            };
            let Some(db) = db else {
                return input;
            };
            let scratch = db.alloc_scratch().unwrap();
            return Box::new(input.filter(move |p| {
                let mut matched = false;
                let _ = db.scan(p, &scratch, |_, _, _, _| {
                    matched = true;
                    Matching::Terminate
                });
                if matched {
                    metrics.exclusions.increment(1);
                }
                !matched
            }));
        }

        match &self.source {
            ExclusionSetSource::File(_) => {
                let ExclusionSetContext::ExactFileSet { path, filter } = context else {
                    return input;
                };
                Box::new(input.filter(move |p| {
                    if let Some(filter) = filter
                        && filter.contains(&p.to_string())
                    {
                        let mut lines = BufReader::new(File::open(path).unwrap()).lines();
                        let matched = lines.any(|l| l.unwrap() == **p);
                        if matched {
                            metrics.exclusions.increment(1);
                        }
                        !matched
                    } else {
                        true
                    }
                }))
            }
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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn split_target_basic() {
        let input = Box::new(vec!["this.is.test.string.com."].into_iter());
        let split_target = Preprocessor::SplitTarget(SplitTargetConfig { ignore_tld: false });

        let mut result = split_target.process(input, &PreprocessorContext::Empty);

        assert_eq!(result.next().unwrap(), "this");
        assert_eq!(result.next().unwrap(), "is");
        assert_eq!(result.next().unwrap(), "test");
        assert_eq!(result.next().unwrap(), "string");
        assert_eq!(result.next().unwrap(), "com");
        assert_eq!(result.next(), None);
    }

    #[test]
    fn split_target_basic_ignore_tld() {
        let input = Box::new(vec!["this.is.test.string.com."].into_iter());
        let split_target = Preprocessor::SplitTarget(SplitTargetConfig { ignore_tld: true });

        let mut result = split_target.process(input, &PreprocessorContext::Empty);

        assert_eq!(result.next().unwrap(), "this");
        assert_eq!(result.next().unwrap(), "is");
        assert_eq!(result.next().unwrap(), "test");
        assert_eq!(result.next().unwrap(), "string");
        assert_eq!(result.next(), None);
    }

    #[tokio::test]
    async fn exclusion_set_static() {
        let input = Box::new(vec!["this", "is", "test", "string", "com"].into_iter());
        let exclusion_set = Preprocessor::ExclusionSet(ExclusionSetConfig {
            source: ExclusionSetSource::Static(vec!["is".to_string(), "com".to_string()]),
            regex: false,
        });

        let ctx = exclusion_set.build_context("test", "Test", 0);
        let mut result = exclusion_set.process(input, &ctx);

        assert_eq!(result.next().unwrap(), "this");
        assert_eq!(result.next().unwrap(), "test");
        assert_eq!(result.next().unwrap(), "string");
        assert_eq!(result.next(), None);
    }

    #[tokio::test]
    async fn exclusion_set_regex() {
        let input = Box::new(vec!["this", "is", "test", "string", "com"].into_iter());
        let exclusion_set = Preprocessor::ExclusionSet(ExclusionSetConfig {
            source: ExclusionSetSource::Static(vec!["^t.*".to_string()]),
            regex: true,
        });

        let ctx = exclusion_set.build_context("test", "Test", 0);
        let mut result = exclusion_set.process(input, &ctx);

        assert_eq!(result.next().unwrap(), "is");
        assert_eq!(result.next().unwrap(), "string");
        assert_eq!(result.next().unwrap(), "com");
        assert_eq!(result.next(), None);
    }

    #[tokio::test]
    async fn exclusion_set_regex_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "^t.*\n^str.*\n").unwrap();
        let input = Box::new(vec!["this", "is", "test", "string", "com"].into_iter());
        let exclusion_set = Preprocessor::ExclusionSet(ExclusionSetConfig {
            source: ExclusionSetSource::File(file.path().to_path_buf()),
            regex: true,
        });

        let mut ctx = exclusion_set.build_context("test", "Test", 0);
        exclusion_set.preload_context(&mut ctx).await;
        let mut result = exclusion_set.process(input, &ctx);

        assert_eq!(result.next().unwrap(), "is");
        assert_eq!(result.next().unwrap(), "com");
        assert_eq!(result.next(), None);
    }

    #[tokio::test]
    async fn exclusion_set_exact_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "is\ncom\n").unwrap();
        let input = Box::new(vec!["this", "is", "test", "string", "com"].into_iter());
        let exclusion_set = Preprocessor::ExclusionSet(ExclusionSetConfig {
            source: ExclusionSetSource::File(file.path().to_path_buf()),
            regex: false,
        });

        let mut ctx = exclusion_set.build_context("test", "Test", 0);
        exclusion_set.preload_context(&mut ctx).await;
        let mut result = exclusion_set.process(input, &ctx);

        assert_eq!(result.next().unwrap(), "this");
        assert_eq!(result.next().unwrap(), "test");
        assert_eq!(result.next().unwrap(), "string");
        assert_eq!(result.next(), None);
    }
}
