// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::missing_errors_doc,
    reason = "client API: error docs covered in module-level docs"
)]

//! # ToadStool Client Library
//!
//! A comprehensive client library for connecting to ToadStool universal compute servers
//! and submitting workloads for execution.
//!
//! ## Features
//!
//! - **JSON-RPC Client**: JSON-RPC 2.0 workload submission and status monitoring
//! - **Ecosystem Integration**: Direct integration with coordination, security, and storage services
//! - **Load Balancing**: Automatic discovery and load balancing across ToadStool nodes
//! - **Retry Logic**: Configurable retry policies for resilient execution
//! - **Authentication**: Support for API keys, tokens, and ecosystem authentication
//!
//! ## Architecture
//!
//! The client library is built around the [`ToadStoolClient`] struct, which provides
//! a high-level interface for interacting with ToadStool servers. It supports multiple
//! authentication methods, automatic retries, and real-time event streaming.
//!
//! ### Workload Types
//!
//! - [`WorkloadType::Native`] - Execute native binaries
//! - [`WorkloadType::Container`] - Run containerized applications
//! - [`WorkloadType::Wasm`] - Execute WebAssembly modules
//! - [`WorkloadType::Python`] - Run Python scripts
//! - [`WorkloadType::Custom`] - Custom workload types
//!
//! ### Builder Pattern
//!
//! Use the builder pattern for constructing workloads:
//! - [`NativeWorkloadBuilder`] - For native executables
//! - [`ContainerWorkloadBuilder`] - For container images
//! - [`WasmWorkloadBuilder`] - For WebAssembly modules
//! - [`PythonWorkloadBuilder`] - For Python scripts
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use toadstool_client::{ToadStoolClient, WorkloadSubmission, ClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // ✅ MODERN: Capability-based service discovery
//!     // Instead of hardcoding, use service discovery or config constants:
//!     //
//!     // Option 1: Environment-based (development/testing)
//!     let endpoint = std::env::var("TOADSTOOL_SERVER_URL")
//!         .unwrap_or_else(|_| std::env::var("TOADSTOOL_SERVER_URL").unwrap_or_default());
//!     
//!     // Option 2: Production with discovery (see ClientConfig::with_discovery)
//!     let client = ToadStoolClient::new(&endpoint).await?;
//!     
//!     // Submit a native workload
//!     let workload = WorkloadSubmission::native()
//!         .executable("/bin/echo")
//!         .args(vec!["Hello, ToadStool!".to_string()])
//!         .build()?;
//!     
//!     let execution = client.submit_workload(workload).await?;
//!     println!("Submitted execution: {}", execution.execution_id);
//!     
//!     // Wait for completion
//!     let result = client.wait_for_completion(execution.execution_id).await?;
//!     println!("Execution completed: {:?}", result.status);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Authentication
//!
//! The client supports multiple authentication methods:
//!
//! ```rust
//! use toadstool_client::{ClientConfig, AuthConfig};
//!
//! // API Key authentication
//! let config = ClientConfig {
//!     auth: Some(AuthConfig::ApiKey {
//!         key: "your-api-key".to_string(),
//!         header_name: "X-API-Key".to_string(),
//!     }),
//!     ..Default::default()
//! };
//!
//! // Bearer token authentication
//! let config = ClientConfig {
//!     auth: Some(AuthConfig::BearerToken {
//!         token: "your-bearer-token".to_string(),
//!     }),
//!     ..Default::default()
//! };
//! ```

// Internal client module with all implementation
mod client;

#[cfg(feature = "tarpc")]
mod tarpc_client;

// Re-export all public types and functions
pub use client::{
    AuthConfig,
    ClientConfig,
    ClientError,
    ClientResult,
    ClusterStatus,
    ContainerWorkloadBuilder,
    ExecutionInfo,
    ExecutionMetrics,
    ExecutionOutput,
    ExecutionStatus,
    JobPriority,
    NativeWorkloadBuilder,
    PythonWorkloadBuilder,
    ResourceRequirements,
    ToadStoolEvent,
    WasmWorkloadBuilder,
    WorkloadSubmission,
    WorkloadType,
};

#[cfg(feature = "runtime")]
pub use client::{ToadStoolClient, execution_submit_method};

#[cfg(feature = "tarpc")]
pub use tarpc_client::{ClientEndpoint, TarpcClientError, ToadStoolTarpcClient};

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
