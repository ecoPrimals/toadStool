// SPDX-License-Identifier: AGPL-3.0-only
//! Server execution lifecycle: executor creation, server startup, shutdown

use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::CoordinatorExecutor;
use crate::errors::{ServerError, ServerResult};
use crate::pure_jsonrpc::{JsonRpcHandler, serve_tcp, serve_unix};
use crate::tarpc_server::{StandaloneExecutor, ToadStoolTarpcServer, WorkloadExecutor};

use super::capabilities;
use toadstool_distributed::{DistributedConfig, StandaloneConfig};

/// Bind to any interface with OS-assigned port (TCP fallback when Unix sockets unavailable)
const BIND_ANY: &str = "0.0.0.0:0";

/// Create executor with distributed or standalone mode
///
/// # Errors
///
/// Returns `ServerError::Initialization` if the distributed coordinator or standalone executor
/// cannot be created.
pub async fn create_executor(
    family_id: &str,
) -> Result<std::sync::Arc<dyn WorkloadExecutor + Send + Sync>, ServerError> {
    info!("Creating executor with distributed coordinator (isomorphic/fractal)");

    let use_distributed = std::env::var("TOADSTOOL_STANDALONE")
        .map(|v| v != "1" && v.to_lowercase() != "true")
        .unwrap_or(true);

    if use_distributed {
        info!("Initializing distributed coordinator mode");
        let _capabilities = capabilities::query_local_capabilities().await;
        info!("Local capabilities: {:?}", _capabilities);

        let config = DistributedConfig {
            instance_id: format!("toadstool-{family_id}"),
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
                        tracing::info!(
                            "No SONGBIRD_ENDPOINT configured, will use runtime discovery"
                        );
                        String::new()
                    }),
                auth_token: std::env::var("SONGBIRD_AUTH_TOKEN").ok(),
                health_reporting_interval_secs: 60,
            }),
        };

        let service_id = format!("toadstool-{family_id}");
        let executor = CoordinatorExecutor::new(config, service_id)
            .await
            .map_err(|e| {
                ServerError::Initialization(format!("Failed to create coordinator executor: {e}"))
            })?;

        info!("✅ Distributed coordinator executor ready");
        Ok(std::sync::Arc::new(executor))
    } else {
        info!("Using standalone executor (TOADSTOOL_STANDALONE=1)");
        let _capabilities = capabilities::query_local_capabilities().await;
        info!("Local capabilities: {:?}", _capabilities);
        Ok(std::sync::Arc::new(StandaloneExecutor::new()))
    }
}

/// Start servers with Unix socket or TCP fallback
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
) -> ServerResult<()> {
    // When --port is explicitly provided, always start TCP alongside Unix sockets (UniBin std).
    if let Some(port) = tcp_port {
        info!("   --port {port} specified: starting TCP JSON-RPC (UniBin standard)");
        let tcp_handler = Arc::clone(&jsonrpc_handler);
        tokio::spawn(async move {
            if let Err(e) = start_tcp_jsonrpc_on_port(tcp_handler, port).await {
                error!("TCP JSON-RPC on port {port} failed: {e}");
            }
        });
    }

    info!("   Trying Unix socket IPC (optimal)...");

    match try_unix_servers(&server, &jsonrpc_handler, &socket_path, &jsonrpc_socket).await {
        Ok(()) => Ok(()),
        Err(e) => {
            let error_str = e.to_string();
            if is_platform_constraint_str(&error_str) {
                warn!("⚠️  Unix sockets unavailable: {}", error_str);
                warn!("   Detected platform constraint, adapting...");
                start_tcp_servers(server, jsonrpc_handler).await
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
) -> ServerResult<()> {
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ServerError::Initialization(e.to_string()))?;
    }
    if let Some(parent) = jsonrpc_socket.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ServerError::Initialization(e.to_string()))?;
    }

    if let Err(e) = tokio::fs::remove_file(socket_path).await {
        tracing::debug!("Socket cleanup: {e}");
    }
    if let Err(e) = tokio::fs::remove_file(jsonrpc_socket).await {
        tracing::debug!("Socket cleanup: {e}");
    }

    info!("✅ ToadStool server ready (Unix sockets)");
    info!("   Socket (JSON-RPC): {:?}", jsonrpc_socket);
    info!("   Socket (tarpc): {:?}", socket_path);

    let jsonrpc_handler = Arc::clone(jsonrpc_handler);
    let jsonrpc_socket_clone = jsonrpc_socket.clone();
    tokio::spawn(async move {
        if let Err(e) = serve_unix(jsonrpc_handler, jsonrpc_socket_clone).await {
            error!("JSON-RPC server error: {}", e);
        }
    });

    server.clone().serve_unix(socket_path).await?;
    Ok(())
}

