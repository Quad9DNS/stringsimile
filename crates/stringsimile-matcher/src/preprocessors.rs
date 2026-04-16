//! stringsimile matcher preprocessors

use std::{
    borrow::Cow,
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

/// Input data iterator
pub type BoxedTargetWithMetadataIter<'a> =
    Box<dyn Iterator<Item = (Cow<'a, str>, Cow<'a, Map<String, Value>>)> + 'a>;

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
    /// Punycode preprocessor
    ///
    /// Encodes/decodes punycode.
    /// Useful for domain names.
    Punycode(PunycodeConfig),
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
        &'a self,
        input: BoxedTargetWithMetadataIter<'a>,
        context: &'a PreprocessorContext,
    ) -> BoxedTargetWithMetadataIter<'a> {
        match self {
            Preprocessor::SplitTarget(config) => {
                let ignore_tld = config.ignore_tld;
                Box::new(input.flat_map(move |(input_str, metadata)| {
                    input_str
                        .split('.')
                        .filter(|s| !s.is_empty())
                        .enumerate()
                        .map(move |p| {
                            let mut metadata = metadata.clone();
                            metadata.to_mut().insert(
                                "split_string".to_string(),
                                Value::String(p.1.to_string().clone()),
                            );
                            metadata
                                .to_mut()
                                .insert("split_position".to_string(), Value::Number(p.0.into()));
                            (Cow::from(p.1.to_string()), metadata)
                        })
                        .split_target(ignore_tld)
                        .collect::<Vec<_>>()
                }))
            }
            Preprocessor::ExclusionSet(exclusion_set_config) => {
                exclusion_set_config.process(input, context)
            }
            Preprocessor::Punycode(punycode_config) => punycode_config.process(input),
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
            Preprocessor::SplitTarget(_) | Preprocessor::Punycode(_) => PreprocessorContext::Empty,
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
            Preprocessor::SplitTarget(_) | Preprocessor::Punycode(_) => {}
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
        input: BoxedTargetWithMetadataIter<'a>,
        context: &'a PreprocessorContext,
    ) -> BoxedTargetWithMetadataIter<'a> {
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
                let _ = db.scan(&*p.0, &scratch, |_, _, _, _| {
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
                        && filter.contains(&p.0.to_string())
                    {
                        let mut lines = BufReader::new(File::open(path).unwrap()).lines();
                        let matched = lines.any(|l| l.unwrap() == *p.0);
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
                    let res = set.contains(&*p.0);
                    if res {
                        metrics.exclusions.increment(1);
                    }
                    !res
                }))
            }
        }
    }
}

/// Configuration for the punycode preprocessor
#[derive(Debug, Clone, Default)]
pub struct PunycodeConfig {
    /// If set to true, this preprocessor will encode non-punycode strings to punycode
    pub encode: bool,
    /// If set to true, this preprocessor will decode punycode strings
    pub decode: bool,
    /// If set to true, this preprocessor will keep both the original and the encoded/decoded
    /// result
    pub keep_both: bool,
}

