//! ToadStool client module - re-exports for public API

pub mod builders;
pub mod config;
pub mod core;
pub mod error;
pub mod types;
pub mod workload;

// Re-export main types for convenience
pub use config::{AuthConfig, ClientConfig};
pub use core::ToadStoolClient;
pub use error::{ClientError, ClientResult};
pub use types::{
    ClusterStatus, ExecutionInfo, ExecutionMetrics, ExecutionOutput, ExecutionStatus, JobPriority,
    ResourceRequirements, ToadStoolEvent, WorkloadSubmission, WorkloadType,
};

// Re-export builders
pub use builders::{
    ContainerWorkloadBuilder, NativeWorkloadBuilder, PythonWorkloadBuilder, WasmWorkloadBuilder,
};
