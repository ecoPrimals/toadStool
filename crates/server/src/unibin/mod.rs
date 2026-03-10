// SPDX-License-Identifier: AGPL-3.0-only
//! UniBin server entry point
//!
//! Shared server main logic for both toadstool and toadstool-server binaries

mod capabilities;
mod execution;
mod format;

// Re-export for integration tests (coverage)
pub use execution::{
    create_executor, is_platform_constraint_str, is_selinux_enforcing, start_servers_with_fallback,
    write_tcp_discovery_file,
};

use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Exit codes following uniBin/ecoBin standard
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

use crate::errors::ServerError;
use crate::pure_jsonrpc::JsonRpcHandler;
use crate::tarpc_server::ToadStoolTarpcServer;

/// Resolve family ID from CLI override or environment (testable helper)
#[must_use]
pub fn resolve_family_id(family_id_override: Option<String>) -> String {
    family_id_override
        .or_else(|| std::env::var("TOADSTOOL_FAMILY_ID").ok())
        .or_else(|| std::env::var("TOADSTOOL_FAMILY").ok())
        .or_else(|| std::env::var("BIOMEOS_FAMILY_ID").ok())
        .unwrap_or_else(|| {
            warn!("No family ID (CLI or env) set, using 'default'");
            warn!("For multi-instance support, use --family-id=nat0 or set one of:");
            warn!("  export TOADSTOOL_FAMILY_ID=nat0 (primal-specific)");
            warn!("  export BIOMEOS_FAMILY_ID=nat0 (orchestrator-provided)");
            "default".to_string()
        })
}

/// Resolve node ID from environment (testable helper)
#[must_use]
pub fn resolve_node_id() -> String {
    std::env::var("TOADSTOOL_NODE_ID").unwrap_or_else(|_| {
        info!("TOADSTOOL_NODE_ID not set, using 'default'");
        "default".to_string()
    })
}

/// Shutdown signal type for ecoBin compliance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownSignal {
    /// SIGINT (Ctrl+C) - user interrupt
    Sigint,
    /// SIGTERM - system/orchestrator shutdown request
    Sigterm,
    /// Error listening for signals
    Error(&'static str),
}

/// Run ToadStool in server/daemon mode
///
/// # Errors
///
/// Returns [`ServerError`] if socket path resolution, executor creation, or server startup fails.
pub async fn run_server_main(family_id_override: Option<String>) -> Result<(), ServerError> {
    info!(
        "🍄 ToadStool Universal Compute Server v{}",
        env!("CARGO_PKG_VERSION")
    );
    info!("CPU, GPU, Neuromorphic - Different orders of the same architecture");

    let family_id = resolve_family_id(family_id_override);
    let node_id = resolve_node_id();

    info!("Family ID: {}", family_id);
    info!("Node ID: {}", node_id);

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

    let socket_path = format::get_socket_path(&family_id, &node_id)?;
    info!("✅ Final socket path: {:?}", socket_path);

    info!("Initializing compute executor...");
    let executor = execution::create_executor(&family_id).await?;
    let version = env!("CARGO_PKG_VERSION").to_string();

    let error_count = Arc::new(AtomicU64::new(0));

    // Pass &version to server (borrow), move version to handler
    let server = ToadStoolTarpcServer::new(
        version.as_str(),
        Arc::clone(&executor),
        Some(Arc::clone(&error_count)),
    );

    info!("🌍 Attempting registration with Songbird discovery service...");
    match toadstool::ipc_helpers::register_with_songbird().await {
        Ok(()) => {
            info!("✅ Successfully registered with Songbird!");
        }
        Err(e) => {
            warn!("⚠️  Could not register with Songbird: {}", e);
            warn!("   Operating in standalone mode (no discovery)");
        }
    }

    // Scan biomeOS socket directory for stale sockets and discovered primals
    // (groundSpring V99 adaptive discovery pattern).
    let biomeos_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    if biomeos_dir.exists() {
        let mut discovered = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&biomeos_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("sock") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        discovered.push(name.to_string());
                    }
                }
            }
        }
        if discovered.is_empty() {
            info!("🔍 biomeOS socket dir exists but no primals discovered");
        } else {
            info!(
                "🔍 Discovered {} primal socket(s): {}",
                discovered.len(),
                discovered.join(", ")
            );
        }
    } else {
        info!("🔍 biomeOS socket dir not found (standalone mode)");
    }

    info!("🔌 Starting IPC servers (isomorphic mode)...");

    let jsonrpc_socket = socket_path.with_extension("jsonrpc.sock");
    let jsonrpc_handler = Arc::new(JsonRpcHandler::new(
        Arc::clone(&executor),
        version,
        Some(Arc::clone(&error_count)),
    ));

    let socket_path_for_server = socket_path.clone();
    let jsonrpc_socket_for_server = jsonrpc_socket.clone();

    let server_handle = tokio::spawn(async move {
        match execution::start_servers_with_fallback(
            server,
            jsonrpc_handler,
            socket_path_for_server,
            jsonrpc_socket_for_server,
        )
        .await
        {
            Ok(()) => info!("✅ Servers stopped gracefully"),
            Err(e) => error!("❌ Server error: {}", e),
        }
    });

    info!("Ready for shutdown (Ctrl+C or SIGTERM)");
    let shutdown_signal = execution::wait_for_shutdown_signal().await;

    match shutdown_signal {
        ShutdownSignal::Sigint => info!("📡 Received SIGINT (Ctrl+C)"),
        ShutdownSignal::Sigterm => info!("📡 Received SIGTERM (graceful shutdown)"),
        ShutdownSignal::Error(err) => error!("Failed to listen for shutdown signals: {}", err),
    }

    info!("Shutting down ToadStool server...");
    server_handle.abort();

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

#[cfg(test)]
mod tests;
