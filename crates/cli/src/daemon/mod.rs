//! ToadStool Daemon Mode
//!
//! 🍄 **Like the fungus: Same organism, different forms**
//!
//! - **CLI Mode** (fruiting body): Specialized, project-specific execution
//! - **Daemon Mode** (mycelium): Network-wide, resource-sharing compute service
//!
//! ## Architecture
//!
//! The daemon mode transforms ToadStool from a CLI tool into an ecosystem workload execution service:
//!
//! - **HTTP API Server**: Accept workload requests from other primals or remote nodes
//! - **Capability Registry**: Auto-register capabilities, report resources, heartbeat
//! - **Workload Manager**: Queue, execute, and monitor workloads
//! - **Resource Monitor**: Track CPU, memory, GPU, storage and report to registry
//! - **Infant Discovery**: Discover security and coordination providers at runtime by capability
//!
//! ## Usage
//!
//! ```bash
//! # Start daemon with biomeOS registration
//! toadstool daemon --register
//!
//! # Start daemon on custom port
//! toadstool daemon --port 8085
//!
//! # Submit workload via API
//! curl -X POST http://localhost:8084/api/v1/workload/submit \
//!   -H "Content-Type: application/json" \
//!   -d '{"biome_yaml": "...", "requester": "beardog"}'
//! ```
//!
//! ## Infant Discovery
//!
//! The daemon starts with ZERO knowledge and discovers everything at runtime:
//!
//! 1. Load self-knowledge (ports, resources)
//! 2. Connect to capability registry (if --register)
//! 3. Register capabilities (Compute, Storage, Orchestration)
//! 4. Discover security provider by capability
//! 5. Discover coordination provider by capability
//! 6. Start API server
//! 7. Begin heartbeat reporting

use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

mod api_types;
mod config;
mod http_server;
mod server;
mod workload_manager;

pub use api_types::*;
pub use config::DaemonConfig;
pub use server::DaemonServer;
pub use workload_manager::WorkloadManager;

/// Start ToadStool in daemon mode
///
/// ## Infant Discovery Flow
///
/// 1. **Self-Knowledge**: Load own ports and resource info
/// 2. **Registry Discovery**: Connect to capability registry (optional)
/// 3. **Capability Registration**: Report what we provide (Compute, Storage, Orchestration)
/// 4. **Dependency Discovery**: Find security and coordination providers by capability
/// 5. **API Server**: Start HTTP server for workload submission
/// 6. **Heartbeat**: Report resources and health to registry
///
/// ## Philosophy
///
/// Zero hardcoded knowledge. Everything discovered at runtime via infant discovery.
pub async fn start_daemon(
    port: u16,
    register_with_biomeos: bool,
    socket_path: Option<PathBuf>,
    config_path: Option<PathBuf>,
    max_workloads: usize,
    biomeos_socket: Option<PathBuf>,
) -> Result<()> {
    info!("🍄 Starting ToadStool daemon mode...");
    info!("📍 Port: {}", port);
    info!("🔗 Capability registry: {}", if register_with_biomeos { "enabled" } else { "disabled" });

    // Load configuration
    let config = DaemonConfig::load(
        port,
        register_with_biomeos,
        socket_path,
        config_path,
        max_workloads,
        biomeos_socket,
    ).await?;

    // Start daemon server
    let daemon = DaemonServer::start(config).await?;

    info!("✅ ToadStool daemon started successfully");
    info!("🌐 API: http://localhost:{}/api/v1", port);
    info!("📊 Health: http://localhost:{}/health", port);
    info!("📈 Metrics: http://localhost:{}/metrics", port);
    
    if register_with_biomeos {
        info!("🔗 Registered with capability registry");
    } else {
        info!("📍 Running in standalone mode (no registry)");
    }

    // Run daemon until shutdown signal
    daemon.run().await?;

    Ok(())
}

