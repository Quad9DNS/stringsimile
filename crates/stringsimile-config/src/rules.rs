//! Configuration for rules
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use stringsimile_matcher::{
    Error,
    rule::{GenericMatcherRule, IntoGenericMatcherRule},
    rules::{
        bitflip::BitflipRule,
        confusables::ConfusablesRule,
        damerau_levenshtein::DamerauLevenshteinRule,
        hamming::HammingRule,
        jaro::JaroRule,
        jaro_winkler::JaroWinklerRule,
        levenshtein::LevenshteinRule,
        match_rating::MatchRatingRule,
        metaphone::{MetaphoneRule, MetaphoneRuleType},
        nysiis::NysiisRule,
        soundex::{SoundexRule, SoundexRuleType},
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "rule_type", rename_all = "snake_case", content = "values")]
/// Configuration for rules
pub enum RuleConfig {
    /// Configuration for Levenshtein rule
    Levenshtein(LevenshteinConfig),
    /// Configuration for Hamming rule
    Hamming(HammingConfig),
    /// Configuration for Confusables rule
    Confusables,
    /// Configuration for Damerau Levenshtein rule
    DamerauLevenshtein(DamerauLevenshteinConfig),
    /// Configuration for Jaro rule
    Jaro(JaroConfig),
    /// Configuration for Jaro-Winkler rule
    JaroWinkler(JaroWinklerConfig),
    /// Configuration for Soundex rule
    Soundex(SoundexConfig),
    /// Configuration for Metaphone rule
    Metaphone(MetaphoneConfig),
    /// Configuration for NYSIIS rule
    Nysiis(NysiisConfig),
    /// Configuration for Match Rating rule
    MatchRating,
    /// Configuration for Bitflip rule
    Bitflip(Option<BitflipConfig>),
}

/// Errors for rule configuration
#[derive(Debug, Clone, Snafu)]
pub enum RuleConfigError {
    /// Jaro rule configuration error
    #[snafu(display(
        "Invalid match percent threshold for Jaro rule. It has to be a decimal value between 0 and 1. Found: {}",
        input_value
    ))]
    JaroConfigThresholdError {
        /// Value that was provided to the rule
        input_value: f64,
    },

    /// Jaro Winkler rule configuration error
    #[snafu(display(
        "Invalid match percent threshold for Jaro-Winkler rule. It has to be a decimal value between 0 and 1. Found: {}",
        input_value
    ))]
    JaroWinklerConfigThresholdError {
        /// Value that was provided to the rule
        input_value: f64,
    },

    /// Soundex rule configuration error
    #[snafu(display(
        "Invalid minimum similarity value for Soundex rule. Maximum allowed value normal soundex is 4. Found: {}",
        input_value
    ))]
    SoundexConfigSimilarityError {
        /// Value that was provided to the rule
        input_value: usize,
    },

    /// Metaphone rule configuration error
    #[snafu(display(
        "Invalid target string for Metaphone rule. The string_match must be ASCII for metaphone rule.",
    ))]
    MetaphoneNonAsciiTargetError,

    /// Bitflip rule configuration error
    #[snafu(display(
        "Invalid char subset for Bitflip rule. Only ASCII chars are allowed. Non-ASCII char at byte index {} in \"{}\"",
        input_str,
        index
    ))]
    BitflipNonAsciiCharError {
        /// Value that was provided in custom_char_subset field of the rule.
        input_str: String,
        /// Position at which non-ASCII char was found.
        index: usize,
    },
}

