//! Utility module: errors, logging, metrics, and serde helpers.

pub mod errors;
pub mod metrics;
pub mod logging;
pub mod serde_helpers;

pub use errors::{BlockchainError, Result};
pub use metrics::MetricsRegistry;
pub use logging::{init_logging, log_info, log_warn, log_error};
