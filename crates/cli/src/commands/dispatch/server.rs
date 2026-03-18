// SPDX-License-Identifier: AGPL-3.0-or-later
//! Server and daemon command handlers
//!
//! Server, Daemon, ByobServer - long-running service modes.

use tracing::info;

use crate::{CliError, Result};

/// Run ToadStool in server/daemon mode (UniBin compliant)
pub async fn run_server_daemon(family_id: Option<String>) -> Result<()> {
    info!("🚀 Starting ToadStool server (UniBin mode)...");

    toadstool_server::run_server_main(family_id)
        .await
        .map_err(|e| CliError::Other(format!("Server failed: {e}")))?;

    Ok(())
}

/// Run BYOB server
pub async fn run_byob_server(
    bind: Option<String>,
    port: u16,
    config: Option<std::path::PathBuf>,
) -> Result<()> {
    info!("🍄 Starting Toadstool BYOB Server");
    let bind = bind.unwrap_or_else(toadstool_config::config_utils::ConfigUtils::get_bind_address);
    let config = toadstool_runtime_container::byob_server::ByobServerConfig {
        bind_address: Some(bind),
        port: Some(port),
        config_path: config.as_ref().and_then(|p| p.to_str().map(String::from)),
    };
    toadstool_runtime_container::byob_server::run_byob_server(config)
        .await
        .map_err(|e| CliError::Other(format!("BYOB server failed: {e}")))
}
