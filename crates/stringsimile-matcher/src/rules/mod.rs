//! Collection of rule implementations

#[cfg(feature = "rules-confusables")]
pub mod confusables;
#[cfg(feature = "rules-damerau-levenshtein")]
pub mod damerau_levenshtein;
#[cfg(feature = "rules-jaro")]
pub mod jaro;
#[cfg(feature = "rules-levenshtein")]
pub mod levenshtein;
