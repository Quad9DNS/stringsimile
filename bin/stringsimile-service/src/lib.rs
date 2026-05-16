use stringsimile_matcher::Error;

mod cli;
pub mod config;
mod error;
pub mod field_access;
pub mod inputs;
mod message;
mod metrics;
pub mod metrics_exporters;
pub mod outputs;
pub mod processor;
pub mod service;
mod signal;
mod system_metrics;

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL_ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// Type alias for generic result.
pub type Result<T> = std::result::Result<T, Error>;
