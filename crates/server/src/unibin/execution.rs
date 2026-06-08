// SPDX-License-Identifier: AGPL-3.0-or-later
//! Server execution lifecycle: executor creation, server startup, shutdown

use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::CoordinatorExecutor;
use crate::errors::{ServerError, ServerResult};
use crate::glowplug_client::discover_gpu_bdfs;
use crate::pure_jsonrpc::{JsonRpcHandler, serve_tcp, serve_unix, serve_unix_prebound};
use crate::tarpc_server::{StandaloneExecutor, ToadStoolTarpcServer, WorkloadExecutorDispatch};

use super::capabilities;
use toadstool_common::constants::platform_paths::sysfs;
use toadstool_common::interned_strings::socket_env;
use toadstool_distributed::{DistributedConfig, StandaloneConfig};

/// Defaults for UniBin coordinator integration when corresponding env vars are unset.
mod unibin_execution_defaults {
    /// Default maximum concurrent workload executions for the embedded standalone executor.
    pub const DEFAULT_MAX_CONCURRENT_WORKLOADS: u32 = 10;
    /// Default wall-clock timeout for workload execution (seconds).
    pub const DEFAULT_WORKLOAD_TIMEOUT_SECS: u64 = 300;
    /// Default cap for the local job queue when job queuing is enabled.
    pub const DEFAULT_MAX_JOB_QUEUE_SIZE: usize = 100;
    /// Interval between coordination health reports (seconds).
    pub const DEFAULT_COORDINATION_HEALTH_REPORT_INTERVAL_SECS: u64 = 60;
}

/// Snapshot of UniBin networking and coordination options (read once at process startup).
#[derive(Debug, Clone)]
pub struct UnibinExecutionConfig {
    /// Host for `host:0` TCP bind fallback and `--port` JSON-RPC binds (`TOADSTOOL_BIND_ADDRESS`).
    pub bind_host: String,
    /// Full `host:port` override for TCP IPC when Unix sockets are unavailable (`TOADSTOOL_TCP_BIND_ADDRESS`).
    pub tcp_bind_address: Option<String>,
    /// When false, use standalone executor (`TOADSTOOL_STANDALONE`).
    pub use_distributed: bool,
    /// Optional bearer token for coordination plane registration.
    pub coordination_auth_token: Option<String>,
    /// Maximum concurrent executions (`TOADSTOOL_MAX_CONCURRENT_EXECUTIONS`).
    pub max_concurrent_executions: u32,
    /// Default workload timeout in seconds (`TOADSTOOL_EXECUTION_TIMEOUT`).
    pub default_timeout_secs: u64,
    /// Whether the embedded job queue is enabled.
    pub enable_job_queue: bool,
    /// Maximum queued jobs when [`Self::enable_job_queue`] is true.
    pub max_queue_size: usize,
    /// Seconds between coordination health reports.
    pub health_reporting_interval_secs: u64,
    /// Skip hardware probes (GPU/NPU discovery) for headless deployment.
    pub headless: bool,

    /// Launcher-injected transport endpoint (sourDough standard).
    /// When set, the server uses this transport instead of self-binding.
    pub transport_endpoint: Option<toadstool_common::transport_endpoint::TransportEndpoint>,
}

