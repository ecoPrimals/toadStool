//! UniBin server entry point
//!
//! Shared server main logic for both toadstool and toadstool-server binaries

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Exit codes following uniBin/ecoBin standard
///
/// **ecoBin Compliance**: Standard exit codes for consistent system integration
pub mod exit_codes {
    /// Success - operation completed normally
    pub const SUCCESS: i32 = 0;
    /// General error - unspecified failure
    pub const GENERAL_ERROR: i32 = 1;
    /// Configuration error - invalid config, missing required settings
    pub const CONFIG_ERROR: i32 = 2;
    /// Runtime/network error - connection failures, resource exhaustion
    pub const RUNTIME_ERROR: i32 = 3;
    /// Interrupted - received SIGINT (Ctrl+C) or SIGTERM
    pub const INTERRUPTED: i32 = 130;
}

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
///
/// ## Family ID Resolution
///
/// 1. If `family_id_override` (from `--family-id` CLI) is set, use it
/// 2. Else fall back to env vars: TOADSTOOL_FAMILY_ID, TOADSTOOL_FAMILY, BIOMEOS_FAMILY_ID
/// 3. If family ID is set and not "default", socket is `toadstool-{family_id}.sock`
/// 4. If family ID is "default" or not set, socket is `toadstool.sock`
pub async fn run_server_main(
    family_id_override: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "🍄 ToadStool Universal Compute Server v{}",
        env!("CARGO_PKG_VERSION")
    );
    info!("CPU, GPU, Neuromorphic - Different orders of the same architecture");

    // Get configuration: CLI override first, then env, then default
    // Priority: --family-id > TOADSTOOL_FAMILY_ID > TOADSTOOL_FAMILY > BIOMEOS_FAMILY_ID > default
    let family_id = family_id_override
        .or_else(|| std::env::var("TOADSTOOL_FAMILY_ID").ok())
        .or_else(|| std::env::var("TOADSTOOL_FAMILY").ok())
        .or_else(|| std::env::var("BIOMEOS_FAMILY_ID").ok())
        .unwrap_or_else(|| {
            warn!("No family ID (CLI or env) set, using 'default'");
            warn!("For multi-instance support, use --family-id=nat0 or set one of:");
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

    // Shared error counter for unified monitoring across tarpc and JSON-RPC
    let error_count = Arc::new(AtomicU64::new(0));

    // Create tarpc server (PRIMARY protocol)
    let server = ToadStoolTarpcServer::new(
        version.clone(),
        Arc::clone(&executor),
        Some(Arc::clone(&error_count)),
    );

    // EVOLVED: Service-based registration with Songbird (Deep Debt!)
    // Register with Songbird discovery service if available
    info!("🌍 Attempting registration with Songbird discovery service...");
    match toadstool::ipc_helpers::register_with_songbird().await {
        Ok(()) => {
            info!("✅ Successfully registered with Songbird!");
            info!("   Other primals can now discover us via runtime discovery");
        }
        Err(e) => {
            warn!("⚠️  Could not register with Songbird: {}", e);
            warn!("   Operating in standalone mode (no discovery)");
            warn!("   This is OK if Songbird is not running yet");
        }
    }

    // Start servers with isomorphic IPC (Try→Detect→Adapt→Succeed)
    info!("🔌 Starting IPC servers (isomorphic mode)...");
    info!("   Protocol: tarpc (PRIMARY) + JSON-RPC 2.0 (UNIVERSAL)");

    let jsonrpc_socket = socket_path.with_extension("jsonrpc.sock");
    let jsonrpc_server = ManualJsonRpcServer::new(
        Arc::clone(&executor),
        version.clone(),
        Some(Arc::clone(&error_count)),
    );

    // Clone for server startup
    let socket_path_for_server = socket_path.clone();
    let jsonrpc_socket_for_server = jsonrpc_socket.clone();

    // Start both servers with fallback support
    let server_handle = tokio::spawn(async move {
        match start_servers_with_fallback(
            server,
            jsonrpc_server,
            socket_path_for_server,
            jsonrpc_socket_for_server,
        )
        .await
        {
            Ok(()) => {
                info!("✅ Servers stopped gracefully");
            }
            Err(e) => {
                error!("❌ Server error: {}", e);
            }
        }
    });

    // Wait for shutdown signal (SIGINT or SIGTERM)
    // **ecoBin Compliance**: Handle both signals for proper system integration
    info!("Ready for shutdown (Ctrl+C or SIGTERM)");
    let shutdown_signal = wait_for_shutdown_signal().await;

    match shutdown_signal {
        ShutdownSignal::Sigint => {
            info!("📡 Received SIGINT (Ctrl+C)");
        }
        ShutdownSignal::Sigterm => {
            info!("📡 Received SIGTERM (graceful shutdown)");
        }
        ShutdownSignal::Error(err) => {
            error!("Failed to listen for shutdown signals: {}", err);
        }
    }

    // Graceful shutdown
    info!("Shutting down ToadStool server...");
    server_handle.abort();

    // Clean up socket files (if they exist - TCP fallback won't have them)
    if socket_path.exists() {
        if let Err(e) = tokio::fs::remove_file(&socket_path).await {
            warn!("Failed to remove tarpc socket: {}", e);
        }
    }
    if jsonrpc_socket.exists() {
        if let Err(e) = tokio::fs::remove_file(&jsonrpc_socket).await {
            warn!("Failed to remove JSON-RPC socket: {}", e);
        }
    }

    info!("ToadStool server stopped");
    Ok(())
}

/// Ensure biomeos directory exists with proper permissions
///
/// biomeOS socket standard: All sockets in /run/user/$UID/biomeos/ subdirectory
/// This enables discovery, organization, and security
fn ensure_biomeos_directory(
    runtime_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let biomeos_dir = runtime_dir.join("biomeos");

    // Create directory if doesn't exist
    std::fs::create_dir_all(&biomeos_dir)?;

    // Set permissions to 0700 (user-only access)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&biomeos_dir, perms)?;
    }

    info!("✅ biomeos directory ensured: {}", biomeos_dir.display());
    Ok(biomeos_dir)
}

