// SPDX-License-Identifier: AGPL-3.0-only
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(test, allow(deprecated))]
#![allow(
    clippy::cast_lossless,
    clippy::doc_comment_double_space_linebreaks,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::manual_is_variant_and,
    clippy::manual_let_else,
    clippy::manual_midpoint,
    clippy::map_unwrap_or,
    clippy::match_same_arms,
    clippy::must_use_candidate,
    clippy::needless_continue,
    clippy::needless_pass_by_value,
    clippy::redundant_closure_for_method_calls,
    clippy::ref_option,
    clippy::return_self_not_must_use,
    clippy::self_only_used_in_recursion,
    clippy::similar_names,
    clippy::struct_field_names,
    clippy::unnecessary_debug_formatting,
    clippy::unnecessary_wraps,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_self,
    clippy::used_underscore_binding,
    clippy::float_cmp,
    clippy::no_effect_underscore_binding,
    clippy::struct_excessive_bools,
    clippy::default_trait_access
)]

//! # `ToadStool` Server Library
//!
//! A comprehensive server library for building `ToadStool` universal compute servers
//! that can accept and execute workloads across multiple runtime engines.
//!
//! ## Features
//!
//! - **JSON-RPC Server**: JSON-RPC 2.0 workload submission and status monitoring
//! - **WebSocket Server**: Real-time event streaming and notifications  
//! - **Runtime Engine Integration**: Support for Native, WASM, Container, Python, GPU runtimes
//! - **Load Balancing**: Intelligent workload distribution across available resources
//! - **Resource Management**: CPU, memory, storage, and GPU resource tracking
//! - **Authentication & Authorization**: Configurable security policies
//! - **Ecosystem Integration**: Integration with Songbird, `BearDog`, `NestGate`
//! - **UniBin Support**: Main server function for UniBin integration
//!
//! ## Quick Start
//!
//! ```ignore
//! use toadstool_server::{ToadStoolServer, ServerConfig};
//! use toadstool_runtime_native::NativeRuntimeEngine;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create server configuration
//!     let config = ServerConfig::default()
//!         .bind_address(format!("0.0.0.0:{}", toadstool_config::ports::server_port()))
//!         .enable_api(true);
//!     
//!     // Create server instance
//!     let mut server = ToadStoolServer::new(config).await?;
//!     
//!     // Register runtime engines
//!     server.register_runtime_engine("native", Box::new(NativeRuntimeEngine::new())).await?;
//!     
//!     // Start the server
//!     server.start().await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## UniBin Integration
//!
//! For UniBin architecture, use the `run_server_main()` function:
//!
//! ```ignore
//! use toadstool_server::run_server_main;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     run_server_main().await
//! }
//! ```

// Re-export public types
pub use config::{
    AuthenticationConfig, HealthCheckConfig, LoggingConfig, PrimalCapabilitiesConfig,
    RateLimitingConfig, ServerConfig,
};
pub use errors::{ServerError, ServerResult};
pub use state::{ActiveExecution, ClientInfo, ServerEvent, ServerState, ServerStatistics};

// Re-export server functions for daemon
#[deprecated(
    since = "2.2.0",
    note = "Use pure_jsonrpc::JsonRpcHandler — no TCP hardcoding"
)]
// Re-export deprecated tarpc types for backward compatibility
#[allow(deprecated)]
pub use tarpc_server::{StandaloneExecutor, ToadStoolTarpcServer, WorkloadExecutor};

// EVOLVED: TestExecutor isolated to testing (deep debt principle)
// Backward compatibility alias for test code
#[cfg(test)]
#[deprecated(since = "2.2.0", note = "Use StandaloneExecutor instead")]
pub use tarpc_server::TestExecutor;

// ⚠️ IMPORTANT: Protocol Priority (wateringHole Standard)
// 1. PRIMARY: JSON-RPC 2.0 over Unix sockets (universal, language-agnostic)
// 2. OPTIONAL: tarpc over Unix sockets (binary RPC for performance-critical paths)
// 3. DEPRECATED: HTTP/TCP (use Songbird for HTTP/TLS)
//
// Per PRIMAL_IPC_PROTOCOL.md and UNIVERSAL_IPC_STANDARD_V3.md:
// JSON-RPC 2.0 is the REQUIRED protocol for inter-primal communication.
// tarpc is OPTIONAL for internal high-performance paths.
//
// See pure_jsonrpc::connection::serve_unix() and tarpc_server::serve_unix()
// for correct implementations.