impl RuleConfig {
    /// Generates a rule implementation from this config
    ///
    /// `ignore_mismatch_metadata` flag can be enabled to potentially speed up some rules, at the
    /// cost of missing metadata for mismatches.
    pub fn build(
        &self,
        target_str: &str,
        ignore_mismatch_metadata: bool,
    ) -> Result<Box<dyn GenericMatcherRule>, Error> {
        Ok(match self {
            RuleConfig::Levenshtein(levenshtein_config) => Box::new(
                levenshtein_config
                    .build(ignore_mismatch_metadata)?
                    .into_generic_matcher(),
            ),
            RuleConfig::Hamming(hamming_config) => {
                Box::new(hamming_config.build()?.into_generic_matcher())
            }
            RuleConfig::Confusables => Box::new(ConfusablesConfig.build()?.into_generic_matcher()),
            RuleConfig::Jaro(jaro_config) => Box::new(jaro_config.build()?.into_generic_matcher()),
            RuleConfig::JaroWinkler(jaro_winkler_config) => {
                Box::new(jaro_winkler_config.build()?.into_generic_matcher())
            }
            RuleConfig::DamerauLevenshtein(damerau_levenshtein_config) => {
                Box::new(damerau_levenshtein_config.build(ignore_mismatch_metadata)?)
            }
            RuleConfig::Soundex(soundex_config) => {
                Box::new(soundex_config.build(target_str)?.into_generic_matcher())
            }
            RuleConfig::Metaphone(metaphone_config) => {
                Box::new(metaphone_config.build(target_str)?.into_generic_matcher())
            }
            RuleConfig::Nysiis(nysiis_config) => {
                Box::new(nysiis_config.build(target_str)?.into_generic_matcher())
            }
            RuleConfig::MatchRating => {
                Box::new(MatchRatingConfig.build(target_str)?.into_generic_matcher())
            }
            RuleConfig::Bitflip(bitflip_config) => Box::new(
                bitflip_config
                    .clone()
                    .unwrap_or_default()
                    .build(target_str)?
                    .into_generic_matcher(),
            ),
        })
    }
}

/// Configuration for Levenshtein rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevenshteinConfig {
    /// Maximum distance
    pub maximum_distance: u32,
}

impl LevenshteinConfig {
    fn build(&self, ignore_mismatch_metadata: bool) -> Result<LevenshteinRule, Error> {
        Ok(LevenshteinRule {
            maximum_distance: self.maximum_distance,
            ignore_mismatch_metadata,
        })
    }
}

/// Configuration for Levenshtein rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HammingConfig {
    /// Maximum distance
    pub maximum_distance: u32,
}

impl HammingConfig {
    fn build(&self) -> Result<HammingRule, Error> {
        Ok(HammingRule {
            maximum_distance: self.maximum_distance,
        })
    }
}

/// Configuration for Confusables rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfusablesConfig;

impl ConfusablesConfig {
    fn build(&self) -> Result<ConfusablesRule, Error> {
        Ok(ConfusablesRule)
    }
}

/// Configuration for Damerau Levenshtein rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamerauLevenshteinConfig {
    /// Maximum distance
    pub maximum_distance: u32,
}

impl DamerauLevenshteinConfig {
    fn build(&self, ignore_mismatch_metadata: bool) -> Result<DamerauLevenshteinRule, Error> {
        Ok(DamerauLevenshteinRule {
            maximum_distance: self.maximum_distance,
            ignore_mismatch_metadata,
        })
    }
}

/// Configuration for Jaro rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JaroConfig {
    /// Maximum distance
    pub match_percent_threshold: f64,
}

impl JaroConfig {
    fn build(&self) -> Result<JaroRule, Error> {
        if self.match_percent_threshold < 0.0 || self.match_percent_threshold > 1.0 {
            return Err(RuleConfigError::JaroConfigThresholdError {
                input_value: self.match_percent_threshold,
            }
            .into());
        }

        Ok(JaroRule {
            match_percent: self.match_percent_threshold,
        })
    }
}

/// Configuration for Jaro-Winkler rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JaroWinklerConfig {
    /// Maximum distance
    pub match_percent_threshold: f64,
}

impl JaroWinklerConfig {
    fn build(&self) -> Result<JaroWinklerRule, Error> {
        if self.match_percent_threshold < 0.0 || self.match_percent_threshold > 1.0 {
            return Err(RuleConfigError::JaroWinklerConfigThresholdError {
                input_value: self.match_percent_threshold,
            }
            .into());
        }

        Ok(JaroWinklerRule {
            match_percent: self.match_percent_threshold,
        })
    }
}

/// Configuration for Soundex rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundexConfig {
    /// Minimum similarity
    pub minimum_similarity: usize,
    /// Type of soundex (normal or refined)
    #[serde(default)]
    pub soundex_type: SoundexRuleType,
}