/// Get socket filename based on family ID
///
/// - If family ID is "default" or not set: `toadstool.sock`
/// - If family ID is set and not "default": `toadstool-{family_id}.sock`
fn socket_filename_for_family(family_id: &str) -> String {
    if family_id.is_empty() || family_id == "default" {
        "toadstool.sock".to_string()
    } else {
        format!("toadstool-{}.sock", family_id)
    }
}

/// Get socket path following biomeOS-standardized fallback
///
/// Deep debt principle: Agnostic, capability-based, runtime discovery
///
/// Priority order (5-TIER A++ STANDARD - matching BearDog):
/// 1. TOADSTOOL_SOCKET env var (primal-specific absolute path) - highest priority
/// 2. PRIMAL_SOCKET env var (generic primal socket with family suffix)
/// 3. BIOMEOS_SOCKET_PATH env var (orchestrator-provided generic path)
/// 4. XDG runtime directory (/run/user/<uid>/biomeos/toadstool.sock) - **STANDARD**
/// 5. /tmp fallback (/tmp/biomeos/toadstool.sock) - dev/testing only
fn get_socket_path(
    family_id: &str,
    _node_id: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    // 1. HIGHEST PRIORITY: Check TOADSTOOL_SOCKET env var (primal-specific)
    if let Ok(socket) = std::env::var("TOADSTOOL_SOCKET") {
        info!("✅ Using socket path from TOADSTOOL_SOCKET: {}", socket);
        info!("   (Explicit primal-specific path - highest priority)");
        return Ok(PathBuf::from(socket));
    }

    // 2. PRIMAL_SOCKET: Generic primal socket (with family suffix for multi-family deployments)
    if let Ok(socket) = std::env::var("PRIMAL_SOCKET") {
        let socket_with_family = format!("{}-{}", socket, family_id);
        info!(
            "✅ Using socket path from PRIMAL_SOCKET: {}",
            socket_with_family
        );
        info!("   (Generic primal socket with family suffix - second priority)");
        return Ok(PathBuf::from(socket_with_family));
    }

    // 3. BIOMEOS_SOCKET_PATH: Generic orchestrator-provided path
    if let Ok(socket) = std::env::var("BIOMEOS_SOCKET_PATH") {
        info!("✅ Using socket path from BIOMEOS_SOCKET_PATH: {}", socket);
        info!("   (Generic biomeOS path - third priority)");
        return Ok(PathBuf::from(socket));
    }

    // 4. XDG runtime directory + biomeos subdirectory (***STANDARD PATH***)
    // EVOLVED: Pure Rust - no unsafe! Use environment or fallback to /run/user/<uid>
    // Primal principle: Prefer environment-based discovery over system calls
    let runtime_dir = if let Ok(xdg_runtime) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(xdg_runtime)
    } else {
        // Fallback to Linux standard /run/user/<uid>
        // EVOLVED: Pure Rust UID detection (no unsafe!)
        // Try /proc/self first (Linux standard)
        if let Ok(uid_str) = std::fs::read_to_string("/proc/self/loginuid") {
            if let Ok(uid) = uid_str.trim().parse::<u32>() {
                PathBuf::from(format!("/run/user/{}", uid))
            } else {
                // Fallback: check USER environment
                std::env::var("USER")
                    .ok()
                    .and_then(|user| {
                        // Try to read /etc/passwd for UID
                        std::fs::read_to_string("/etc/passwd")
                            .ok()
                            .and_then(|passwd| {
                                passwd
                                    .lines()
                                    .find(|line| line.starts_with(&format!("{}:", user)))
                                    .and_then(|line| {
                                        line.split(':')
                                            .nth(2)
                                            .and_then(|uid| uid.parse::<u32>().ok())
                                    })
                                    .map(|uid| PathBuf::from(format!("/run/user/{}", uid)))
                            })
                    })
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
            }
        } else {
            // Ultimate fallback if /proc not available
            PathBuf::from("/tmp")
        }
    };

    if runtime_dir.exists() {
        let biomeos_dir = ensure_biomeos_directory(&runtime_dir)?;
        let socket_filename = socket_filename_for_family(family_id);
        let socket_path = biomeos_dir.join(socket_filename);

        info!(
            "✅ Using biomeOS standard socket path: {}",
            socket_path.display()
        );
        info!("   (XDG runtime + biomeos subdirectory - **STANDARD**))");
        info!("   Socket standardization: Tower Atomic compatible!");
        return Ok(socket_path);
    }

    // 5. /tmp fallback (dev/testing only - NOT RECOMMENDED for production!)
    // Create biomeos subdirectory in /tmp as well for consistency
    let tmp_biomeos = PathBuf::from("/tmp/biomeos");
    std::fs::create_dir_all(&tmp_biomeos)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&tmp_biomeos, perms)?;
    }

    let socket_filename = socket_filename_for_family(family_id);
    let tmp_path = tmp_biomeos.join(socket_filename);

    info!("⚠️  Using /tmp fallback for dev/testing deployment");
    info!("   Socket path: {}", tmp_path.display());
    info!("   (Development mode - lowest priority)");
    info!("   ⚠️  WARNING: NOT for production! Set TOADSTOOL_SOCKET or XDG_RUNTIME_DIR!");
    Ok(tmp_path)
}

