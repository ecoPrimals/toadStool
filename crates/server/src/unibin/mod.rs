// SPDX-License-Identifier: AGPL-3.0-or-later
//! UniBin server entry point
//!
//! Shared server main logic for both toadstool and toadstool-server binaries

mod capabilities;
mod execution;
mod format;
#[expect(unsafe_code, reason = "systemd fd store / sd_notify requires unsafe")]
pub(crate) mod systemd_fdstore;

// Re-exports: capability probe, execution helpers, socket layout (integration tests / coverage)
pub use capabilities::query_local_capabilities;
pub use execution::{
    UnibinExecutionConfig, create_executor, is_platform_constraint_str, is_selinux_enforcing,
    start_servers_with_fallback, write_tcp_discovery_file,
};
pub use format::{
    ensure_biomeos_directory, get_socket_path, legacy_socket_filename_for_family,
    socket_filename_for_family,
};

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
use toadstool_common::interned_strings::socket_env;

/// Resolve family ID from CLI override or environment (testable helper)
#[must_use]
pub fn resolve_family_id(family_id_override: Option<String>) -> String {
    family_id_override
        .or_else(|| std::env::var(socket_env::TOADSTOOL_FAMILY_ID).ok())
        .or_else(|| std::env::var(socket_env::TOADSTOOL_FAMILY).ok())
        .or_else(|| std::env::var(socket_env::BIOMEOS_FAMILY_ID).ok())
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
    std::env::var(socket_env::TOADSTOOL_NODE_ID).unwrap_or_else(|_| {
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
/// `bind_override` is an explicit `host:port` from `--bind` (takes precedence over
/// `--port` and `TOADSTOOL_BIND_ADDRESS`).
/// `tcp_port` enables newline-delimited JSON-RPC on the given TCP port (UniBin `--port`).
///
/// # Errors
///
/// Returns [`ServerError`] if socket path resolution, executor creation, or server startup fails.
pub async fn run_server_main(
    family_id_override: Option<String>,
    bind_override: Option<String>,
    tcp_port: Option<u16>,
    socket_override: Option<PathBuf>,
    biomeos_socket_override: Option<PathBuf>,
    headless: bool,
) -> Result<(), ServerError> {
    info!(
        "🍄 ToadStool Universal Compute Server v{}",
        env!("CARGO_PKG_VERSION")
    );
    info!("CPU, GPU, Neuromorphic - Different orders of the same architecture");

    // BTSP Protocol Standard §Compliance: refuse if FAMILY_ID + BIOMEOS_INSECURE=1
    if let Err(msg) = toadstool_common::primal_sockets::check_insecure_guard() {
        error!("🔒 {msg}");
        return Err(ServerError::Configuration(msg));
    }

    let family_id = resolve_family_id(family_id_override);
    let node_id = resolve_node_id();

    info!("Family ID: {}", family_id);
    info!("Node ID: {}", node_id);

    // BTSP awareness: log security posture
    let env = toadstool_common::primal_sockets::SocketPathEnv::from_env();
    if toadstool_common::primal_sockets::is_btsp_required(&env) {
        info!("🔒 BTSP mode: FAMILY_ID set — production security posture");
        info!("🔒 Family-scoped socket handshake expected on incoming connections");
    } else {
        info!("🔓 Development mode: no FAMILY_ID — BTSP handshake not required");
    }

    info!("🔍 Socket Path Discovery:");
    info!(
        "  Checking TOADSTOOL_SOCKET: {:?}",
        std::env::var(socket_env::TOADSTOOL_SOCKET).ok()
    );
    info!(
        "  Checking BIOMEOS_SOCKET_PATH: {:?}",
        std::env::var(socket_env::BIOMEOS_SOCKET_PATH).ok()
    );
    info!(
        "  Checking XDG_RUNTIME_DIR: {:?}",
        std::env::var(socket_env::XDG_RUNTIME_DIR).ok()
    );

    let jsonrpc_socket_path = format::get_socket_path(
        &family_id,
        &node_id,
        socket_override.as_deref(),
        biomeos_socket_override.as_deref(),
    )?;
    info!("✅ Final socket path: {:?}", jsonrpc_socket_path);

    // Wave 49/54 startup optimization: pre-bind JSON-RPC socket BEFORE heavy init
    // and start an early health responder so launchers get immediate health.liveness
    // responses while wgpu + mDNS + executor construction run in the background.
    let (early_stop_tx, early_stop_rx) = tokio::sync::watch::channel(false);
    let jsonrpc_listener =
        match crate::pure_jsonrpc::prebind_unix_listener(&jsonrpc_socket_path).await {
            Ok(listener) => {
                let listener = Arc::new(listener);
                let _early_health =
                    crate::pure_jsonrpc::spawn_early_health_responder(&listener, early_stop_rx);
                info!("⚡ JSON-RPC socket pre-bound — early health responder active");
                Some(listener)
            }
            Err(e) => {
                warn!("Pre-bind failed (will bind later): {e}");
                None
            }
        };

    let mut unibin_config = execution::UnibinExecutionConfig::from_env();

    // --bind host:port overrides both bind_host and tcp_port
    let tcp_port = if let Some(ref bind) = bind_override {
        if let Some((host, port_str)) = bind.rsplit_once(':') {
            if let Ok(port) = port_str.parse::<u16>() {
                host.clone_into(&mut unibin_config.bind_host);
                Some(port)
            } else {
                unibin_config.bind_host.clone_from(bind);
                tcp_port
            }
        } else {
            unibin_config.bind_host.clone_from(bind);
            tcp_port
        }
    } else {
        tcp_port
    };

    if headless {
        info!("🖥️ Headless mode: hardware probes disabled (pure-compute IPC server)");
        unibin_config.headless = true;
    }

    info!("Initializing compute executor...");
    let executor = execution::create_executor(&family_id, &unibin_config).await?;
    let version = env!("CARGO_PKG_VERSION").to_string();

    let error_count = Arc::new(AtomicU64::new(0));
    // PG-62: readiness flag — health.readiness returns "starting" until this is set
    let ready = Arc::new(AtomicBool::new(false));

    // Pass &version to server (borrow), move version to handler
    let server = ToadStoolTarpcServer::new(
        version.as_str(),
        Arc::clone(&executor),
        Some(Arc::clone(&error_count)),
    );

    info!("🔌 Starting IPC servers (isomorphic mode)...");

    // JSON-RPC (primary) uses the pre-bound socket path from above.
    let jsonrpc_socket = jsonrpc_socket_path;

    // tarpc (secondary) uses a separate socket to avoid bind collision (LD-05):
    //   compute-tarpc.sock / compute-{fid}-tarpc.sock
    let tarpc_filename = format::tarpc_socket_filename_for_family(&family_id);
    let tarpc_socket_path = jsonrpc_socket.parent().map_or_else(
        || jsonrpc_socket.with_extension("tarpc.sock"),
        |dir| dir.join(tarpc_filename),
    );

    // Legacy symlink: toadstool.sock → compute.sock for callers still using
    // primal-named discovery. Self-Knowledge v1.1 §Migration allows this.
    let legacy_filename = format::legacy_socket_filename_for_family(&family_id);
    let legacy_socket = tarpc_socket_path
        .parent()
        .map(|dir| dir.join(legacy_filename));
    if let Some(ref legacy) = legacy_socket
        && legacy != &tarpc_socket_path
    {
        let _ = std::fs::remove_file(legacy);
        #[cfg(unix)]
        {
            if let Err(e) = std::os::unix::fs::symlink(&tarpc_socket_path, legacy) {
                warn!(
                    "Could not create legacy symlink {} → {}: {e}",
                    legacy.display(),
                    tarpc_socket_path.display()
                );
            } else {
                info!(
                    "🔗 Legacy symlink: {} → {}",
                    legacy.display(),
                    tarpc_socket_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                );
            }
        }
    }

    let jsonrpc_handler = Arc::new(JsonRpcHandler::new(
        Arc::clone(&executor),
        version,
        Some(Arc::clone(&error_count)),
        Arc::clone(&ready),
        Some(Arc::new(jsonrpc_socket.clone())),
    ));

    // Extract anchor store before handler moves into the server task
    let anchor_store = jsonrpc_handler.anchor_store();

    // Recover VFIO anchors from systemd fd store (survives daemon restart)
    let recovered = systemd_fdstore::retrieve_anchors();
    if !recovered.is_empty() {
        let count = recovered.len();
        let mut store = anchor_store.lock().await;
        for (bdf, anchor) in recovered {
            info!(
                bdf,
                "recovered VfioAnchor from systemd fd store — GPU warm state preserved"
            );
            store.insert(bdf, anchor);
        }
        info!(
            count,
            "restored VfioAnchor(s) from previous daemon instance"
        );
    }

    let tarpc_path_for_server = tarpc_socket_path.clone();
    let jsonrpc_socket_for_server = jsonrpc_socket.clone();
    let unibin_for_server = unibin_config.clone();

    // Stop the early health responder — full handler is ready to accept.
    let _ = early_stop_tx.send(true);
    tokio::task::yield_now().await;

    let server_handle = tokio::spawn(async move {
        match execution::start_servers_with_fallback(
            server,
            jsonrpc_handler,
            tarpc_path_for_server,
            jsonrpc_socket_for_server,
            tcp_port,
            &unibin_for_server,
            jsonrpc_listener,
        )
        .await
        {
            Ok(()) => info!("✅ Servers stopped gracefully"),
            Err(e) => error!("❌ Server error: {}", e),
        }
    });

    // PCIe keepalive — prevents PLX PEX 8747 from D3cold-gating K80 GPUs.
    tokio::spawn(async { crate::background::pcie_keepalive::run().await });

    // Verify forensic logging works before any handoff is attempted.
    toadstool_cylinder::vfio::sovereign_handoff::forensics::startup_smoke_test();

    // Exp 229: catalyst handoff watchdog — monitors handoff liveness and
    // performs emergency interrupt quench + process kill if the pipeline
    // becomes unresponsive (diesel engine safety net).
    if let Err(e) = crate::background::catalyst_watchdog::start_watchdog_thread() {
        error!(error = %e, "failed to spawn catalyst watchdog thread; handoff safety net disabled");
    }

    // Exp 232: kernel oops sentinel — monitors /dev/kmsg for crash signatures
    // and saves triage reports before the system goes down.
    if let Err(e) = crate::background::kernel_sentinel::start_sentinel_thread() {
        error!(error = %e, "failed to spawn kernel sentinel thread; crash forensics disabled");
    }

    // PG-62: discovery registration and biomeOS scan run AFTER listeners are
    // spawned so that health.liveness is reachable during initialization.
    // Callers see {"status":"starting"} until ready flag is set below.
    info!("Self-registering with discovery service...");
    match toadstool::ipc_helpers::register_with_discovery().await {
        Ok(()) => {
            info!("Successfully self-registered with discovery service");
        }
        Err(e) => {
            warn!("Could not self-register with discovery service: {}", e);
            warn!("Operating in standalone mode (no discovery)");
        }
    }

    let biomeos_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    if biomeos_dir.exists() {
        let mut discovered = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&biomeos_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("sock")
                    && let Some(name) = path.file_stem().and_then(|s| s.to_str())
                {
                    discovered.push(name.to_string());
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

    ready.store(true, Ordering::Release);
    info!("✅ Server fully initialized — health.liveness → alive");

    if let Err(e) = systemd_fdstore::sd_notify("READY=1\n") {
        info!("sd_notify(READY=1) skipped: {e} (normal outside systemd)");
    } else {
        info!("sd_notify(READY=1) sent to systemd");
    }

    // Wave 43: Neural API self-announcement — register capabilities, cost hints,
    // and latency estimates with biomeOS so routing weights are built.
    {
        let announce_socket = jsonrpc_socket.to_string_lossy().to_string();
        match toadstool::ipc_helpers::self_announce_to_biomeos(
            crate::ipc_surface::ANNOUNCED_METHODS,
            &announce_socket,
        )
        .await
        {
            Ok(()) => info!("Neural API: announced to biomeOS (compute, science, inference)"),
            Err(e) => info!("Neural API: announce skipped — {e} (standalone mode)"),
        }
    }

    info!("Ready for shutdown (Ctrl+C or SIGTERM)");
    let shutdown_signal = execution::wait_for_shutdown_signal().await;

    match shutdown_signal {
        ShutdownSignal::Sigint => info!("📡 Received SIGINT (Ctrl+C)"),
        ShutdownSignal::Sigterm => info!("📡 Received SIGTERM (graceful shutdown)"),
        ShutdownSignal::Error(err) => error!("Failed to listen for shutdown signals: {}", err),
    }

    // Store VFIO anchor fds in systemd's FileDescriptorStore.
    // systemd holds duplicated fds in PID 1, keeping the VFIO binding
    // alive across our process exit — prevents Secondary Bus Reset.
    {
        let anchors = anchor_store.lock().await;
        if !anchors.is_empty() {
            let count = anchors.len();
            let stored = systemd_fdstore::store_anchors(&anchors);
            if stored > 0 {
                info!(
                    anchors = count,
                    fds_stored = stored,
                    "stored VFIO fds in systemd fd store — GPUs will stay warm"
                );
            } else {
                warn!("failed to store fds in systemd — falling back to anchor leak");
                drop(anchors);
                let mut anchors = anchor_store.lock().await;
                for (_bdf, anchor) in anchors.drain() {
                    anchor.leak();
                }
            }
        }
    }

    info!("Shutting down ToadStool server...");
    server_handle.abort();

    if tarpc_socket_path.exists()
        && let Err(e) = tokio::fs::remove_file(&tarpc_socket_path).await
    {
        warn!("Failed to remove tarpc socket: {}", e);
    }
    if jsonrpc_socket.exists()
        && let Err(e) = tokio::fs::remove_file(&jsonrpc_socket).await
    {
        warn!("Failed to remove JSON-RPC socket: {}", e);
    }
    // Clean up legacy symlink
    if let Some(ref legacy) = legacy_socket
        && (legacy.exists() || legacy.symlink_metadata().is_ok())
        && let Err(e) = tokio::fs::remove_file(legacy).await
    {
        warn!("Failed to remove legacy symlink: {}", e);
    }

    info!("ToadStool server stopped");
    Ok(())
}

#[cfg(test)]
mod tests;