impl SoundexConfig {
    fn build(&self, target_str: &str) -> Result<SoundexRule, Error> {
        if self.soundex_type == SoundexRuleType::Normal && self.minimum_similarity > 4 {
            return Err(RuleConfigError::SoundexConfigSimilarityError {
                input_value: self.minimum_similarity,
            }
            .into());
        }

        Ok(SoundexRule::new(
            self.soundex_type,
            self.minimum_similarity,
            target_str,
        ))
    }
}

/// Configuration for Metaphone rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaphoneConfig {
    /// Max length of the generated Metaphone code
    #[serde(default = "default_metaphone_max_code_length")]
    pub max_code_length: Option<usize>,
    /// Type of soundex (normal or refined)
    #[serde(default)]
    pub metaphone_type: MetaphoneRuleType,
}

const fn default_metaphone_max_code_length() -> Option<usize> {
    Some(4)
}

impl MetaphoneConfig {
    fn build(&self, target_str: &str) -> Result<MetaphoneRule, Error> {
        if !target_str.is_ascii() {
            return Err(Box::new(RuleConfigError::MetaphoneNonAsciiTargetError));
        }
        Ok(MetaphoneRule::new(
            self.metaphone_type,
            self.max_code_length,
            target_str,
        ))
    }
}

impl Default for MetaphoneConfig {
    fn default() -> Self {
        Self {
            max_code_length: default_metaphone_max_code_length(),
            metaphone_type: Default::default(),
        }
    }
}

/// Configuration for Nysiis rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NysiisConfig {
    /// Strict mode can be disabled to allow codes over 6 characters in length
    #[serde(default = "default_nysiis_strict_mode")]
    pub strict: bool,
}

const fn default_nysiis_strict_mode() -> bool {
    true
}

impl NysiisConfig {
    fn build(&self, target_str: &str) -> Result<NysiisRule, Error> {
        Ok(NysiisRule::new(self.strict, target_str))
    }
}

impl Default for NysiisConfig {
    fn default() -> Self {
        Self {
            strict: default_nysiis_strict_mode(),
        }
    }
}

/// Configuration for Match Rating rule
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatchRatingConfig;

impl MatchRatingConfig {
    fn build(&self, target_str: &str) -> Result<MatchRatingRule, Error> {
        Ok(MatchRatingRule::new(target_str))
    }
}

/// Predefined type of char subset
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BitflipCharSubset {
    /// Chars valid in DNS context (a-z,A-Z,0-9,-,.).
    #[default]
    Dns,
    /// All printable ASCII chars.
    Printable,
    /// Custom array of valid chars.
    Custom,
}

/// Configuration for Biflip rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitflipConfig {
    /// Predefined char subset or "custom" to use [`BitflipConfig::custom_char_subset`].
    #[serde(default)]
    pub char_subset: BitflipCharSubset,
    /// Custom char subset to use (list of valid characters).
    #[serde(default)]
    pub custom_char_subset: String,
    /// Whether matching with bitflipped variants should be case sensitive.
    #[serde(default = "default_bitflip_case_sensitive")]
    pub case_sensitive: bool,
}

const fn default_bitflip_case_sensitive() -> bool {
    true
}

impl BitflipConfig {
    fn build(&self, target_str: &str) -> Result<BitflipRule, Error> {
        match self.char_subset {
            BitflipCharSubset::Dns => Ok(BitflipRule::new_dns(target_str, self.case_sensitive)),
            BitflipCharSubset::Printable => Ok(BitflipRule::new_ascii_printable(
                target_str,
                self.case_sensitive,
            )),
            BitflipCharSubset::Custom => {
                let valid_chars = self
                    .custom_char_subset
                    .chars()
                    .enumerate()
                    .map(|(i, c)| u8::try_from(c).map_err(|_| i))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|index| RuleConfigError::BitflipNonAsciiCharError {
                        input_str: self.custom_char_subset.clone(),
                        index,
                    })?;
                Ok(BitflipRule::new(
                    &valid_chars,
                    target_str,
                    self.case_sensitive,
                ))
            }
        }
    }
}

