// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![expect(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::missing_errors_doc,
    reason = "metrics casts are bounds-checked; error docs tracked in NEXT_STEPS"
)]

//! `ToadStool` monitoring component
//!
//! Cross-platform resource monitoring with configurable granularity.

// Module declarations — types always available (WASM-safe)
pub mod thresholds;
pub mod types;

mod metric_types;

#[cfg(feature = "runtime")]
pub mod platform;
#[cfg(feature = "runtime")]
pub mod process;

#[cfg(feature = "runtime")]
mod collection;
#[cfg(feature = "runtime")]
mod reporting;

// Re-export types for backward compatibility
pub use metric_types::SystemResourceMonitor;
pub use types::{MonitoringConfig, MonitoringGranularity, ResourceMonitorError, ThresholdAction};

#[cfg(all(test, feature = "runtime"))]
mod tests;
