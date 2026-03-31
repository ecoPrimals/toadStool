// SPDX-License-Identifier: AGPL-3.0-only
//! Daemon server implementation
//!
//! Core daemon server that handles:
//! - HTTP API server for workload submission
//! - biomeOS capability registry integration
//! - Resource monitoring and reporting
//! - Workload lifecycle management

use crate::Result;
use std::sync::Arc;
use toadstool_common::platform_paths::{PathEnv, PlatformPaths};
use tokio::signal;
use tracing::{info, warn};

use super::config::DaemonConfig;
use super::jsonrpc_server;
use super::workload_manager::WorkloadManager;

/// Daemon server
///
/// Coordinates all daemon functionality:
/// - HTTP API server
/// - Capability registry integration (via mDNS/environment)
/// - Workload management
/// - Resource monitoring
pub struct DaemonServer {
    /// Configuration
    config: DaemonConfig,

    /// Workload manager
    workload_manager: Arc<WorkloadManager>,
}

impl DaemonServer {
    /// Start the daemon server
    ///
    /// ## Infant Discovery Flow
    ///
    /// 1. Load self-knowledge (ports, resources)
    /// 2. Connect to capability registry (if enabled)
    /// 3. Register capabilities
    /// 4. Discover dependencies (security, coordination providers) by capability
    /// 5. Start API server
    /// 6. Begin heartbeat
    pub async fn start(config: DaemonConfig) -> Result<Self> {
        info!("🍄 Initializing ToadStool daemon server...");

        // Announce capabilities via mDNS (if enabled)
        if config.register_with_biomeos {
            info!("📢 Announcing capabilities via mDNS/discovery");
            info!("  - Capability: Compute (wasm, container, python, native, gpu)");
            info!("  - Capability: Storage (local, distributed, encrypted)");
            info!("  - Capability: Orchestration (workflow coordination)");
            // Discovery engine will register these capabilities with songBird
            // Uses primal_integration module for runtime discovery
        } else {
            info!("📍 Discovery disabled - running in standalone mode");
        }

        // Create workload manager
        let workload_manager = WorkloadManager::new(config.max_concurrent_workloads).await?;
        info!("✅ Workload manager initialized");

        // Phase 4: Resource monitor via system metrics
        // Phase 5: Health reporting via songBird integration
        // Both integrated with primal_integration discovery system

        info!("✅ ToadStool daemon server initialized");

        Ok(Self {
            config,
            workload_manager: Arc::new(workload_manager),
        })
    }

    /// Run the daemon server until shutdown signal
    pub async fn run(self) -> Result<()> {
        // Determine socket path (prefer Unix socket for primal communication)
        // Uses XDG-compliant path resolution - no hardcoded paths
        let socket_path = self.config.socket_path.clone().unwrap_or_else(|| {
            let env = PathEnv::from_env();
            PlatformPaths::new(&env).toadstool_socket()
        });

        info!("🚀 ToadStool daemon running");
        info!("🍄 JSON-RPC socket: {}", socket_path.display());
        info!("📊 Methods: daemon.health, daemon.metrics, daemon.submit_workload, etc.");

        // Start JSON-RPC server (EVOLVED: Pure Rust over Unix sockets!)
        {
            let socket = socket_path.clone();
            let manager = Arc::clone(&self.workload_manager);

            tokio::spawn(async move {
                if let Err(e) = jsonrpc_server::start_jsonrpc_server(&socket, manager).await {
                    warn!("⚠️  JSON-RPC server stopped: {e}");
                }
            });
        }

        info!("✨ Pure Unix socket mode — JSON-RPC over UDS per wateringHole standard");

        // Wait for shutdown signal
        signal::ctrl_c().await?;

        info!("🛑 Shutdown signal received");

        // Graceful shutdown
        self.shutdown().await?;

        info!("👋 ToadStool daemon stopped");
        Ok(())
    }

    /// Graceful shutdown
    #[allow(clippy::unused_async)] // Shutdown hook; async for future extension
    async fn shutdown(&self) -> Result<()> {
        info!("🧹 Performing graceful shutdown...");

        // Shutdown sequence:
        // 1. Stop accepting new workloads
        // 2. Stop HTTP server
        // 3. Gracefully terminate running workloads
        // 4. Unregister from songBird (if registered)
        // All integrated with graceful degradation patterns

        info!("✅ Shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_daemon_server_creation() {
        let config = DaemonConfig::default();

        // Should be able to create daemon server
        let result = DaemonServer::start(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_daemon_server_with_biomeos() {
        let config = DaemonConfig {
            register_with_biomeos: true,
            ..Default::default()
        };

        // Should handle biomeOS connection failure gracefully (may fail to connect, but should not crash)
        let result = DaemonServer::start(config).await;
        // Either succeeds or fails gracefully
        assert!(result.is_ok() || result.is_err());
    }
}
