//! Resource estimation for collaborative intelligence
//!
//! This module provides resource estimation capabilities for execution graphs.
//! It analyzes graph structure, identifies parallelization opportunities, and
//! estimates total resource requirements and execution duration.
//!
//! ## Deep Debt Principles
//!
//! - **No Hardcoding**: Estimation based on actual requirements, no magic numbers
//! - **Capability-Based**: Uses system capabilities for realistic estimates
//! - **Self-Knowledge**: Each node provides its own requirements
//! - **Runtime Discovery**: Queries real system state for accurate estimates
//! - **Safe Rust**: All algorithms in safe Rust, no unsafe blocks

mod estimator;
mod types;

#[cfg(test)]
mod tests;

pub use estimator::ResourceEstimator;
pub use types::{EstimationError, NodeEstimate, ResourceEstimate};
