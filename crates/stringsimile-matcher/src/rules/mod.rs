//! Collection of rule implementations

#[cfg(feature = "rules-confusables")]
pub mod confusables;
#[cfg(feature = "rules-damerau-levenshtein")]
pub mod damerau_levenshtein;
#[cfg(feature = "rules-hamming")]
pub mod hamming;
#[cfg(feature = "rules-jaro")]
pub mod jaro;
#[cfg(feature = "rules-jaro-winkler")]
pub mod jaro_winkler;
#[cfg(feature = "rules-levenshtein")]
pub mod levenshtein;
#[cfg(feature = "rules-metaphone")]
pub mod metaphone;
#[cfg(feature = "rules-nysiis")]
pub mod nysiis;
#[cfg(feature = "rules-soundex")]
pub mod soundex;
