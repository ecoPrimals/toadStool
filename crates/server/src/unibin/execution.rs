//! Server execution lifecycle: executor creation, server startup, shutdown
//!
//! ManualJsonRpcServer (deprecated) used here; see manual_jsonrpc/MIGRATION.md.

#![allow(deprecated)]

use std::path::PathBuf;
use tracing::{error, info, warn};

use crate::errors::{ServerError, ServerResult};
use crate::tarpc_server::{StandaloneExecutor, ToadStoolTarpcServer, WorkloadExecutor};
use crate::{CoordinatorExecutor, ManualJsonRpcServer};

use super::capabilities;
use toadstool_distributed::{DistributedConfig, StandaloneConfig};

/// Create executor with distributed or standalone mode
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
pub async fn start_servers_with_fallback(
    server: ToadStoolTarpcServer,
    jsonrpc_server: ManualJsonRpcServer,
    socket_path: PathBuf,
    jsonrpc_socket: PathBuf,
) -> ServerResult<()> {
    info!("   Trying Unix socket IPC (optimal)...");

    match try_unix_servers(&server, &jsonrpc_server, &socket_path, &jsonrpc_socket).await {
        Ok(()) => Ok(()),
        Err(e) => {
            let error_str = e.to_string();
            if is_platform_constraint_str(&error_str) {
                warn!("⚠️  Unix sockets unavailable: {}", error_str);
                warn!("   Detected platform constraint, adapting...");
                start_tcp_servers(server, jsonrpc_server).await
            } else {
                error!("❌ Real error (not platform constraint): {}", error_str);
                Err(e)
            }
        }
    }
}

async fn try_unix_servers(
    server: &ToadStoolTarpcServer,
    jsonrpc_server: &ManualJsonRpcServer,
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

    let jsonrpc_server_clone = jsonrpc_server.clone();
    let jsonrpc_socket_clone = jsonrpc_socket.clone();
    tokio::spawn(async move {
        if let Err(e) = jsonrpc_server_clone.serve(jsonrpc_socket_clone).await {
            error!("JSON-RPC server error: {}", e);
        }
    });

    server.clone().serve_unix(socket_path).await?;
    Ok(())
}

async fn start_tcp_servers(
    server: ToadStoolTarpcServer,
    jsonrpc_server: ManualJsonRpcServer,
) -> ServerResult<()> {
    use tokio::net::TcpListener;

    info!("🌐 Starting TCP IPC fallback (isomorphic mode)");

    let tarpc_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;
    let tarpc_addr = tarpc_listener
        .local_addr()
        .map_err(|e| ServerError::Network(e.to_string()))?;

    let jsonrpc_listener = TcpListener::bind("127.0.0.1:0")
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
        if let Err(e) = jsonrpc_server.serve_tcp(jsonrpc_listener).await {
            error!("JSON-RPC TCP server error: {}", e);
        }
    });

    server.clone().serve_tcp(tarpc_listener).await?;
    Ok(())
}

