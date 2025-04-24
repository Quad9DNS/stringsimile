use stringsimile_matcher::rule::Error;

mod cli;
mod config;
mod inputs;
mod outputs;
mod processor;
pub mod service;
mod signal;

/// Type alias for generic result.
pub type Result<T> = std::result::Result<T, Error>;