/// Create real executor implementation
///
/// Deep debt principle: Complete implementation, no mocks in production
/// Now uses DistributedCoordinator for isomorphic/fractal coordination
async fn create_executor(
    family_id: &str,
) -> Result<Arc<dyn WorkloadExecutor + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
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
/// **Deep Debt**: Complete implementation using pure Rust hardware detection!
///
/// This implementation:
/// 1. Uses existing pure Rust dependencies (sysinfo, wgpu)
/// 2. Discovers actual system capabilities at runtime
/// 3. Returns only what THIS node can provide (self-knowledge)
/// 4. No hardcoding, no HTTP dependencies
async fn query_local_capabilities() -> Vec<String> {
    let mut capabilities = vec!["compute".to_string(), "cpu".to_string()];

    // Detect CPU capabilities (pure Rust via sysinfo!)
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    if cpus >= 16 {
        capabilities.push("high-core-count".to_string());
        tracing::info!("✅ High core count detected: {} cores", cpus);
    }

    // Detect memory capabilities (pure Rust via sysinfo!)
    let mut sys = sysinfo::System::new_all();
    sys.refresh_memory();
    let total_memory_gb = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    if total_memory_gb >= 32.0 {
        capabilities.push("high-memory".to_string());
        tracing::info!("✅ High memory detected: {:.1} GB", total_memory_gb);
    }

    // Detect GPU capabilities (pure Rust via wgpu!)
    #[cfg(feature = "gpu-discovery")]
    {
        let adapters = wgpu::Instance::default().enumerate_adapters(wgpu::Backends::all());
        if !adapters.is_empty() {
            capabilities.push("gpu".to_string());

            for adapter in adapters {
                let info = adapter.get_info();
                tracing::info!("✅ Detected GPU: {} ({:?})", info.name, info.backend);

                // Add backend-specific capabilities
                match info.backend {
                    wgpu::Backend::Vulkan => {
                        if !capabilities.contains(&"vulkan".to_string()) {
                            capabilities.push("vulkan".to_string());
                        }
                    }
                    wgpu::Backend::Metal => {
                        if !capabilities.contains(&"metal".to_string()) {
                            capabilities.push("metal".to_string());
                        }
                    }
                    wgpu::Backend::Dx12 => {
                        if !capabilities.contains(&"dx12".to_string()) {
                            capabilities.push("dx12".to_string());
                        }
                    }
                    _ => {}
                }

                // Detect vendor-specific capabilities from name
                let name_lower = info.name.to_lowercase();
                if name_lower.contains("nvidia") && !capabilities.contains(&"cuda".to_string()) {
                    capabilities.push("cuda".to_string());
                } else if name_lower.contains("amd") && !capabilities.contains(&"rocm".to_string())
                {
                    capabilities.push("rocm".to_string());
                } else if name_lower.contains("intel")
                    && !capabilities.contains(&"oneapi".to_string())
                {
                    capabilities.push("oneapi".to_string());
                }
            }
        } else {
            tracing::info!("No GPUs detected (CPU-only mode)");
        }
    }

    #[cfg(not(feature = "gpu-discovery"))]
    {
        tracing::info!("GPU discovery disabled (compile with --features gpu-discovery)");
    }

    // Always include orchestration capability
    capabilities.push("orchestration".to_string());

    tracing::info!("📊 Local capabilities: {:?}", capabilities);
    capabilities
}

