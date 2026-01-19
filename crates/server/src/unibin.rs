//! UniBin server entry point
//!
//! Shared server main logic for both toadstool and toadstool-server binaries

use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};

use toadstool_distributed::{DistributedConfig, StandaloneConfig};
// DISABLED: HTTP-based Songbird registration (legacy)
// use crate::songbird_client::{
//     build_capabilities, query_system_resources, ServiceLocation, SongbirdClient,
//     SongbirdRegistration,
// };
use crate::tarpc_server::{StandaloneExecutor, ToadStoolTarpcServer, WorkloadExecutor};
use crate::{CoordinatorExecutor, ManualJsonRpcServer};

/// Run ToadStool in server/daemon mode
///
/// This is the main entry point for both `toadstool server` (UniBin)
/// and `toadstool-server` (standalone binary).
///
/// ## UniBin Architecture
///
/// This function enables the UniBin pattern by providing a shared
/// implementation that can be called from multiple binary entry points.
pub async fn run_server_main() -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "🍄 ToadStool Universal Compute Server v{}",
        env!("CARGO_PKG_VERSION")
    );
    info!("CPU, GPU, Neuromorphic - Different orders of the same architecture");

    // Get configuration from environment (TRUE PRIMAL standard)
    // Priority: TOADSTOOL_FAMILY_ID > TOADSTOOL_FAMILY > BIOMEOS_FAMILY_ID > default
    let family_id = std::env::var("TOADSTOOL_FAMILY_ID")
        .or_else(|_| std::env::var("TOADSTOOL_FAMILY"))
        .or_else(|_| std::env::var("BIOMEOS_FAMILY_ID"))
        .unwrap_or_else(|_| {
            warn!("No family ID environment variables set, using 'default'");
            warn!("For multi-instance support, set one of:");
            warn!("  export TOADSTOOL_FAMILY_ID=nat0 (primal-specific)");
            warn!("  export BIOMEOS_FAMILY_ID=nat0 (orchestrator-provided)");
            "default".to_string()
        });

    let node_id = std::env::var("TOADSTOOL_NODE_ID").unwrap_or_else(|_| {
        info!("TOADSTOOL_NODE_ID not set, using 'default'");
        "default".to_string()
    });

    info!("Family ID: {}", family_id);
    info!("Node ID: {}", node_id);

    // Determine socket path using biomeOS-standardized 3-tier fallback
    info!("🔍 Socket Path Discovery:");
    info!(
        "  Checking TOADSTOOL_SOCKET: {:?}",
        std::env::var("TOADSTOOL_SOCKET").ok()
    );
    info!(
        "  Checking BIOMEOS_SOCKET_PATH: {:?}",
        std::env::var("BIOMEOS_SOCKET_PATH").ok()
    );
    info!(
        "  Checking XDG_RUNTIME_DIR: {:?}",
        std::env::var("XDG_RUNTIME_DIR").ok()
    );

    let socket_path = get_socket_path(&family_id, &node_id)?;
    info!("✅ Final socket path: {:?}", socket_path);

    // Create executor (workload handler) - now with distributed coordinator
    info!("Initializing compute executor...");
    let executor = create_executor(&family_id).await?;
    let version = env!("CARGO_PKG_VERSION").to_string();

    // Create tarpc server (PRIMARY protocol)
    let server = ToadStoolTarpcServer::new(version.clone(), Arc::clone(&executor));

    // EVOLUTION NOTE: Songbird registration now happens via capability-based discovery
    // Songbird discovers ToadStool by checking known socket paths (environment/XDG)
    // No HTTP client needed! Registration happens when Songbird finds our Unix socket.
    // See: ARCHITECTURAL_DEBT_SONGBIRD_HTTP_JAN_16_2026.md for evolution plan
    //
    // OLD (HTTP-based - removed for TRUE UniBin):
    // register_with_ecosystem(&socket_path, &family_id, &version).await;
    info!("🔍 Capability-based discovery: Songbird will find us via Unix socket");
    info!("   Socket advertised via environment/XDG runtime directory");

    // Start manual JSON-RPC server on Unix socket (for universal compatibility)
    info!("Starting manual JSON-RPC 2.0 server on Unix socket (UNIVERSAL)...");
    let jsonrpc_socket = socket_path.with_extension("jsonrpc.sock");
    let jsonrpc_server = ManualJsonRpcServer::new(Arc::clone(&executor), version.clone());
    let jsonrpc_socket_clone = jsonrpc_socket.clone();
    tokio::spawn(async move {
        if let Err(e) = jsonrpc_server.serve(jsonrpc_socket_clone).await {
            error!("JSON-RPC server error: {}", e);
        }
    });

    info!("Starting tarpc server on Unix socket (PRIMARY protocol)...");
    info!("✅ ToadStool server ready");
    info!("Socket (tarpc): {:?}", socket_path);
    info!("Socket (JSON-RPC): {:?}", jsonrpc_socket);
    info!("Protocol: tarpc (binary RPC, PRIMARY)");
    info!("Protocol: JSON-RPC 2.0 (universal, FALLBACK)");
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

    // Clean up socket files
    if let Err(e) = tokio::fs::remove_file(&socket_path_clone).await {
        warn!("Failed to remove tarpc socket: {}", e);
    }
    let jsonrpc_socket = socket_path_clone.with_extension("jsonrpc.sock");
    if let Err(e) = tokio::fs::remove_file(&jsonrpc_socket).await {
        warn!("Failed to remove JSON-RPC socket: {}", e);
    }

    info!("ToadStool server stopped");
    Ok(())
}

