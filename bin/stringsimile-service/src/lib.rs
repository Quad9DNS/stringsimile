use stringsimile_matcher::Error;

mod cli;
pub mod config;
mod error;
pub mod field_access;
pub mod inputs;
mod metrics;
pub mod metrics_exporters;
pub mod outputs;
mod processor;
pub mod service;
mod signal;
mod system_metrics;

/// Type alias for generic result.
pub type Result<T> = std::result::Result<T, Error>;