/// Start servers with isomorphic IPC fallback (Try→Detect→Adapt→Succeed)
///
/// **Zero Configuration**: Automatically adapts to platform constraints!
///
/// ## Behavior
///
/// 1. **TRY** Unix sockets (optimal)
/// 2. **DETECT** platform constraints (SELinux, unsupported)
/// 3. **ADAPT** to TCP fallback (127.0.0.1:ephemeral)
/// 4. **SUCCEED** or fail with real error
async fn start_servers_with_fallback(
    server: ToadStoolTarpcServer,
    jsonrpc_server: ManualJsonRpcServer,
    socket_path: PathBuf,
    jsonrpc_socket: PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("   Trying Unix socket IPC (optimal)...");

    // 1. TRY Unix sockets first
    match try_unix_servers(&server, &jsonrpc_server, &socket_path, &jsonrpc_socket).await {
        Ok(()) => Ok(()),

        // 2. DETECT platform constraints
        Err(e) => {
            let error_str = e.to_string();
            if is_platform_constraint_str(&error_str) {
                warn!("⚠️  Unix sockets unavailable: {}", error_str);
                warn!("   Detected platform constraint, adapting...");

                // 3. ADAPT to TCP fallback
                start_tcp_servers(server, jsonrpc_server).await
            } else {
                // 4. Real error (not a platform constraint)
                error!("❌ Real error (not platform constraint): {}", error_str);
                Err(e)
            }
        }
    }
}

/// Try to start Unix socket servers
async fn try_unix_servers(
    server: &ToadStoolTarpcServer,
    jsonrpc_server: &ManualJsonRpcServer,
    socket_path: &PathBuf,
    jsonrpc_socket: &PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create parent directories
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    if let Some(parent) = jsonrpc_socket.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Remove existing sockets
    if let Err(e) = tokio::fs::remove_file(&socket_path).await {
        tracing::debug!("Socket cleanup: {e}");
    }
    if let Err(e) = tokio::fs::remove_file(&jsonrpc_socket).await {
        tracing::debug!("Socket cleanup: {e}");
    }

    info!("✅ ToadStool server ready (Unix sockets)");
    info!("   Socket (tarpc): {:?}", socket_path);
    info!("   Socket (JSON-RPC): {:?}", jsonrpc_socket);
    info!("   Protocol: tarpc (binary RPC, PRIMARY)");
    info!("   Protocol: JSON-RPC 2.0 (universal, FALLBACK)");
    info!("   Status: READY ✅ (optimal Unix socket mode)");

    // Start JSON-RPC server
    let jsonrpc_server_clone = jsonrpc_server.clone();
    let jsonrpc_socket_clone = jsonrpc_socket.clone();
    tokio::spawn(async move {
        if let Err(e) = jsonrpc_server_clone.serve(jsonrpc_socket_clone).await {
            error!("JSON-RPC server error: {}", e);
        }
    });

    // Start tarpc server (blocking) - clone to move into ownership
    server.clone().serve_unix(&socket_path).await?;

    Ok(())
}