/// Get socket path following biomeOS-standardized fallback
///
/// Deep debt principle: Agnostic, capability-based, runtime discovery
///
/// Priority order (TRUE PRIMAL standard):
/// 1. TOADSTOOL_SOCKET env var (primal-specific absolute path) - highest priority
/// 2. BIOMEOS_SOCKET_PATH env var (orchestrator-provided generic path)
/// 3. XDG runtime directory (/run/user/<uid>/toadstool-<family>.sock) - user mode
/// 4. /tmp fallback (/tmp/toadstool-<family>.sock) - system mode
fn get_socket_path(family_id: &str, _node_id: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // 1. HIGHEST PRIORITY: Check TOADSTOOL_SOCKET env var (primal-specific)
    if let Ok(socket) = std::env::var("TOADSTOOL_SOCKET") {
        info!("✅ Using socket path from TOADSTOOL_SOCKET: {}", socket);
        info!("   (Orchestrator-provided explicit path - highest priority)");
        return Ok(PathBuf::from(socket));
    }

    // 2. BIOMEOS_SOCKET_PATH: Generic orchestrator-provided path
    if let Ok(socket) = std::env::var("BIOMEOS_SOCKET_PATH") {
        info!("✅ Using socket path from BIOMEOS_SOCKET_PATH: {}", socket);
        info!("   (Generic biomeOS path - second priority)");
        return Ok(PathBuf::from(socket));
    }

    // 3. XDG runtime directory (standard for user-mode deployments)
    // EVOLVED: Pure Rust - no unsafe! Use environment or fallback to /tmp
    // Primal principle: Prefer environment-based discovery over system calls
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        // Fallback: Use /tmp with username for multi-user systems
        // This is safer and works in all environments (containers, etc.)
        let username = std::env::var("USER").unwrap_or_else(|_| "default".to_string());
        format!("/tmp/toadstool-runtime-{}", username)
    });

    let xdg_path = PathBuf::from(&runtime_dir).join(format!("toadstool-{}.sock", family_id));

    if PathBuf::from(&runtime_dir).exists() {
        info!("⚠️  Using XDG runtime directory fallback: {}", runtime_dir);
        info!("   (User-mode deployment - third priority)");
        info!("   NOTE: For orchestrator deployments, set TOADSTOOL_SOCKET env var!");
        return Ok(xdg_path);
    }

    // 4. /tmp fallback (system-wide deployments, containers, minimal systems)
    let tmp_path = PathBuf::from("/tmp").join(format!("toadstool-{}.sock", family_id));

    info!("⚠️  Using /tmp fallback for system-wide deployment");
    info!("   (System-mode deployment - lowest priority)");
    info!("   NOTE: For orchestrator deployments, set TOADSTOOL_SOCKET env var!");
    Ok(tmp_path)
}

