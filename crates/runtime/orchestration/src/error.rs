//! Orchestration error types

use thiserror::Error;

/// Errors that can occur during workload orchestration
#[derive(Debug, Error)]
pub enum OrchestrationError {
    #[error("No substrates available")]
    NoSubstrates,

    #[error("All substrates failed")]
    AllSubstratesFailed,

    #[error("Operation count must be > 0")]
    InvalidOperationCount,

    #[error("Substrate error: {0}")]
    Substrate(String),
}
