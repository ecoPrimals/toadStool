// SPDX-License-Identifier: AGPL-3.0-or-later
//! Server and daemon command handlers
//!
//! Server, Daemon, ByobServer - long-running service modes.

use tracing::info;

use crate::{CliError, Result};

/// Run ToadStool in server/daemon mode (UniBin compliant)
///
/// `bind_override` is an explicit `host:port` from `--bind` that takes precedence
/// over `--port` and `TOADSTOOL_BIND_ADDRESS`.
pub async fn run_server_daemon(
    family_id: Option<String>,
    bind_override: Option<String>,
    port: Option<u16>,
    socket_override: Option<std::path::PathBuf>,
    biomeos_socket_override: Option<std::path::PathBuf>,
) -> Result<()> {
    info!("🚀 Starting ToadStool server (UniBin mode)...");

    toadstool_server::run_server_main(
        family_id,
        bind_override,
        port,
        socket_override,
        biomeos_socket_override,
    )
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use crate::CliError;

    #[tokio::test(flavor = "current_thread")]
    async fn run_byob_server_invalid_bind_maps_to_cli_error() {
        let err = super::run_byob_server(
            Some("not-a-valid-socket-addr-for-parse".to_string()),
            8080,
            None,
        )
        .await
        .expect_err("invalid bind should fail before listen");

        assert!(
            matches!(err, CliError::Other(ref msg) if msg.contains("BYOB server failed")),
            "expected BYOB mapping, got {err:?}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn run_byob_server_missing_config_file_maps_to_cli_error() {
        let err = super::run_byob_server(
            None,
            8080,
            Some(PathBuf::from("/nonexistent/byob-config.toml")),
        )
        .await
        .expect_err("missing config should fail");

        assert!(
            matches!(err, CliError::Other(ref msg) if msg.contains("BYOB server failed")),
            "expected BYOB mapping, got {err:?}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn run_server_daemon_blocks_until_shutdown() {
        temp_env::async_with_vars([("TOADSTOOL_STANDALONE", Some("1"))], async {
            let result = tokio::time::timeout(
                Duration::from_millis(80),
                super::run_server_daemon(
                    Some("test-family-id".to_string()),
                    None,
                    None,
                    None,
                    None,
                ),
            )
            .await;

            assert!(
                result.is_err(),
                "server daemon should block on shutdown signal (timeout expected)"
            );
        })
        .await;
    }
}
