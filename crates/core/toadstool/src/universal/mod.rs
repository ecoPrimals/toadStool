// SPDX-License-Identifier: AGPL-3.0-only
//! Universal compute platform modules
//!
//! Smart refactoring: Domain-driven module organization.
//! Types are logically grouped by functional responsibility.

// Core types
pub mod requests;
pub mod types;

// Job management
pub mod jobs;

// Resource management
pub mod resources;

// Scheduling
pub mod scheduler;

// Streaming dispatch (absorbed from hotSpring v0.6.24)
pub mod streaming_dispatch;

// Pipeline DAG (absorbed from neuralSpring S134)
pub mod pipeline_graph;

// Platform
pub mod platform;

// Primal system
pub mod provider;
pub mod registry;
pub mod traits;

// Re-exports for backward compatibility
pub use jobs::{JobPriority, UniversalJob, UniversalJobType};
pub use pipeline_graph::{
    PipelineExecution, PipelineGraph, StageNode, StageOutput, StageResult, Substrate,
};
pub use platform::{
    get_platform_status, init_with_runtime_engines, PlatformStatus, UniversalComputePlatform,
    UniversalPlatformConfig,
};
pub use provider::ToadStoolPrimalProvider;
pub use registry::UniversalPrimalRegistry;
pub use requests::{PrimalEndpoints, PrimalRequest, PrimalResponse, ResponseStatus};
pub use resources::{ResourceAllocation, ResourceCoordinator, UniversalSystemResources};
pub use scheduler::UniversalScheduler;
pub use streaming_dispatch::{
    DispatchMode, DispatchStats, ProgressCallback, StageProgress, StreamingDispatchContext,
};
pub use traits::UniversalPrimalProvider;
pub use types::{
    NetworkLocation, PrimalCapability, PrimalContext, PrimalHealth, PrimalType, SecurityLevel,
};