// EVOLVED: Test exports properly isolated
#[cfg(test)]
pub use mocks::{MockResourceMonitor, MockSystemResourcesWithUsage};

// RESOLVED (S155b): SIGSEGV on process exit (Vulkan+Nvidia+Linux)
// Root cause: wgpu adapter drops during process exit segfault on NVIDIA proprietary
// Vulkan driver. See gfx-rs/wgpu#4650, #8365.
// Fix: All wgpu-touching tests guard on `gpu_guards::is_wgpu_safe()` and skip on
// NVIDIA proprietary. Combined with `current_thread` tokio flavor to avoid drop races.

// Module declarations
pub mod background;
pub mod capabilities; // Self-knowledge & peer discovery
pub mod config;
pub mod coordinator_executor;
pub mod cross_gate; // Cross-gate compute delegation (job routing across mesh)
pub mod errors;
pub mod gpu_job_queue; // GPU compute job queue (compute.submit/status/result/cancel/list)
pub mod gpu_system; // GPU system query helpers (query_gpu_devices, query_gpu_memory)

// Graph types for collaborative intelligence - modularized for code size compliance
pub mod graph_edge;
pub mod graph_errors;
pub mod graph_node;
pub mod graph_types; // Main graph types (ExecutionGraph, builders)

// manual_jsonrpc: REMOVED S94 — fully replaced by pure_jsonrpc
// handlers: REMOVED — HTTP REST handlers are songBird's domain; use pure_jsonrpc

// ✅ EVOLVED: Mocks isolated to testing (deep debt principle)
#[cfg(test)]
pub mod mocks;

pub(crate) mod coral_reef_client; // Internal: used by dispatch for coralReef coordination
pub mod glowplug_client; // coral-ember VFIO passthrough IPC client

// ✅ CANONICAL: JSON-RPC 2.0 (SemanticMethodRegistry, proper error types)
// lifecycle: REMOVED — HTTP lifecycle is songBird's domain; use pure_jsonrpc
pub mod pure_jsonrpc;
pub mod resource_estimator;
pub mod resource_optimizer;
pub mod resource_validator;
// routes: REMOVED — HTTP routes are songBird's domain; use pure_jsonrpc
pub mod rpc_types; // Pure RPC types (no HTTP deps)
// server: REMOVED — axum HTTP server is songBird's domain; use pure_jsonrpc
pub mod state;
pub mod tarpc_server;
pub mod unibin; // UniBin server entry point (shared between binaries)

// Re-export background services for tests
pub use background::start_background_services;

// Re-export UniBin entry point for external use
pub use unibin::run_server_main;

// Re-export pure RPC types (deep debt solution)
pub use rpc_types::semantic_methods;
pub use rpc_types::{
    AvailableResources, ComputeCapabilities, ComputeUnit, ExecutionMetrics, HealthStatus,
    ResourceRequirements, TarpcWorkloadSubmission, ToadStoolComputeRpc, ToadStoolComputeRpcClient,
    WorkloadPriority, WorkloadResult, WorkloadStatus, WorkloadSubmission,
};

// Re-export coordinator executor
pub use coordinator_executor::CoordinatorExecutor;

// ManualJsonRpcServer: REMOVED S94 — use pure_jsonrpc::JsonRpcHandler

// Re-export collaborative intelligence types
pub use graph_types::{
    EdgeType, ExecutionGraph, GraphEdge, GraphNode, GraphValidationError, NodeResourceRequirements,
};
pub use resource_estimator::{EstimationError, NodeEstimate, ResourceEstimate, ResourceEstimator};
pub use resource_optimizer::{
    Bottleneck, ImprovementEstimate, Opportunity, OptimizationSuggestions, ResourceOptimizer,
};
pub use resource_validator::{
    AvailabilityResult, ResourceGap, ResourceValidator, SystemCapabilities, ValidationError,
};
