// SPDX-License-Identifier: AGPL-3.0-or-later
//! ToadStool client module - re-exports for public API

pub mod builders;
pub mod config;
#[cfg(feature = "runtime")]
pub mod core;
pub mod error;
pub mod types;
pub mod workload;

#[cfg(test)]
mod core_tests;

pub use config::{AuthConfig, ClientConfig};
#[cfg(feature = "runtime")]
pub use core::{ToadStoolClient, execution_submit_method};
pub use error::{ClientError, ClientResult};
pub use types::{
    ClusterStatus, ExecutionInfo, ExecutionMetrics, ExecutionOutput, ExecutionStatus, JobPriority,
    ResourceRequirements, ToadStoolEvent, WorkloadSubmission, WorkloadType,
};

pub use builders::{
    ContainerWorkloadBuilder, NativeWorkloadBuilder, PythonWorkloadBuilder, WasmWorkloadBuilder,
};