impl UnibinExecutionConfig {
    /// Load UniBin execution settings from the process environment (call once at startup).
    #[must_use]
    #[expect(deprecated, reason = "reads legacy SONGBIRD_AUTH_TOKEN as backward-compat fallback")]
    pub fn from_env() -> Self {
        let bind_host = std::env::var(socket_env::TOADSTOOL_BIND_ADDRESS)
            .unwrap_or_else(|_| toadstool_common::constants::network::LOCALHOST_IPV4.into());
        let tcp_bind_address = std::env::var(socket_env::TOADSTOOL_TCP_BIND_ADDRESS).ok();
        let use_distributed = std::env::var(socket_env::TOADSTOOL_STANDALONE)
            .map_or(true, |v| v != "1" && v.to_lowercase() != "true");
        let coordination_auth_token = std::env::var(socket_env::COORDINATION_AUTH_TOKEN)
            .ok()
            .or_else(|| {
                std::env::var(socket_env::LEGACY_SONGBIRD_AUTH_TOKEN)
                    .ok()
                    .map(|v| {
                        tracing::warn!(
                            env_var = %socket_env::LEGACY_SONGBIRD_AUTH_TOKEN,
                            value = %v,
                            "deprecated LEGACY env variable used — migrate to capability-based discovery"
                        );
                        v
                    })
            });
        let max_concurrent_executions =
            std::env::var(socket_env::TOADSTOOL_MAX_CONCURRENT_EXECUTIONS)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(unibin_execution_defaults::DEFAULT_MAX_CONCURRENT_WORKLOADS);
        let default_timeout_secs = std::env::var(socket_env::TOADSTOOL_EXECUTION_TIMEOUT)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(unibin_execution_defaults::DEFAULT_WORKLOAD_TIMEOUT_SECS);

        Self {
            bind_host,
            tcp_bind_address,
            use_distributed,
            coordination_auth_token,
            max_concurrent_executions,
            default_timeout_secs,
            enable_job_queue: true,
            max_queue_size: unibin_execution_defaults::DEFAULT_MAX_JOB_QUEUE_SIZE,
            health_reporting_interval_secs:
                unibin_execution_defaults::DEFAULT_COORDINATION_HEALTH_REPORT_INTERVAL_SECS,
            headless: std::env::var("TOADSTOOL_HEADLESS")
                .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
            transport_endpoint: toadstool_common::transport_endpoint::TransportEndpoint::from_env()
                .unwrap_or_else(|e| {
                    tracing::warn!("invalid TRANSPORT_ENDPOINT: {e}");
                    None
                }),
        }
    }

    /// `host:0` bind address for OS-assigned TCP port (TCP fallback when Unix sockets unavailable).
    #[must_use]
    pub fn bind_any_os_port(&self) -> String {
        format!("{}:0", self.bind_host)
    }

    /// Effective TCP bind address for IPC fallback: explicit `TOADSTOOL_TCP_BIND_ADDRESS` or [`Self::bind_any_os_port`].
    #[must_use]
    pub fn tcp_ipc_bind_addr(&self) -> String {
        self.tcp_bind_address
            .clone()
            .unwrap_or_else(|| self.bind_any_os_port())
    }
}

/// Create executor with distributed or standalone mode
///
/// # Errors
///
/// Returns `ServerError::Initialization` if the distributed coordinator or standalone executor
/// cannot be created.
pub async fn create_executor(
    family_id: &str,
    cfg: &UnibinExecutionConfig,
) -> Result<std::sync::Arc<WorkloadExecutorDispatch>, ServerError> {
    info!("Creating executor with distributed coordinator (isomorphic/fractal)");

    if cfg.use_distributed {
        info!("Initializing distributed coordinator mode");
        let capabilities = if cfg.headless {
            capabilities::query_baseline_only().await
        } else {
            capabilities::query_local_capabilities().await
        };
        info!("Local capabilities: {:?}", capabilities);

        let socket_env = toadstool_common::primal_sockets::SocketPathEnv::from_env();
        let config = DistributedConfig {
            instance_id: format!("toadstool-{family_id}"),
            standalone: StandaloneConfig {
                max_concurrent_executions: cfg.max_concurrent_executions,
                default_timeout_secs: cfg.default_timeout_secs,
                enable_job_queue: cfg.enable_job_queue,
                max_queue_size: cfg.max_queue_size,
            },
            coordination: Some(toadstool_distributed::CoordinationConfig {
                endpoint: socket_env
                    .coordination_connection_hint
                    .clone()
                    .unwrap_or_else(|| {
                        tracing::info!(
                            "No coordination endpoint configured, will use runtime discovery"
                        );
                        String::new()
                    }),
                auth_token: cfg.coordination_auth_token.clone(),
                health_reporting_interval_secs: cfg.health_reporting_interval_secs,
            }),
        };

        let service_id = format!("toadstool-{family_id}");
        let executor = CoordinatorExecutor::new(config, service_id)
            .await
            .map_err(|e| {
                ServerError::Initialization(format!("Failed to create coordinator executor: {e}"))
            })?;

        info!("✅ Distributed coordinator executor ready");
        Ok(std::sync::Arc::new(WorkloadExecutorDispatch::Coordinator(
            executor,
        )))
    } else {
        info!("Using standalone executor (TOADSTOOL_STANDALONE=1)");
        let capabilities = capabilities::query_local_capabilities().await;
        info!("Local capabilities: {:?}", capabilities);
        Ok(std::sync::Arc::new(WorkloadExecutorDispatch::Standalone(
            StandaloneExecutor::new(),
        )))
    }
}