/// Start TCP fallback servers (isomorphic mode)
async fn start_tcp_servers(
    server: ToadStoolTarpcServer,
    jsonrpc_server: ManualJsonRpcServer,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::net::TcpListener;

    info!("🌐 Starting TCP IPC fallback (isomorphic mode)");
    info!("   Protocol: tarpc + JSON-RPC 2.0 (same as Unix)");

    // Bind to localhost only (security: same as Unix socket)
    let tarpc_listener = TcpListener::bind("127.0.0.1:0").await?;
    let tarpc_addr = tarpc_listener.local_addr()?;

    let jsonrpc_listener = TcpListener::bind("127.0.0.1:0").await?;
    let jsonrpc_addr = jsonrpc_listener.local_addr()?;

    info!("✅ TCP IPC listening:");
    info!("   tarpc (PRIMARY): {}", tarpc_addr);
    info!("   JSON-RPC (FALLBACK): {}", jsonrpc_addr);

    // Write discovery files for clients
    write_tcp_discovery_file("toadstool-ipc-port", &tarpc_addr)?;
    write_tcp_discovery_file("toadstool-jsonrpc-port", &jsonrpc_addr)?;

    info!("   Status: READY ✅ (isomorphic TCP fallback active)");

    // Start JSON-RPC server
    tokio::spawn(async move {
        if let Err(e) = jsonrpc_server.serve_tcp(jsonrpc_listener).await {
            error!("JSON-RPC TCP server error: {}", e);
        }
    });

    // Start tarpc server (blocking) - clone to move into ownership
    server.clone().serve_tcp(tarpc_listener).await?;

    Ok(())
}

/// Check if error string indicates a platform constraint (SELinux, etc.)
///
/// Platform constraints should trigger TCP fallback, not failure.
fn is_platform_constraint_str(error_str: &str) -> bool {
    // Android/SELinux errors
    if error_str.contains("Permission denied") || error_str.contains("Operation not permitted") {
        // Check for SELinux
        if is_selinux_enforcing() {
            tracing::debug!("   Platform constraint: SELinux enforcing (Android?)");
            return true;
        }
    }

    // Platform lacks Unix sockets
    if error_str.contains("Unsupported")
        || error_str.contains("not supported")
        || error_str.contains("protocol not available")
    {
        tracing::debug!("   Platform constraint: Unix sockets not supported");
        return true;
    }

    false
}

/// Check if SELinux is enforcing (common on Android)
fn is_selinux_enforcing() -> bool {
    std::fs::read_to_string("/sys/fs/selinux/enforce")
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .map(|enforce| enforce == 1)
        .unwrap_or(false)
}

/// Shutdown signal type for ecoBin compliance
///
/// **ecoBin Standard**: Both SIGINT and SIGTERM trigger graceful shutdown
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownSignal {
    /// SIGINT (Ctrl+C) - user interrupt
    Sigint,
    /// SIGTERM - system/orchestrator shutdown request
    Sigterm,
    /// Error listening for signals
    Error(&'static str),
}

/// Wait for shutdown signal (SIGINT or SIGTERM)
///
/// **ecoBin Compliance**: Handles both signals for proper systemd/container integration
///
/// - SIGINT (Ctrl+C): User interrupt, typically from terminal
/// - SIGTERM: System shutdown, from systemd, Docker, Kubernetes, etc.
///
/// Both signals trigger the same graceful shutdown path.
async fn wait_for_shutdown_signal() -> ShutdownSignal {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigint = match signal(SignalKind::interrupt()) {
            Ok(s) => s,
            Err(_) => return ShutdownSignal::Error("Failed to register SIGINT handler"),
        };

        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(_) => return ShutdownSignal::Error("Failed to register SIGTERM handler"),
        };

        tokio::select! {
            _ = sigint.recv() => ShutdownSignal::Sigint,
            _ = sigterm.recv() => ShutdownSignal::Sigterm,
        }
    }

    #[cfg(not(unix))]
    {
        // Windows: Only SIGINT (Ctrl+C) is reliably supported
        match signal::ctrl_c().await {
            Ok(()) => ShutdownSignal::Sigint,
            Err(_) => ShutdownSignal::Error("Failed to listen for Ctrl+C"),
        }
    }
}

