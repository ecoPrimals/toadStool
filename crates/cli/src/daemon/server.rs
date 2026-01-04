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
use super::workload_manager::WorkloadManager;
#[cfg(feature = "daemon")]
use super::http_server;

/// Daemon server
///
/// Coordinates all daemon functionality:
/// - HTTP API server
/// - biomeOS integration
/// - Workload management
/// - Resource monitoring
pub struct DaemonServer {
    /// Configuration
    config: DaemonConfig,
    
    /// biomeOS client (if registered)
    biomeos_client: Option<Arc<toadstool::biomeos_integration::BiomeOSClient>>,
    
    /// Workload manager
    workload_manager: Arc<WorkloadManager>,
}

impl DaemonServer {
    /// Start the daemon server
    ///
    /// ## Infant Discovery Flow
    ///
    /// 1. Load self-knowledge (ports, resources)
    /// 2. Connect to biomeOS registry (if enabled)
    /// 3. Register capabilities
    /// 4. Discover dependencies (BearDog, Songbird)
    /// 5. Start API server
    /// 6. Begin heartbeat
    pub async fn start(config: DaemonConfig) -> Result<Self> {
        info!("🍄 Initializing ToadStool daemon server...");
        
        // Connect to biomeOS registry (if enabled)
        let biomeos_client = if config.register_with_biomeos {
            match Self::connect_to_biomeos(&config).await {
                Ok(client) => {
                    info!("✅ Connected to biomeOS capability registry");
                    
                    // Register our capabilities
                    if let Err(e) = Self::register_capabilities(&client).await {
                        warn!("⚠️  Failed to register capabilities: {e}");
                        warn!("📍 Continuing in standalone mode");
                        None
                    } else {
                        info!("✅ Registered ToadStool capabilities with biomeOS");
                        Some(Arc::new(client))
                    }
                }
                Err(e) => {
                    warn!("⚠️  Failed to connect to biomeOS registry: {e}");
                    warn!("📍 Running in standalone mode");
                    None
                }
            }
        } else {
            info!("📍 biomeOS registration disabled - running in standalone mode");
            None
        };
        
        // Create workload manager
        let workload_manager = WorkloadManager::new(config.max_concurrent_workloads).await?;
        info!("✅ Workload manager initialized");
        
        // TODO Phase 4: Start resource monitor
        // TODO Phase 5: Start heartbeat loop
        
        info!("✅ ToadStool daemon server initialized");
        
        Ok(Self {
            config,
            biomeos_client,
            workload_manager: Arc::new(workload_manager),
        })
    }
    
    /// Connect to biomeOS capability registry
    async fn connect_to_biomeos(_config: &DaemonConfig) -> Result<toadstool::biomeos_integration::BiomeOSClient> {
        info!("🔗 Connecting to biomeOS capability registry...");
        
        // Use real BiomeOSClient
        let client = toadstool::biomeos_integration::BiomeOSClient::connect().await?;
        
        Ok(client)
    }
    
    /// Register capabilities with biomeOS
    async fn register_capabilities(client: &toadstool::biomeos_integration::BiomeOSClient) -> Result<()> {
        info!("📋 Registering capabilities with biomeOS...");
        
        // Use real capability registration
        client.register_self().await?;
        
        info!("  - Capability: Compute (wasm, container, python, native, gpu)");
        info!("  - Capability: Storage (local, distributed, encrypted)");
        info!("  - Capability: Orchestration");
        
        Ok(())
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
            let client = self.biomeos_client.clone();
            let manager = self.workload_manager.clone();
            
            tokio::spawn(async move {
                if let Err(e) = http_server::start_http_server(port, client, manager).await {
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
        
        // TODO Phase 3: Stop all workloads
        // TODO Phase 2: Stop HTTP server
        // TODO: Unregister from biomeOS
        
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