/// Start servers with Unix socket or TCP fallback.
///
/// When `jsonrpc_listener` is `Some`, the JSON-RPC server uses the pre-bound
/// listener (Wave 49 fast-bind optimization). Otherwise it binds its own.
///
/// # Errors
///
/// Returns `ServerError` if Unix or TCP server startup fails.
pub async fn start_servers_with_fallback(
    server: ToadStoolTarpcServer,
    jsonrpc_handler: Arc<JsonRpcHandler>,
    socket_path: PathBuf,
    jsonrpc_socket: PathBuf,
    tcp_port: Option<u16>,
    cfg: &UnibinExecutionConfig,
    jsonrpc_listener: Option<Arc<tokio::net::UnixListener>>,
) -> ServerResult<()> {
    // Transport injection: if TRANSPORT_ENDPOINT is set, use the injected transport
    if let Some(ref te) = cfg.transport_endpoint {
        use toadstool_common::transport_endpoint::TransportEndpoint;
        info!("🔌 TRANSPORT_ENDPOINT injected: {te}");
        return match te {
            TransportEndpoint::Uds { path } => {
                try_unix_servers(&server, &jsonrpc_handler, &socket_path, path, jsonrpc_listener).await
            }
            TransportEndpoint::Tcp { host, port } => {
                info!("   Launcher-injected TCP: {host}:{port}");
                start_tcp_jsonrpc_on_port(Arc::clone(&jsonrpc_handler), *port, host.clone()).await
            }
            TransportEndpoint::MeshRelay { peer_id, capability } => {
                info!("   Mesh relay transport not yet wired (peer={peer_id}, cap={capability})");
                Err(ServerError::Network(
                    "mesh_relay transport not yet supported".into(),
                ))
            }
        };
    }

    if let Some(port) = tcp_port {
        info!("   --port {port} specified: starting TCP JSON-RPC (UniBin standard)");
        let tcp_handler = Arc::clone(&jsonrpc_handler);
        let bind_host = cfg.bind_host.clone();
        tokio::spawn(async move {
            if let Err(e) = start_tcp_jsonrpc_on_port(tcp_handler, port, bind_host).await {
                error!("TCP JSON-RPC on port {port} failed: {e}");
            }
        });
    }

    info!("   Trying Unix socket IPC (optimal)...");

    match try_unix_servers(&server, &jsonrpc_handler, &socket_path, &jsonrpc_socket, jsonrpc_listener).await {
        Ok(()) => Ok(()),
        Err(e) => {
            let error_str = e.to_string();
            if is_platform_constraint_str(&error_str) {
                warn!("⚠️  Unix sockets unavailable: {}", error_str);
                warn!("   Detected platform constraint, adapting...");
                start_tcp_servers(server, jsonrpc_handler, cfg).await
            } else {
                error!("❌ Real error (not platform constraint): {}", error_str);
                Err(e)
            }
        }
    }
}