impl Default for BitflipConfig {
    fn default() -> Self {
        Self {
            char_subset: Default::default(),
            custom_char_subset: Default::default(),
            case_sensitive: default_bitflip_case_sensitive(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_levenshtein() {
        let json = r#"
        {
            "rule_type": "levenshtein",
            "values": {
                "maximum_distance": 3
            }
        }
            "#;

        let RuleConfig::Levenshtein(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Levenshtein config");
        };
        assert_eq!(3, config.maximum_distance);
    }

    #[test]
    fn test_parse_jaro() {
        let json = r#"
        {
            "rule_type": "jaro",
            "values": {
                "match_percent_threshold": 0.4
            }
        }
            "#;

        let RuleConfig::Jaro(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Jaro config");
        };
        assert_eq!(0.4, config.match_percent_threshold);
    }

    #[test]
    fn test_parse_confusables() {
        let json = r#"
        {
            "rule_type": "confusables"
        }
            "#;

        let RuleConfig::Confusables = serde_json::from_str(json).unwrap() else {
            panic!("Expected Confusables config");
        };
    }

    #[test]
    fn test_parse_damerau_levenshtein() {
        let json = r#"
        {
            "rule_type": "damerau_levenshtein",
            "values": {
                "maximum_distance": 3
            }
        }
            "#;

        let RuleConfig::DamerauLevenshtein(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Damera Levenshtein config");
        };
        assert_eq!(3, config.maximum_distance);
    }

    #[test]
    fn test_parse_jaro_winkler() {
        let json = r#"
        {
            "rule_type": "jaro_winkler",
            "values": {
                "match_percent_threshold": 0.4
            }
        }
            "#;

        let RuleConfig::JaroWinkler(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Jaro-Winkler config");
        };
        assert_eq!(0.4, config.match_percent_threshold);
    }

    #[test]
    fn test_parse_hamming() {
        let json = r#"
        {
            "rule_type": "hamming",
            "values": {
                "maximum_distance": 3
            }
        }
            "#;

        let RuleConfig::Hamming(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Hamming config");
        };
        assert_eq!(3, config.maximum_distance);
    }

    #[test]
    fn test_parse_soundex_normal() {
        let json = r#"
        {
            "rule_type": "soundex",
            "values": {
                "minimum_similarity": 3,
                "soundex_type": "normal"
            }
        }
            "#;

        let RuleConfig::Soundex(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Soundex config");
        };
        assert_eq!(3, config.minimum_similarity);
        assert_eq!(SoundexRuleType::Normal, config.soundex_type);
    }

    #[test]
    fn test_parse_soundex_normal_default() {
        let json = r#"
        {
            "rule_type": "soundex",
            "values": {
                "minimum_similarity": 3
            }
        }
            "#;

        let RuleConfig::Soundex(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Soundex config");
        };
        assert_eq!(3, config.minimum_similarity);
        assert_eq!(SoundexRuleType::Normal, config.soundex_type);
    }

    #[test]
    fn test_parse_soundex_refined() {
        let json = r#"
        {
            "rule_type": "soundex",
            "values": {
                "minimum_similarity": 3,
                "soundex_type": "refined"
            }
        }
            "#;

        let RuleConfig::Soundex(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Soundex config");
        };
        assert_eq!(3, config.minimum_similarity);
        assert_eq!(SoundexRuleType::Refined, config.soundex_type);
    }

    #[test]
    fn test_parse_metaphone_normal() {
        let json = r#"
        {
            "rule_type": "metaphone",
            "values": {
                "max_code_length": 3,
                "metaphone_type": "normal"
            }
        }
            "#;

        let RuleConfig::Metaphone(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Metaphone config");
        };
        assert_eq!(Some(3), config.max_code_length);
        assert_eq!(MetaphoneRuleType::Normal, config.metaphone_type);
    }

    #[test]
    fn test_parse_metaphone_default() {
        let json = r#"
        {
            "rule_type": "metaphone",
            "values": {}
        }
            "#;

        let RuleConfig::Metaphone(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Metaphone config");
        };
        assert_eq!(Some(4), config.max_code_length);
        assert_eq!(MetaphoneRuleType::Normal, config.metaphone_type);
    }

    #[test]
    fn test_parse_metaphone_null_length() {
        let json = r#"
        {
            "rule_type": "metaphone",
            "values": {
                "max_code_length": null
            }
        }
            "#;

        let RuleConfig::Metaphone(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Metaphone config");
        };
        assert_eq!(None, config.max_code_length);
        assert_eq!(MetaphoneRuleType::Normal, config.metaphone_type);
    }

    #[test]
    fn test_parse_metaphone_double_default_length() {
        let json = r#"
        {
            "rule_type": "metaphone",
            "values": {
                "metaphone_type": "double"
            }
        }
            "#;

        let RuleConfig::Metaphone(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Metaphone config");
        };
        assert_eq!(default_metaphone_max_code_length(), config.max_code_length);
        assert_eq!(MetaphoneRuleType::Double, config.metaphone_type);
    }

    #[test]
    fn test_parse_metaphone_double() {
        let json = r#"
        {
            "rule_type": "metaphone",
            "values": {
                "max_code_length": 3,
                "metaphone_type": "double"
            }
        }
            "#;

        let RuleConfig::Metaphone(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Metaphone config");
        };
        assert_eq!(Some(3), config.max_code_length);
        assert_eq!(MetaphoneRuleType::Double, config.metaphone_type);
    }

    #[test]
    fn test_parse_nysiis_non_strict() {
        let json = r#"
        {
            "rule_type": "nysiis",
            "values": {
                "strict": false
            }
        }
            "#;

        let RuleConfig::Nysiis(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Nysiis config");
        };
        assert!(!config.strict);
    }

    #[test]
    fn test_parse_nysiis_default() {
        let json = r#"
        {
            "rule_type": "nysiis",
            "values": {}
        }
            "#;

        let RuleConfig::Nysiis(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Nysiis config");
        };
        assert!(config.strict);
    }

    #[test]
    fn test_parse_match_rating() {
        let json = r#"
        {
            "rule_type": "match_rating"
        }
            "#;

        let RuleConfig::MatchRating = serde_json::from_str(json).unwrap() else {
            panic!("Expected Match Rating config");
        };
    }

    #[test]
    fn test_parse_bitflip_defaults() {
        let json = r#"
        {
            "rule_type": "bitflip"
        }
            "#;

        let RuleConfig::Bitflip(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Biflip config");
        };
        let config = config.unwrap_or_default();
        assert!(config.case_sensitive);
        assert!(config.custom_char_subset.is_empty());
        assert!(matches!(config.char_subset, BitflipCharSubset::Dns));
    }

    #[test]
    fn test_parse_bitflip_ascii_printable() {
        let json = r#"
        {
            "rule_type": "bitflip",
            "values": {
                "char_subset": "printable",
                "case_sensitive": false
            }
        }
            "#;

        let RuleConfig::Bitflip(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Biflip config");
        };
        let config = config.unwrap_or_default();
        assert!(!config.case_sensitive);
        assert!(config.custom_char_subset.is_empty());
        assert!(matches!(config.char_subset, BitflipCharSubset::Printable));
    }

    #[test]
    fn test_parse_bitflip_ascii_custom() {
        let json = r#"
        {
            "rule_type": "bitflip",
            "values": {
                "char_subset": "custom",
                "custom_char_subset": "abcdefhijkl",
                "case_sensitive": true
            }
        }
            "#;

        let RuleConfig::Bitflip(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Biflip config");
        };
        let config = config.unwrap_or_default();
        assert!(config.case_sensitive);
        assert_eq!(config.custom_char_subset, "abcdefhijkl");
        assert!(matches!(config.char_subset, BitflipCharSubset::Custom));
    }

    #[test]
    fn test_parse_bitflip_non_ascii_chars() {
        let json = r#"
        {
            "rule_type": "bitflip",
            "values": {
                "char_subset": "custom",
                "custom_char_subset": "abcčćddžđ",
                "case_sensitive": true
            }
        }
            "#;

        let RuleConfig::Bitflip(config) = serde_json::from_str(json).unwrap() else {
            panic!("Expected Biflip config");
        };
        let config = config.unwrap_or_default();
        let err = config
            .build("test")
            .expect_err("Expected Bitflip rule build to faile due to non ascii custom char subset");
        let RuleConfigError::BitflipNonAsciiCharError { input_str, index } =
            err.downcast_ref().unwrap()
        else {
            panic!("Expected bitflip rule build to fail");
        };
        assert_eq!(input_str, "abcčćddžđ");
        assert_eq!(*index, 3);
    }
}