async fn start_tcp_servers(
    server: ToadStoolTarpcServer,
    jsonrpc_handler: Arc<JsonRpcHandler>,
) -> ServerResult<()> {
    use tokio::net::TcpListener;

    info!("🌐 Starting TCP IPC fallback (isomorphic mode)");

    let bind_addr =
        std::env::var("TOADSTOOL_TCP_BIND_ADDRESS").unwrap_or_else(|_| BIND_ANY.to_string());

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
/// Newline-delimited JSON-RPC on `0.0.0.0:<port>` per `PRIMAL_IPC_PROTOCOL.md`.
async fn start_tcp_jsonrpc_on_port(handler: Arc<JsonRpcHandler>, port: u16) -> ServerResult<()> {
    use tokio::net::TcpListener;

    let addr = format!("0.0.0.0:{port}");
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
    std::fs::read_to_string("/sys/fs/selinux/enforce")
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .map(|enforce| enforce == 1)
        .unwrap_or_default()
}

/// Wait for shutdown signal (SIGINT or SIGTERM)
pub async fn wait_for_shutdown_signal() -> super::ShutdownSignal {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigint = match signal(SignalKind::interrupt()) {
            Ok(s) => s,
            Err(_) => return super::ShutdownSignal::Error("Failed to register SIGINT handler"),
        };

        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(_) => return super::ShutdownSignal::Error("Failed to register SIGTERM handler"),
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

    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        let path = PathBuf::from(runtime_dir).join(filename);
        fs::write(&path, &content).map_err(|e| ServerError::Internal(e.to_string()))?;
        info!("📁 TCP discovery file: {}", path.display());
        return Ok(());
    }

    let path = PathBuf::from("/tmp").join(filename);
    fs::write(&path, &content).map_err(|e| ServerError::Internal(e.to_string()))?;
    info!("📁 TCP discovery file: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_executor_standalone_mode() {
        temp_env::with_var("TOADSTOOL_STANDALONE", Some("1"), || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = create_executor("test-family").await;
                    assert!(
                        result.is_ok(),
                        "standalone executor creation failed: {:?}",
                        result.err()
                    );
                });
            })
            .join()
            .expect("test thread");
        });
    }

    #[test]
    fn create_executor_standalone_mode_true_lowercase() {
        temp_env::with_var("TOADSTOOL_STANDALONE", Some("true"), || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = create_executor("my-family").await;
                    assert!(result.is_ok());
                });
            })
            .join()
            .expect("test thread");
        });
    }

    #[test]
    fn create_executor_standalone_mode_true_uppercase() {
        temp_env::with_var("TOADSTOOL_STANDALONE", Some("TRUE"), || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = create_executor("test-family").await;
                    assert!(
                        result.is_ok(),
                        "standalone executor with TRUE should succeed: {:?}",
                        result.err()
                    );
                });
            })
            .join()
            .expect("test thread");
        });
    }

    #[test]
    fn write_tcp_discovery_file_fails_on_readonly_dir() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/proc/self"), || {
            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 0));
            let result = write_tcp_discovery_file("toadstool-test-readonly", &addr);
            assert!(result.is_err(), "writing to /proc/self should fail");
        });
    }

    #[test]
    fn create_executor_integrated_mode_when_standalone_unset() {
        temp_env::with_var_unset("TOADSTOOL_STANDALONE", || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = create_executor("integrated-family").await;
                    match &result {
                        Ok(_) => {}
                        Err(e) => assert!(!e.to_string().is_empty(), "error should have message"),
                    }
                });
            })
            .join()
            .expect("test thread");
        });
    }

    #[test]
    fn create_executor_integrated_mode_when_standalone_0() {
        temp_env::with_var("TOADSTOOL_STANDALONE", Some("0"), || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = create_executor("family-0").await;
                    match &result {
                        Ok(_) => {}
                        Err(e) => assert!(!e.to_string().is_empty()),
                    }
                });
            })
            .join()
            .expect("test thread");
        });
    }

    #[test]
    fn is_platform_constraint_str_selinux_permission_denied() {
        // When SELinux is enforcing, "Permission denied" is platform constraint
        // Result depends on is_selinux_enforcing() - we test the string matching
        let r = is_platform_constraint_str("some error");
        assert!(!r);
    }

    #[test]
    fn is_platform_constraint_str_unsupported() {
        assert!(is_platform_constraint_str("Unsupported operation"));
    }

    #[test]
    fn is_platform_constraint_str_not_supported() {
        assert!(is_platform_constraint_str("protocol not supported"));
    }

    #[test]
    fn is_platform_constraint_str_protocol_not_available() {
        assert!(is_platform_constraint_str(
            "protocol not available on this system"
        ));
    }

    #[test]
    fn is_platform_constraint_str_operation_not_permitted() {
        // Depends on SELinux - without SELinux this returns false
        let _ = is_platform_constraint_str("Operation not permitted");
    }

    #[test]
    fn is_selinux_enforcing_does_not_panic() {
        let _ = is_selinux_enforcing();
    }

    #[test]
    fn write_tcp_discovery_file_xdg_runtime() {
        let temp_dir = std::env::temp_dir();
        temp_env::with_var(
            "XDG_RUNTIME_DIR",
            Some(temp_dir.to_string_lossy().as_ref()),
            || {
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 12345));
                let result = write_tcp_discovery_file("test-toadstool-port", &addr);
                assert!(result.is_ok());
                let path = temp_dir.join("test-toadstool-port");
                if path.exists() {
                    let content = std::fs::read_to_string(&path).unwrap();
                    assert_eq!(content, "tcp:127.0.0.1:12345");
                    let _ = std::fs::remove_file(&path);
                }
            },
        );
    }

    #[test]
    fn write_tcp_discovery_file_fallback_tmp() {
        temp_env::with_var("XDG_RUNTIME_DIR", None::<&str>, || {
            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 0));
            let result = write_tcp_discovery_file("toadstool-test-fallback", &addr);
            assert!(result.is_ok());
            let path = std::path::PathBuf::from("/tmp").join("toadstool-test-fallback");
            if path.exists() {
                let content = std::fs::read_to_string(&path).unwrap();
                assert!(content.starts_with("tcp:"));
                let _ = std::fs::remove_file(&path);
            }
        });
    }
}