/// Create real executor implementation
///
/// Deep debt principle: Complete implementation, no mocks in production
/// Now uses DistributedCoordinator for isomorphic/fractal coordination
async fn create_executor(
    family_id: &str,
) -> Result<Arc<dyn WorkloadExecutor + Send + Sync>, Box<dyn std::error::Error>> {
    info!("Creating executor with distributed coordinator (isomorphic/fractal)");

    // Check if we should use distributed mode (default) or standalone fallback
    let use_distributed = std::env::var("TOADSTOOL_STANDALONE")
        .map(|v| v != "1" && v.to_lowercase() != "true")
        .unwrap_or(true); // Default to distributed

    if use_distributed {
        info!("Initializing distributed coordinator mode");

        // Query local capabilities first
        let capabilities = query_local_capabilities().await;
        info!("Local capabilities: {:?}", capabilities);

        // Create distributed config
        let config = DistributedConfig {
            instance_id: format!("toadstool-{}", family_id),
            standalone: StandaloneConfig {
                max_concurrent_executions: 10,
                default_timeout_secs: 300,
                enable_job_queue: true,
                max_queue_size: 100,
            },
            songbird_integration: Some(toadstool_distributed::SongbirdConfig {
                endpoint: std::env::var("SONGBIRD_ENDPOINT")
                    .or_else(|_| std::env::var("TOADSTOOL_COORDINATION_ENDPOINT"))
                    .unwrap_or_else(|_| {
                        // Runtime discovery fallback - will attempt mDNS/DNS-SD discovery
                        // No hardcoded localhost - respects deployment environment
                        tracing::info!(
                            "No SONGBIRD_ENDPOINT configured, will use runtime discovery"
                        );
                        String::new() // Empty = trigger discovery
                    }),
                auth_token: std::env::var("SONGBIRD_AUTH_TOKEN").ok(),
                health_reporting_interval_secs: 60,
            }),
        };

        // Create coordinator executor
        let service_id = format!("toadstool-{}", family_id);
        let executor = CoordinatorExecutor::new(config, service_id)
            .await
            .map_err(|e| format!("Failed to create coordinator executor: {}", e))?;

        info!("✅ Distributed coordinator executor ready");
        Ok(Arc::new(executor))
    } else {
        info!("Using standalone executor (TOADSTOOL_STANDALONE=1)");
        let capabilities = query_local_capabilities().await;
        info!("Local capabilities: {:?}", capabilities);
        Ok(Arc::new(StandaloneExecutor::new()))
    }
}

/// Query local GPU and compute capabilities
///
/// Deep debt principle: Self-knowledge only
async fn query_local_capabilities() -> Vec<String> {
    // TEMPORARY: Return basic capabilities until we implement pure Rust capability query
    // TODO: Implement capability discovery without HTTP dependencies
    vec![
        "compute".to_string(),
        "cpu".to_string(),
        "orchestration".to_string(),
    ]
}

// DISABLED: HTTP-based ecosystem registration (legacy)
// Evolution: Songbird discovers ToadStool via Unix socket paths
//
// /// Register with ecosystem (Songbird discovery)
// ///
// /// Deep debt principle: Capability-based, runtime discovery, graceful degradation
// async fn register_with_ecosystem(socket_path: &PathBuf, family_id: &str, version: &str) {
//     info!("Attempting to register with ecosystem...");
//
//     // Try to discover Songbird via capability-based discovery
//     // Deep debt principle: No hardcoded endpoints
//     match discover_and_register_songbird(socket_path, family_id, version).await {
//         Ok(()) => {
//             info!("✅ Successfully registered with Songbird");
//         }
//         Err(e) => {
//             warn!("Could not register with Songbird: {}", e);
//             warn!("Operating in standalone mode (will be discovered via mDNS/local scan)");
//         }
//     }
// }
//
// /// Discover Songbird and register our capabilities
// ///
// /// Deep debt principle: Capability-based discovery, no hardcoding
// async fn discover_and_register_songbird(
//     socket_path: &PathBuf,
//     family_id: &str,
//     version: &str,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     // Step 1: Discover Songbird (no hardcoding)
//     let songbird = SongbirdClient::discover()
//         .await
//         .map_err(|e| format!("Failed to discover Songbird: {}", e))?;
//
//     info!("Discovered Songbird successfully");
//
//     // Step 2: Query our local capabilities (self-knowledge)
//     let resources = query_system_resources();
//     let capabilities = build_capabilities(&resources);
//
//     info!("Local capabilities: {:?}", capabilities);
//     info!(
//         "System resources: {} CPU cores, {} MB memory",
//         resources.cpu_cores,
//         resources.total_memory_bytes / 1024 / 1024
//     );
//
//     // Step 3: Build registration
//     let registration = SongbirdRegistration {
//         service_id: format!("toadstool-{}", family_id),
//         service_name: "toadstool".to_string(),
//         family_id: family_id.to_string(),
//         version: version.to_string(),
//         capabilities,
//         location: ServiceLocation {
//             location_type: "unix-socket".to_string(),
//             path: socket_path.to_string_lossy().to_string(),
//             protocol: "tarpc".to_string(),
//         },
//         resources,
//         metadata: std::collections::HashMap::from([
//             ("platform".to_string(), std::env::consts::OS.to_string()),
//             ("arch".to_string(), std::env::consts::ARCH.to_string()),
//         ]),
//         ttl_seconds: 300, // 5 minutes TTL
//     };
//
//     // Step 4: Register with Songbird
//     songbird
//         .register_service(registration)
//         .await
//         .map_err(|e| format!("Failed to register: {}", e))?;
//
//     info!("✅ Registered with Songbird");
//
//     // Step 5: Start heartbeat task
//     let service_id = format!("toadstool-{}", family_id);
//     tokio::spawn(async move {
//         let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
//         loop {
//             interval.tick().await;
//             if let Err(e) = songbird.heartbeat(&service_id).await {
//                 warn!("Heartbeat failed: {}", e);
//             }
//         }
//     });
//
//     Ok(())
// }