impl PunycodeConfig {
    /// Processes iterator of input data producing another iterator with modified data
    pub fn process<'a>(
        &'a self,
        input: BoxedTargetWithMetadataIter<'a>,
    ) -> BoxedTargetWithMetadataIter<'a> {
        Box::new(input.flat_map(move |i| {
            let mut result = Vec::new();
            let mut add_original = self.keep_both;
            let mut original_metadata = i.1;
            'encode: {
                if self.encode {
                    let has_dot = i.0.contains(".");
                    let is_ascii = i.0.is_ascii();
                    let encoded = if has_dot && !is_ascii {
                        idna::domain_to_ascii(&i.0).ok()
                    } else if !has_dot && !is_ascii {
                        idna::punycode::encode_str(&i.0)
                            .map(|encoded| "xn--".to_string() + encoded.as_ref())
                    } else {
                        if has_dot && i.0.contains("xn--") || !has_dot && i.0.starts_with("xn--") {
                            original_metadata
                                .to_mut()
                                .insert("punycode".to_string(), Value::Bool(true));
                        } else {
                            original_metadata
                                .to_mut()
                                .insert("punycode".to_string(), Value::Bool(false));
                        }
                        break 'encode;
                    };
                    original_metadata
                        .to_mut()
                        .insert("punycode".to_string(), Value::Bool(false));
                    if let Some(encoded) = encoded {
                        let metadata = Cow::Owned(Map::from_iter([(
                            "punycode".to_string(),
                            Value::Bool(true),
                        )]));
                        result.push((Cow::from(encoded), metadata));
                    } else {
                        add_original = true;
                    }
                }
            }
            'decode: {
                if self.decode {
                    let has_dot = i.0.contains(".");
                    let decoded = if has_dot && i.0.contains("xn--") {
                        if let (decoded, Ok(())) = idna::domain_to_unicode(&i.0) {
                            Some(decoded)
                        } else {
                            None
                        }
                    } else if !has_dot && i.0.starts_with("xn--") {
                        idna::punycode::decode_to_string(&i.0.replace("xn--", ""))
                    } else {
                        break 'decode;
                    };
                    original_metadata
                        .to_mut()
                        .insert("punycode".to_string(), Value::Bool(true));
                    if let Some(decoded) = decoded {
                        let metadata = Cow::Owned(Map::from_iter([(
                            "punycode".to_string(),
                            Value::Bool(false),
                        )]));
                        result.push((Cow::from(decoded), metadata));
                    } else {
                        add_original = true;
                    }
                }
            }
            if add_original {
                result.push((i.0, original_metadata));
            }
            result.into_iter()
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn split_target_basic() {
        let input = Box::new(
            vec![(
                Cow::from("this.is.test.string.com."),
                Cow::Owned(Map::default()),
            )]
            .into_iter(),
        );
        let split_target = Preprocessor::SplitTarget(SplitTargetConfig { ignore_tld: false });

        let mut result = split_target.process(input, &PreprocessorContext::Empty);

        assert_eq!(result.next().unwrap().0, "this");
        assert_eq!(result.next().unwrap().0, "is");
        assert_eq!(result.next().unwrap().0, "test");
        assert_eq!(result.next().unwrap().0, "string");
        assert_eq!(result.next().unwrap().0, "com");
        assert_eq!(result.next(), None);
    }

    #[test]
    fn split_target_basic_ignore_tld() {
        let input = Box::new(
            vec![(
                Cow::from("this.is.test.string.com."),
                Cow::Owned(Map::default()),
            )]
            .into_iter(),
        );
        let split_target = Preprocessor::SplitTarget(SplitTargetConfig { ignore_tld: true });

        let mut result = split_target.process(input, &PreprocessorContext::Empty);

        assert_eq!(result.next().unwrap().0, "this");
        assert_eq!(result.next().unwrap().0, "is");
        assert_eq!(result.next().unwrap().0, "test");
        assert_eq!(result.next().unwrap().0, "string");
        assert_eq!(result.next(), None);
    }

    #[tokio::test]
    async fn exclusion_set_static() {
        let input = Box::new(
            vec!["this", "is", "test", "string", "com"]
                .into_iter()
                .map(|s| (Cow::from(s), Cow::Owned(Map::default()))),
        );
        let exclusion_set = Preprocessor::ExclusionSet(ExclusionSetConfig {
            source: ExclusionSetSource::Static(vec!["is".to_string(), "com".to_string()]),
            regex: false,
        });

        let ctx = exclusion_set.build_context("test", "Test", 0);
        let mut result = exclusion_set.process(input, &ctx);

        assert_eq!(result.next().unwrap().0, "this");
        assert_eq!(result.next().unwrap().0, "test");
        assert_eq!(result.next().unwrap().0, "string");
        assert_eq!(result.next(), None);
    }

    #[tokio::test]
    async fn exclusion_set_regex() {
        let input = Box::new(
            vec!["this", "is", "test", "string", "com"]
                .into_iter()
                .map(|s| (Cow::from(s), Cow::Owned(Map::default()))),
        );
        let exclusion_set = Preprocessor::ExclusionSet(ExclusionSetConfig {
            source: ExclusionSetSource::Static(vec!["^t.*".to_string()]),
            regex: true,
        });

        let ctx = exclusion_set.build_context("test", "Test", 0);
        let mut result = exclusion_set.process(input, &ctx);

        assert_eq!(result.next().unwrap().0, "is");
        assert_eq!(result.next().unwrap().0, "string");
        assert_eq!(result.next().unwrap().0, "com");
        assert_eq!(result.next(), None);
    }

    #[tokio::test]
    async fn exclusion_set_regex_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "^t.*\n^str.*\n").unwrap();
        let input = Box::new(
            vec!["this", "is", "test", "string", "com"]
                .into_iter()
                .map(|s| (Cow::from(s), Cow::Owned(Map::default()))),
        );
        let exclusion_set = Preprocessor::ExclusionSet(ExclusionSetConfig {
            source: ExclusionSetSource::File(file.path().to_path_buf()),
            regex: true,
        });

        let mut ctx = exclusion_set.build_context("test", "Test", 0);
        exclusion_set.preload_context(&mut ctx).await;
        let mut result = exclusion_set.process(input, &ctx);

        assert_eq!(result.next().unwrap().0, "is");
        assert_eq!(result.next().unwrap().0, "com");
        assert_eq!(result.next(), None);
    }

    #[tokio::test]
    async fn exclusion_set_exact_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "is\ncom\n").unwrap();
        let input = Box::new(
            vec!["this", "is", "test", "string", "com"]
                .into_iter()
                .map(|s| (Cow::from(s), Cow::Owned(Map::default()))),
        );
        let exclusion_set = Preprocessor::ExclusionSet(ExclusionSetConfig {
            source: ExclusionSetSource::File(file.path().to_path_buf()),
            regex: false,
        });

        let mut ctx = exclusion_set.build_context("test", "Test", 0);
        exclusion_set.preload_context(&mut ctx).await;
        let mut result = exclusion_set.process(input, &ctx);

        assert_eq!(result.next().unwrap().0, "this");
        assert_eq!(result.next().unwrap().0, "test");
        assert_eq!(result.next().unwrap().0, "string");
        assert_eq!(result.next(), None);
    }

    #[tokio::test]
    async fn punycode_encode() {
        let input =
            Box::new(vec![(Cow::from("www.café.com"), Cow::Owned(Map::default()))].into_iter());
        let punycode = Preprocessor::Punycode(PunycodeConfig {
            encode: true,
            decode: false,
            keep_both: false,
        });

        let mut result = punycode.process(input, &PreprocessorContext::Empty);

        let first = result.next().unwrap();
        assert_eq!(first.0, "www.xn--caf-dma.com");
        assert_eq!(*first.1.get("punycode").unwrap(), Value::Bool(true));
        assert_eq!(result.next(), None);
    }

    #[tokio::test]
    async fn punycode_encode_keep_both() {
        let input =
            Box::new(vec![(Cow::from("www.café.com"), Cow::Owned(Map::default()))].into_iter());
        let punycode = Preprocessor::Punycode(PunycodeConfig {
            encode: true,
            decode: false,
            keep_both: true,
        });

        let mut result = punycode.process(input, &PreprocessorContext::Empty);

        let first = result.next().unwrap();
        assert_eq!(first.0, "www.xn--caf-dma.com");
        assert_eq!(*first.1.get("punycode").unwrap(), Value::Bool(true));
        let second = result.next().unwrap();
        assert_eq!(second.0, "www.café.com");
        assert_eq!(*second.1.get("punycode").unwrap(), Value::Bool(false));
        assert_eq!(result.next(), None);
    }

    #[tokio::test]
    async fn punycode_decode() {
        let input = Box::new(
            vec![(Cow::from("www.xn--caf-dma.com"), Cow::Owned(Map::default()))].into_iter(),
        );
        let punycode = Preprocessor::Punycode(PunycodeConfig {
            encode: false,
            decode: true,
            keep_both: false,
        });

        let mut result = punycode.process(input, &PreprocessorContext::Empty);

        let first = result.next().unwrap();
        assert_eq!(first.0, "www.café.com");
        assert_eq!(*first.1.get("punycode").unwrap(), Value::Bool(false));
        assert_eq!(result.next(), None);
    }

    #[tokio::test]
    async fn punycode_decode_keep_both() {
        let input = Box::new(
            vec![(Cow::from("www.xn--caf-dma.com"), Cow::Owned(Map::default()))].into_iter(),
        );
        let punycode = Preprocessor::Punycode(PunycodeConfig {
            encode: false,
            decode: true,
            keep_both: true,
        });

        let mut result = punycode.process(input, &PreprocessorContext::Empty);

        let first = result.next().unwrap();
        assert_eq!(first.0, "www.café.com");
        assert_eq!(*first.1.get("punycode").unwrap(), Value::Bool(false));
        let second = result.next().unwrap();
        assert_eq!(second.0, "www.xn--caf-dma.com");
        assert_eq!(*second.1.get("punycode").unwrap(), Value::Bool(true));
        assert_eq!(result.next(), None);
    }

    #[tokio::test]
    async fn punycode_encode_decode_keep_both() {
        let input = Box::new(
            vec![
                (Cow::from("www.café.com"), Cow::Owned(Map::default())),
                (Cow::from("www.xn--caf-dma.com"), Cow::Owned(Map::default())),
            ]
            .into_iter(),
        );
        let punycode = Preprocessor::Punycode(PunycodeConfig {
            encode: true,
            decode: true,
            keep_both: true,
        });

        let mut result = punycode.process(input, &PreprocessorContext::Empty);

        let first = result.next().unwrap();
        assert_eq!(first.0, "www.xn--caf-dma.com");
        assert_eq!(*first.1.get("punycode").unwrap(), Value::Bool(true));
        let second = result.next().unwrap();
        assert_eq!(second.0, "www.café.com");
        assert_eq!(*second.1.get("punycode").unwrap(), Value::Bool(false));
        let third = result.next().unwrap();
        assert_eq!(third.0, "www.café.com");
        assert_eq!(*third.1.get("punycode").unwrap(), Value::Bool(false));
        let fourth = result.next().unwrap();
        assert_eq!(fourth.0, "www.xn--caf-dma.com");
        assert_eq!(*fourth.1.get("punycode").unwrap(), Value::Bool(true));
        assert_eq!(result.next(), None);
    }

    #[tokio::test]
    async fn punycode_encode_decode_with_split_target() {
        let input = Box::new(
            vec![
                (Cow::from("www.café.com"), Cow::Owned(Map::default())),
                (Cow::from("www.xn--caf-dma.com"), Cow::Owned(Map::default())),
            ]
            .into_iter(),
        );
        let punycode = Preprocessor::Punycode(PunycodeConfig {
            encode: true,
            decode: true,
            keep_both: true,
        });
        let split_target = Preprocessor::SplitTarget(SplitTargetConfig { ignore_tld: false });

        let mut result = punycode.process(
            split_target.process(input, &PreprocessorContext::Empty),
            &PreprocessorContext::Empty,
        );

        let first = result.next().unwrap();
        assert_eq!(first.0, "www");
        assert_eq!(*first.1.get("punycode").unwrap(), Value::Bool(false));
        let second = result.next().unwrap();
        assert_eq!(second.0, "xn--caf-dma");
        assert_eq!(*second.1.get("punycode").unwrap(), Value::Bool(true));
        let third = result.next().unwrap();
        assert_eq!(third.0, "café");
        assert_eq!(*third.1.get("punycode").unwrap(), Value::Bool(false));
        let fourth = result.next().unwrap();
        assert_eq!(fourth.0, "com");
        assert_eq!(*fourth.1.get("punycode").unwrap(), Value::Bool(false));
        let fifth = result.next().unwrap();
        assert_eq!(fifth.0, "www");
        assert_eq!(*fifth.1.get("punycode").unwrap(), Value::Bool(false));
        let sixth = result.next().unwrap();
        assert_eq!(sixth.0, "café");
        assert_eq!(*sixth.1.get("punycode").unwrap(), Value::Bool(false));
        let seventh = result.next().unwrap();
        assert_eq!(seventh.0, "xn--caf-dma");
        assert_eq!(*seventh.1.get("punycode").unwrap(), Value::Bool(true));
        let eighth = result.next().unwrap();
        assert_eq!(eighth.0, "com");
        assert_eq!(*eighth.1.get("punycode").unwrap(), Value::Bool(false));
        assert_eq!(result.next(), None);
    }
}
