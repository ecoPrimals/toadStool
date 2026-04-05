// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::missing_errors_doc
)]

//! `ToadStool` monitoring component
//!
//! Cross-platform resource monitoring with configurable granularity.

// Module declarations
pub mod platform;
pub mod process;
pub mod thresholds;
pub mod types;

mod collection;
mod metric_types;
mod reporting;

// Re-export types for backward compatibility
pub use metric_types::SystemResourceMonitor;
pub use types::{MonitoringConfig, MonitoringGranularity, ResourceMonitorError, ThresholdAction};

#[cfg(test)]
mod tests;