async fn try_unix_servers(
    server: &ToadStoolTarpcServer,
    jsonrpc_handler: &Arc<JsonRpcHandler>,
    socket_path: &PathBuf,
    jsonrpc_socket: &PathBuf,
    jsonrpc_listener: Option<Arc<tokio::net::UnixListener>>,
) -> ServerResult<()> {
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ServerError::Initialization(e.to_string()))?;
    }
    if jsonrpc_listener.is_none()
        && let Some(parent) = jsonrpc_socket.parent()
    {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ServerError::Initialization(e.to_string()))?;
    }

    if let Err(e) = tokio::fs::remove_file(socket_path).await {
        tracing::debug!("Socket cleanup: {e}");
    }
    if jsonrpc_listener.is_none()
        && let Err(e) = tokio::fs::remove_file(jsonrpc_socket).await
    {
        tracing::debug!("Socket cleanup: {e}");
    }

    info!("✅ ToadStool server ready (Unix sockets)");
    info!("   Socket (JSON-RPC): {:?}", jsonrpc_socket);
    info!("   Socket (tarpc): {:?}", socket_path);

    write_fleet_file(&discover_gpu_bdfs());

    let jsonrpc_handler = Arc::clone(jsonrpc_handler);
    if let Some(listener) = jsonrpc_listener {
        tokio::spawn(async move {
            if let Err(e) = serve_unix_prebound(jsonrpc_handler, listener).await {
                error!("JSON-RPC server error: {}", e);
            }
        });
    } else {
        let jsonrpc_socket_clone = jsonrpc_socket.clone();
        tokio::spawn(async move {
            if let Err(e) = serve_unix(jsonrpc_handler, jsonrpc_socket_clone).await {
                error!("JSON-RPC server error: {}", e);
            }
        });
    }

    server.clone().serve_unix(socket_path).await?;
    Ok(())
}

async fn start_tcp_servers(
    server: ToadStoolTarpcServer,
    jsonrpc_handler: Arc<JsonRpcHandler>,
    cfg: &UnibinExecutionConfig,
) -> ServerResult<()> {
    use tokio::net::TcpListener;

    info!("🌐 Starting TCP IPC fallback (isomorphic mode)");

    let bind_addr = cfg.tcp_ipc_bind_addr();

    let tarpc_listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;
    let tarpc_addr = tarpc_listener
        .local_addr()
        .map_err(|e| ServerError::Network(e.to_string()))?;

    let jsonrpc_listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;
    let jsonrpc_addr = jsonrpc_listener
        .local_addr()
        .map_err(|e| ServerError::Network(e.to_string()))?;

    info!("✅ TCP IPC listening:");
    info!("   JSON-RPC (PRIMARY): {}", jsonrpc_addr);
    info!("   tarpc (OPTIONAL): {}", tarpc_addr);

    write_tcp_discovery_file("toadstool-ipc-port", &tarpc_addr)?;
    write_tcp_discovery_file("toadstool-jsonrpc-port", &jsonrpc_addr)?;

    write_fleet_file(&discover_gpu_bdfs());

    tokio::spawn(async move {
        if let Err(e) = serve_tcp(jsonrpc_handler, jsonrpc_listener).await {
            error!("JSON-RPC TCP server error: {}", e);
        }
    });

    server.clone().serve_tcp(tarpc_listener).await?;
    Ok(())
}

