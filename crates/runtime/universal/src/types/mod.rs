// SPDX-License-Identifier: AGPL-3.0-or-later
//! Core types for universal compute
//!
//! This module defines the fundamental abstractions that unify CPU, GPU,
//! and neuromorphic processing under a single interface.

mod capabilities;
mod compute_unit;
mod compute_unit_dispatch;
mod error;
mod output;
mod workload;

pub use capabilities::{
    Capabilities, ComputeUnitType, DataType, ExecutionModel, LatencyProfile, OperationType,
    Parallelism, PowerProfile,
};
pub use compute_unit::ComputeUnit;
pub use compute_unit_dispatch::ComputeUnitDispatch;
pub use error::ComputeError;
pub use output::{Output, OutputMetadata};
pub use workload::{ParamValue, Workload, WorkloadBuilder, WorkloadData, WorkloadParams};

#[cfg(test)]
mod tests;
