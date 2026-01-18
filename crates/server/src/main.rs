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
//!
//! ## UniBin Architecture
//!
//! This standalone binary shares logic with the `toadstool server` UniBin command.
//! Both call the same `run_server_main()` function from the toadstool-server library.

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with env filter support
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Call shared UniBin server implementation
    toadstool_server::run_server_main().await
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
    let resources = query_system_resources();
    build_capabilities(&resources)
}

/// Register with ecosystem (Songbird discovery)
///
/// Deep debt principle: Self-knowledge + Announcement (not registration!)
async fn announce_capabilities_to_ecosystem(_socket_path: &PathBuf, _family_id: &str, _version: &str) {
    use toadstool_server::capabilities::PrimalCapabilities;
    
    info!("🔍 Discovering self capabilities (self-knowledge!)");

    // EVOLVED: Self-knowledge + announcement (not external registration!)
    let capabilities = PrimalCapabilities::discover_self("toadstool").await;

    info!("✅ Self-knowledge acquired:");
    info!("   - Primal ID: {}", capabilities.primal_id);
    info!("   - Type: {}", capabilities.primal_type);
    info!("   - CPU Cores: {}", capabilities.resources.cpu_cores);
    info!("   - Memory: {} GB", capabilities.resources.total_memory_bytes / (1024 * 1024 * 1024));
    info!("   - Architecture: {}", capabilities.resources.architecture);
    info!("   - OS: {}", capabilities.resources.os);
    info!("   - Capabilities: {}", capabilities.capabilities.len());

    // Announce capabilities (optional, for peer discovery)
    match capabilities.announce().await {
        Ok(()) => {
            info!("📢 Successfully announced capabilities for peer discovery");
            info!("   Deep debt principle: Announcement, not registration!");
            info!("   Peers can now discover us via capability files");
        }
        Err(e) => {
            warn!("Could not announce capabilities: {}", e);
            warn!("Operating without peer discovery (standalone mode)");
        }
    }
}

// REMOVED: discover_and_register_songbird() - EVOLVED to announcement-based!
// Deep debt evolution: External registration → Self-knowledge + Peer discovery
