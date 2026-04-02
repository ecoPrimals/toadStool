// SPDX-License-Identifier: AGPL-3.0-only
//! ToadStool Daemon Mode
//!
//! **Like the fungus: Same organism, different forms**
//!
//! - **CLI Mode** (fruiting body): Specialized, project-specific execution
//! - **Daemon Mode** (mycelium): Network-wide, resource-sharing compute service
//!
//! ## Architecture
//!
//! The daemon mode transforms ToadStool from a CLI tool into an ecosystem
//! workload execution service using JSON-RPC over Unix domain sockets per
//! wateringHole standards:
//!
//! - **JSON-RPC Server**: Accept workload requests from other primals via UDS
//! - **Capability Registry**: Auto-register capabilities, report resources, heartbeat
//! - **Workload Manager**: Queue, execute, and monitor workloads
//! - **Resource Monitor**: Track CPU, memory, GPU, storage and report to registry
//! - **Infant Discovery**: Discover security and coordination providers at runtime by capability
//!
//! ## Usage
//!
//! ```bash
//! # Start as server (UniBin standard)
//! toadstool server --register
//!
//! # Start with optional TCP listener for cross-host access
//! toadstool server --port 8085
//! ```
//!
//! ## Infant Discovery
//!
//! The daemon starts with ZERO knowledge and discovers everything at runtime:
//!
//! 1. Load self-knowledge (socket paths, resources)
//! 2. Connect to capability registry (if --register)
//! 3. Register capabilities (Compute, GPU dispatch, Shader dispatch)
//! 4. Discover security provider by capability
//! 5. Discover coordination provider by capability
//! 6. Start JSON-RPC server on Unix socket
//! 7. Begin heartbeat reporting

mod api_types;
mod config;
mod jsonrpc_server;
#[cfg(feature = "nautilus")]
mod nautilus_handlers;
mod routes;
mod server;
mod workload_manager;

pub use api_types::*;
pub use config::DaemonConfig;
pub use server::DaemonServer;
pub use workload_manager::WorkloadManager;

#[cfg(test)]
mod tests {
    #[test]
    fn test_daemon_module_exports() {
        use crate::daemon::{DaemonConfig, WorkloadManager};
        let _ = std::mem::size_of::<DaemonConfig>();
        let _ = std::mem::size_of::<WorkloadManager>();
    }
}
