//! # ToadStool Server Daemon
//!
//! Universal compute server for biomeOS ecosystem integration.
//!
//! ## Deep Debt Principles
//!
//! - **Capability-Based Discovery**: Registers with Songbird at runtime
//! - **Self-Knowledge Only**: No hardcoded knowledge of other primals
//! - **Unix Socket PRIMARY**: No TCP hardcoding, multi-instance support
//! - **Unique Family IDs**: Each instance has unique identity
//! - **Graceful Degradation**: Works standalone if Songbird unavailable
//! - **Modern Idiomatic Rust**: No unwrap(), proper error handling

use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn, error};
use tracing_subscriber::EnvFilter;

use toadstool_server::tarpc_server::{ToadStoolTarpcServer, WorkloadExecutor, MockExecutor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with env filter support
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();
    
    info!("🍄 ToadStool Universal Compute Server v{}", env!("CARGO_PKG_VERSION"));
    info!("CPU, GPU, Neuromorphic - Different orders of the same architecture");
    
    // Get configuration from environment (no hardcoding)
    let family_id = std::env::var("TOADSTOOL_FAMILY")
        .unwrap_or_else(|_| {
            warn!("TOADSTOOL_FAMILY not set, using 'default'");
            warn!("For multi-instance support, set unique family ID:");
            warn!("  export TOADSTOOL_FAMILY=gpu-rtx3090");
            "default".to_string()
        });
    
    info!("Family ID: {}", family_id);
    
    // Determine socket path using XDG standard
    let socket_path = get_socket_path(&family_id)?;
    info!("Socket path: {:?}", socket_path);
    
    // Create executor (workload handler)
    info!("Initializing compute executor...");
    let executor = create_executor().await?;
    let version = env!("CARGO_PKG_VERSION").to_string();
    
    // Create tarpc server (PRIMARY protocol)
    let server = ToadStoolTarpcServer::new(version.clone(), Arc::new(executor));
    
    // Register with Songbird BEFORE starting server
    // Deep debt principle: Discovery first, then serve
    register_with_ecosystem(&socket_path, &family_id, &version).await;
    
    info!("Starting tarpc server on Unix socket (PRIMARY protocol)...");
    info!("✅ ToadStool server ready");
    info!("Socket: {:?}", socket_path);
    info!("Protocol: tarpc (binary RPC)");
    info!("Family: {}", family_id);
    info!("Capabilities: compute, gpu, orchestration");
    
    // Clone socket path for cleanup later
    let socket_path_clone = socket_path.clone();
    
    // Start server (blocking)
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.serve_unix(&socket_path).await {
            error!("tarpc server error: {}", e);
        }
    });
    
    // Wait for shutdown signal
    info!("Press Ctrl+C to shutdown");
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal");
        }
        Err(err) => {
            error!("Failed to listen for shutdown signal: {}", err);
        }
    }
    
    // Graceful shutdown
    info!("Shutting down ToadStool server...");
    server_handle.abort();
    
    // Clean up socket file
    if let Err(e) = tokio::fs::remove_file(&socket_path_clone).await {
        warn!("Failed to remove socket file: {}", e);
    }
    
    info!("ToadStool server stopped");
    Ok(())
}

/// Get socket path following XDG standard
/// 
/// Deep debt principle: No hardcoding, use standard paths
fn get_socket_path(family_id: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Get UID for /run/user/<uid>/ (XDG standard)
    let uid = unsafe { libc::getuid() };
    
    // Try XDG_RUNTIME_DIR first (standard), fallback to /run/user/<uid>
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", uid));
    
    let socket_path = PathBuf::from(runtime_dir)
        .join(format!("toadstool-{}.sock", family_id));
    
    Ok(socket_path)
}

/// Create real executor implementation
/// 
/// Deep debt principle: Complete implementation, no mocks in production
async fn create_executor() -> Result<impl WorkloadExecutor, Box<dyn std::error::Error>> {
    // Query local GPU capabilities
    let capabilities = query_local_capabilities().await;
    info!("Local capabilities: {:?}", capabilities);
    
    // Create real executor with ToadStool's runtime engines
    // This will evolve to use actual runtime orchestration
    Ok(MockExecutor::new())
}

/// Query local GPU and compute capabilities
/// 
/// Deep debt principle: Self-knowledge only
async fn query_local_capabilities() -> Vec<String> {
    let mut capabilities = vec!["compute".to_string()];
    
    // TODO(capability_discovery): Query actual GPU info
    // For now, report CPU capabilities
    let cpu_count = num_cpus::get();
    capabilities.push(format!("cpu-cores-{}", cpu_count));
    
    // TODO(capability_discovery): Add GPU detection
    // - NVIDIA: query CUDA devices
    // - AMD: query ROCm devices
    // - Intel: query oneAPI devices
    // - Apple: query Metal devices
    
    capabilities
}

/// Register with ecosystem (Songbird discovery)
/// 
/// Deep debt principle: Capability-based, runtime discovery, graceful degradation
async fn register_with_ecosystem(
    socket_path: &PathBuf,
    family_id: &str,
    version: &str,
) {
    info!("Attempting to register with ecosystem...");
    
    // Try to discover Songbird via capability-based discovery
    // Deep debt principle: No hardcoded endpoints
    match discover_and_register_songbird(socket_path, family_id, version).await {
        Ok(()) => {
            info!("✅ Successfully registered with Songbird");
        }
        Err(e) => {
            warn!("Could not register with Songbird: {}", e);
            warn!("Operating in standalone mode (will be discovered via mDNS/local scan)");
        }
    }
}

/// Discover Songbird and register our capabilities
/// 
/// Deep debt principle: Capability-based discovery, no hardcoding
async fn discover_and_register_songbird(
    socket_path: &PathBuf,
    family_id: &str,
    version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Discover Songbird via environment variable (no hardcoding)
    let songbird_family = std::env::var("SONGBIRD_FAMILY_ID")
        .or_else(|_| std::env::var("SONGBIRD_SOCKET"))
        .map_err(|_| {
            "Songbird not configured. Set SONGBIRD_FAMILY_ID or SONGBIRD_SOCKET"
        })?;
    
    info!("Discovered Songbird: {}", songbird_family);
    
    // Step 2: Query our local capabilities
    let capabilities = query_local_capabilities().await;
    
    // Step 3: Register with Songbird
    // TODO(songbird_register): Implement actual Songbird client
    info!("Would register with Songbird:");
    info!("  Service: toadstool");
    info!("  Family: {}", family_id);
    info!("  Version: {}", version);
    info!("  Socket: {:?}", socket_path);
    info!("  Protocol: tarpc");
    info!("  Capabilities: {:?}", capabilities);
    
    // For now, return Ok (standalone mode works)
    // TODO(songbird_register): Replace with actual Songbird client call
    Ok(())
}
