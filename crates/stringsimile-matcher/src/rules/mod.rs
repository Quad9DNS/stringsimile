//! Collection of rule implementations

#[cfg(feature = "rules-jaro")]
pub mod jaro;
#[cfg(feature = "rules-levenshtein")]
pub mod levenshtein;