/// Start a TCP JSON-RPC listener bound to a specific port (UniBin `--port` support).
///
/// Newline-delimited JSON-RPC per `PRIMAL_IPC_PROTOCOL.md`.
async fn start_tcp_jsonrpc_on_port(
    handler: Arc<JsonRpcHandler>,
    port: u16,
    bind_host: String,
) -> ServerResult<()> {
    use tokio::net::TcpListener;

    let addr = format!("{bind_host}:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| ServerError::Network(format!("--port {port} bind failed: {e}")))?;
    let local = listener
        .local_addr()
        .map_err(|e| ServerError::Network(e.to_string()))?;

    info!("✅ TCP JSON-RPC (--port): {local}");
    write_tcp_discovery_file("toadstool-jsonrpc-port", &local)?;

    serve_tcp(handler, listener).await
}

/// Returns true if the error string indicates a platform constraint (e.g. SELinux, unsupported sockets).
pub fn is_platform_constraint_str(error_str: &str) -> bool {
    if (error_str.contains("Permission denied") || error_str.contains("Operation not permitted"))
        && is_selinux_enforcing()
    {
        tracing::debug!("   Platform constraint: SELinux enforcing (Android?)");
        return true;
    }

    if error_str.contains("Unsupported")
        || error_str.contains("not supported")
        || error_str.contains("protocol not available")
    {
        tracing::debug!("   Platform constraint: Unix sockets not supported");
        return true;
    }

    false
}

/// Returns true if SELinux is in enforcing mode.
pub fn is_selinux_enforcing() -> bool {
    std::fs::read_to_string(sysfs::FS_SELINUX_ENFORCE)
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .is_some_and(|enforce| enforce == 1)
}

fn write_fleet_file(devices: &[String]) {
    let runtime_dir =
        std::env::var(socket_env::XDG_RUNTIME_DIR).unwrap_or_else(|_| "/tmp".to_string());
    let fleet_dir = std::path::PathBuf::from(&runtime_dir).join("biomeos");
    let fleet_path = fleet_dir.join("toadstool-ember-fleet.json");

    if let Err(e) = std::fs::create_dir_all(&fleet_dir) {
        tracing::warn!("Failed to create fleet dir {}: {e}", fleet_dir.display());
        return;
    }

    let mut routes = serde_json::Map::new();
    let socket_path = fleet_dir.join("compute.sock");
    for bdf in devices {
        routes.insert(
            bdf.clone(),
            serde_json::Value::String(socket_path.to_string_lossy().into_owned()),
        );
    }

    let fleet = serde_json::json!({
        "mode": "fleet",
        "routes": routes,
        "standby_count": 0,
        "devices": devices.iter().map(|bdf| {
            serde_json::json!({
                "bdf": bdf,
                "socket": socket_path.to_string_lossy(),
                "vendor": "NVIDIA Corporation",
                "health": "Alive",
                "physics_domains": ["default"],
                "hot_standby_of": null,
                "experiment_dirty": false,
                "needs_warm_cycle": false
            })
        }).collect::<Vec<_>>(),
    });

    match serde_json::to_string_pretty(&fleet) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&fleet_path, json) {
                tracing::warn!("Failed to write fleet file {}: {e}", fleet_path.display());
            } else {
                tracing::info!("Fleet file written: {}", fleet_path.display());
            }
        }
        Err(e) => tracing::warn!("Failed to serialize fleet file: {e}"),
    }
}

/// Wait for shutdown signal (SIGINT or SIGTERM)
pub async fn wait_for_shutdown_signal() -> super::ShutdownSignal {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let Ok(mut sigint) = signal(SignalKind::interrupt()) else {
            return super::ShutdownSignal::Error("Failed to register SIGINT handler");
        };

        let Ok(mut sigterm) = signal(SignalKind::terminate()) else {
            return super::ShutdownSignal::Error("Failed to register SIGTERM handler");
        };

        tokio::select! {
            _ = sigint.recv() => super::ShutdownSignal::Sigint,
            _ = sigterm.recv() => super::ShutdownSignal::Sigterm,
        }
    }

    #[cfg(not(unix))]
    {
        match tokio::signal::ctrl_c().await {
            Ok(()) => super::ShutdownSignal::Sigint,
            Err(_) => super::ShutdownSignal::Error("Failed to listen for Ctrl+C"),
        }
    }
}

/// Write TCP discovery file for service discovery.
///
/// # Errors
///
/// Returns `ServerError::Internal` if the discovery file cannot be written.
pub fn write_tcp_discovery_file(filename: &str, addr: &std::net::SocketAddr) -> ServerResult<()> {
    use std::env;
    use std::fs;

    let content = format!("tcp:{addr}");

    if let Ok(runtime_dir) = env::var(socket_env::XDG_RUNTIME_DIR) {
        let path = PathBuf::from(runtime_dir).join(filename);
        fs::write(&path, &content).map_err(|e| ServerError::Internal(e.to_string()))?;
        info!("📁 TCP discovery file: {}", path.display());
        return Ok(());
    }

    let path = env::temp_dir().join(filename);
    fs::write(&path, &content).map_err(|e| ServerError::Internal(e.to_string()))?;
    info!("📁 TCP discovery file: {}", path.display());
    Ok(())
}

#[cfg(test)]
#[path = "execution_tests.rs"]
mod tests;
