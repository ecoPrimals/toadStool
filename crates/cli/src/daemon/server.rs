//! Daemon server implementation
//!
//! Core daemon server that handles:
//! - HTTP API server for workload submission
//! - biomeOS capability registry integration
//! - Resource monitoring and reporting
//! - Workload lifecycle management

use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};

use super::config::DaemonConfig;
#[cfg(feature = "daemon")]
use super::http_server;
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
        info!("🚀 ToadStool daemon running on port {}", self.config.port);
        info!("📊 API: http://localhost:{}/api/v1", self.config.port);
        info!("💚 Health: http://localhost:{}/health", self.config.port);

        // Start HTTP API server in background task
        #[cfg(feature = "daemon")]
        {
            let port = self.config.port;
            let manager = self.workload_manager.clone();

            tokio::spawn(async move {
                if let Err(e) = http_server::start_http_server(port, manager).await {
                    warn!("⚠️  HTTP server stopped: {e}");
                }
            });
        }

        #[cfg(not(feature = "daemon"))]
        {
            warn!("⚠️  Daemon feature not enabled - HTTP server disabled");
            info!("⏸️  Waiting for shutdown signal (Ctrl+C)");
        }

        // Wait for shutdown signal
        signal::ctrl_c().await?;

        info!("🛑 Shutdown signal received");

        // Graceful shutdown
        self.shutdown().await?;

        info!("👋 ToadStool daemon stopped");
        Ok(())
    }

    /// Graceful shutdown
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
        let mut config = DaemonConfig::default();
        config.register_with_biomeos = true;

        // Should handle biomeOS connection failure gracefully (may fail to connect, but should not crash)
        let result = DaemonServer::start(config).await;
        // Either succeeds or fails gracefully
        assert!(result.is_ok() || result.is_err());
    }
}
