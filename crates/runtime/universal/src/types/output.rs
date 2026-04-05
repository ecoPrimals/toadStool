// SPDX-License-Identifier: AGPL-3.0-or-later
//! Execution output and metadata types.

use super::capabilities::ComputeUnitType;
use super::workload::WorkloadData;

/// Execution output
#[derive(Debug, Clone)]
pub struct Output {
    /// Result data
    pub data: WorkloadData,

    /// Execution metadata
    pub metadata: OutputMetadata,
}

/// Execution metadata
#[derive(Debug, Clone)]
pub struct OutputMetadata {
    /// Which unit executed this
    pub unit_name: String,

    /// Unit type
    pub unit_type: ComputeUnitType,

    /// Actual execution time
    pub duration: std::time::Duration,

    /// Power consumed (if measurable)
    pub power_consumed_mw: Option<f64>,
}