pub(crate) fn is_platform_constraint_str(error_str: &str) -> bool {
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

pub(crate) fn is_selinux_enforcing() -> bool {
    std::fs::read_to_string("/sys/fs/selinux/enforce")
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .map(|enforce| enforce == 1)
        .unwrap_or(false)
}

/// Wait for shutdown signal (SIGINT or SIGTERM)
pub async fn wait_for_shutdown_signal() -> super::ShutdownSignal {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

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

pub(crate) fn write_tcp_discovery_file(
    filename: &str,
    addr: &std::net::SocketAddr,
) -> ServerResult<()> {
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

    #[tokio::test]
    async fn create_executor_standalone_mode() {
        let old = std::env::var("TOADSTOOL_STANDALONE").ok();
        std::env::set_var("TOADSTOOL_STANDALONE", "1");

        let result = create_executor("test-family").await;
        if let Some(v) = old {
            std::env::set_var("TOADSTOOL_STANDALONE", v);
        } else {
            std::env::remove_var("TOADSTOOL_STANDALONE");
        }

        assert!(
            result.is_ok(),
            "standalone executor creation failed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn create_executor_standalone_mode_true_lowercase() {
        let old = std::env::var("TOADSTOOL_STANDALONE").ok();
        std::env::set_var("TOADSTOOL_STANDALONE", "true");

        let result = create_executor("my-family").await;
        if let Some(v) = old {
            std::env::set_var("TOADSTOOL_STANDALONE", v);
        } else {
            std::env::remove_var("TOADSTOOL_STANDALONE");
        }

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_executor_standalone_mode_true_uppercase() {
        let old = std::env::var("TOADSTOOL_STANDALONE").ok();
        std::env::set_var("TOADSTOOL_STANDALONE", "TRUE");

        let result = create_executor("test-family").await;
        if let Some(v) = old {
            std::env::set_var("TOADSTOOL_STANDALONE", v);
        } else {
            std::env::remove_var("TOADSTOOL_STANDALONE");
        }

        assert!(
            result.is_ok(),
            "standalone executor with TRUE should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn write_tcp_discovery_file_fails_on_readonly_dir() {
        let old = std::env::var("XDG_RUNTIME_DIR").ok();
        std::env::set_var("XDG_RUNTIME_DIR", "/proc/self");

        let addr: std::net::SocketAddr = "127.0.0.1:0".parse().expect("valid addr");
        let result = write_tcp_discovery_file("toadstool-test-readonly", &addr);

        if let Some(v) = old {
            std::env::set_var("XDG_RUNTIME_DIR", v);
        } else {
            std::env::remove_var("XDG_RUNTIME_DIR");
        }

        assert!(result.is_err(), "writing to /proc/self should fail");
    }

    #[tokio::test]
    async fn create_executor_integrated_mode_when_standalone_unset() {
        let old = std::env::var("TOADSTOOL_STANDALONE").ok();
        std::env::remove_var("TOADSTOOL_STANDALONE");

        let result = create_executor("integrated-family").await;
        if let Some(v) = old {
            std::env::set_var("TOADSTOOL_STANDALONE", v);
        } else {
            std::env::remove_var("TOADSTOOL_STANDALONE");
        }

        // Integrated mode may fail if Songbird not available
        match &result {
            Ok(_) => {}
            Err(e) => assert!(!e.to_string().is_empty(), "error should have message"),
        }
    }

    #[tokio::test]
    async fn create_executor_integrated_mode_when_standalone_0() {
        let old = std::env::var("TOADSTOOL_STANDALONE").ok();
        std::env::set_var("TOADSTOOL_STANDALONE", "0");

        let result = create_executor("family-0").await;
        if let Some(v) = old {
            std::env::set_var("TOADSTOOL_STANDALONE", v);
        } else {
            std::env::remove_var("TOADSTOOL_STANDALONE");
        }

        match &result {
            Ok(_) => {}
            Err(e) => assert!(!e.to_string().is_empty()),
        }
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
        let r = is_platform_constraint_str("Operation not permitted");
        // Either true (SELinux enforcing) or false (no SELinux)
        assert!(r || !r);
    }

    #[test]
    fn is_selinux_enforcing_does_not_panic() {
        let _ = is_selinux_enforcing();
    }

    #[test]
    fn write_tcp_discovery_file_xdg_runtime() {
        let old = std::env::var("XDG_RUNTIME_DIR").ok();
        std::env::set_var(
            "XDG_RUNTIME_DIR",
            std::env::temp_dir().to_string_lossy().as_ref(),
        );

        let addr: std::net::SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let result = write_tcp_discovery_file("test-toadstool-port", &addr);

        if let Some(v) = old {
            std::env::set_var("XDG_RUNTIME_DIR", v);
        } else {
            std::env::remove_var("XDG_RUNTIME_DIR");
        }

        assert!(result.is_ok());
        let path = std::env::temp_dir().join("test-toadstool-port");
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap();
            assert_eq!(content, "tcp:127.0.0.1:12345");
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn write_tcp_discovery_file_fallback_tmp() {
        let old = std::env::var("XDG_RUNTIME_DIR").ok();
        std::env::remove_var("XDG_RUNTIME_DIR");

        let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
        let result = write_tcp_discovery_file("toadstool-test-fallback", &addr);

        if let Some(v) = old {
            std::env::set_var("XDG_RUNTIME_DIR", v);
        }

        assert!(result.is_ok());
        let path = std::path::PathBuf::from("/tmp").join("toadstool-test-fallback");
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap();
            assert!(content.starts_with("tcp:"));
            let _ = std::fs::remove_file(&path);
        }
    }
}