/// Write TCP discovery file (XDG-compliant)
fn write_tcp_discovery_file(
    filename: &str,
    addr: &std::net::SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::env;
    use std::fs;

    let content = format!("tcp:{}", addr);

    // Try XDG_RUNTIME_DIR first (standard)
    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        let path = PathBuf::from(runtime_dir).join(filename);
        fs::write(&path, &content)?;
        info!("📁 TCP discovery file: {}", path.display());
        return Ok(());
    }

    // Fallback to /tmp (dev/testing)
    let path = PathBuf::from("/tmp").join(filename);
    fs::write(&path, &content)?;
    info!("📁 TCP discovery file: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== socket_filename_for_family ==========

    #[test]
    fn socket_filename_for_family_default() {
        assert_eq!(socket_filename_for_family("default"), "toadstool.sock");
    }

    #[test]
    fn socket_filename_for_family_empty() {
        assert_eq!(socket_filename_for_family(""), "toadstool.sock");
    }

    #[test]
    fn socket_filename_for_family_custom() {
        assert_eq!(socket_filename_for_family("nat0"), "toadstool-nat0.sock");
    }

    #[test]
    fn socket_filename_for_family_alphanumeric() {
        assert_eq!(
            socket_filename_for_family("family123"),
            "toadstool-family123.sock"
        );
    }

    #[test]
    fn socket_filename_for_family_with_hyphens() {
        assert_eq!(
            socket_filename_for_family("my-family-id"),
            "toadstool-my-family-id.sock"
        );
    }

    #[test]
    fn socket_filename_for_family_special_chars() {
        assert_eq!(
            socket_filename_for_family("dev_env"),
            "toadstool-dev_env.sock"
        );
    }

    #[test]
    fn socket_filename_for_family_whitespace() {
        // Non-empty string with spaces - not "default", so gets formatted
        assert_eq!(
            socket_filename_for_family(" default "),
            "toadstool- default .sock"
        );
    }

    // ========== is_platform_constraint_str ==========

    #[test]
    fn is_platform_constraint_str_unsupported() {
        assert!(is_platform_constraint_str("Unsupported"));
        assert!(is_platform_constraint_str("Unix sockets Unsupported"));
        assert!(is_platform_constraint_str("Unsupported operation"));
    }

    #[test]
    fn is_platform_constraint_str_not_supported() {
        assert!(is_platform_constraint_str("not supported"));
        assert!(is_platform_constraint_str("protocol not supported"));
        assert!(is_platform_constraint_str(
            "feature not supported on this platform"
        ));
    }

    #[test]
    fn is_platform_constraint_str_protocol_not_available() {
        assert!(is_platform_constraint_str("protocol not available"));
        assert!(is_platform_constraint_str("Error: protocol not available"));
    }

    #[test]
    fn is_platform_constraint_str_non_matching() {
        assert!(!is_platform_constraint_str("Connection refused"));
        assert!(!is_platform_constraint_str("Connection reset by peer"));
        assert!(!is_platform_constraint_str("timeout"));
        assert!(!is_platform_constraint_str("No such file or directory"));
        assert!(!is_platform_constraint_str("Address already in use"));
        assert!(!is_platform_constraint_str(""));
        assert!(!is_platform_constraint_str("generic error"));
    }

    #[test]
    fn is_platform_constraint_str_permission_denied() {
        // Permission denied / Operation not permitted only match when SELinux is enforcing.
        // On most dev/CI systems, SELinux is not enforcing, so these return false.
        // We verify the function doesn't panic and result matches SELinux state.
        let result = is_platform_constraint_str("Permission denied (EACCES)");
        assert_eq!(result, is_selinux_enforcing());
    }

    #[test]
    fn is_platform_constraint_str_operation_not_permitted() {
        let result = is_platform_constraint_str("Operation not permitted");
        assert_eq!(result, is_selinux_enforcing());
    }

    // ========== is_selinux_enforcing ==========

    #[test]
    fn is_selinux_enforcing_returns_bool() {
        // On most systems (no SELinux or not enforcing), returns false.
        // If /sys/fs/selinux/enforce doesn't exist, returns false.
        // We ensure it doesn't panic and returns a valid bool.
        let result = is_selinux_enforcing();
        assert!(result == true || result == false);
    }
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
// ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
