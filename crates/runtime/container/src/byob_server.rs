// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Toadstool BYOB Server
//!
//! HTTP server for handling BYOB deployment requests from Songbird.
//! Provides compute execution capabilities for team biome deployments.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tracing::info;

use toadstool::{
    RuntimeEngine, ToadStoolError, ToadStoolResult,
    byob::{ByobExecutor, ByobExecutorConfig, create_byob_executor},
};

use crate::byob_routes::ByobApi;
use toadstool_common::constants::network::BIND_ALL_IPV4;
use toadstool_config::ports::daemon_port;

use crate::ContainerRuntimeEngine;

/// Configuration for the BYOB server
#[derive(Debug, Clone, Default)]
pub struct ByobServerConfig {
    /// Server bind address (overrides config file if set)
    pub bind_address: Option<String>,

    /// Server port (overrides config file if set)
    pub port: Option<u16>,

    /// Path to TOML config file (optional; if set, loads `bind_address`, port, `byob_config` from file)
    pub config_path: Option<String>,
}

/// Config file format (for TOML deserialization)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ByobServerConfigFile {
    #[serde(default = "default_bind_address")]
    bind_address: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default)]
    byob_config: ByobExecutorConfig,
}

fn default_bind_address() -> String {
    BIND_ALL_IPV4.to_string()
}

fn default_port() -> u16 {
    daemon_port()
}

/// Run the BYOB server. Binds to the configured address and serves until shutdown.
///
/// # Errors
///
/// Returns an error if configuration loading, runtime engine creation, or server startup fails.
pub async fn run_byob_server(config: ByobServerConfig) -> ToadStoolResult<()> {
    let loaded = load_config_inner(config.config_path.as_deref()).await?;
    let bind_address = config
        .bind_address
        .as_deref()
        .unwrap_or(&loaded.bind_address);
    let port = config.port.unwrap_or(loaded.port);

    let runtime_engine = create_runtime_engine().await?;
    let byob_executor = create_byob_executor(runtime_engine);

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .merge(ByobApi::routes())
        .with_state(byob_executor);

    let addr: SocketAddr = format!("{bind_address}:{port}")
        .parse()
        .map_err(|e| ToadStoolError::configuration(format!("Invalid bind address: {e}")))?;
    info!("Starting Toadstool BYOB Server on {}", addr);

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| ToadStoolError::runtime(format!("Failed to bind BYOB server: {e}")))?;
    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|e| ToadStoolError::runtime(format!("BYOB server error: {e}")))?;

    Ok(())
}

/// Loaded config from file or defaults (`bind_address`, port for server binding)
struct LoadedConfig {
    bind_address: String,
    port: u16,
}

async fn load_config_inner(config_path: Option<&str>) -> ToadStoolResult<LoadedConfig> {
    if let Some(path) = config_path {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToadStoolError::configuration(format!("Failed to read config: {e}")))?;

        let config: ByobServerConfigFile = toml::from_str(&content)
            .map_err(|e| ToadStoolError::configuration(format!("Failed to parse config: {e}")))?;

        Ok(LoadedConfig {
            bind_address: config.bind_address,
            port: config.port,
        })
    } else {
        Ok(LoadedConfig {
            bind_address: default_bind_address(),
            port: default_port(),
        })
    }
}

#[expect(
    clippy::unused_async,
    reason = "kept for API consistency with async runtime creation"
)]
async fn create_runtime_engine() -> ToadStoolResult<Arc<dyn RuntimeEngine>> {
    info!("Initializing container runtime engine");

    let engine = ContainerRuntimeEngine::new()?;

    Ok(Arc::new(engine))
}

async fn root_handler(State(_executor): State<Arc<dyn ByobExecutor>>) -> &'static str {
    "🍄 Toadstool BYOB Server - Ready for team biome deployments!"
}

async fn health_handler(
    State(_executor): State<Arc<dyn ByobExecutor>>,
) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "toadstool-byob-server",
        "version": env!("CARGO_PKG_VERSION"),
        "message": "Ready to execute team biomes"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byob_server_config_default() {
        let config = ByobServerConfig::default();
        assert!(config.bind_address.is_none());
        assert!(config.port.is_none());
        assert!(config.config_path.is_none());
    }

    #[test]
    fn test_byob_server_config_clone() {
        let config = ByobServerConfig {
            bind_address: Some("127.0.0.1".to_string()),
            port: Some(9999),
            config_path: Some("/tmp/test.toml".to_string()),
        };
        let cloned = config.clone();
        assert_eq!(cloned.bind_address, config.bind_address);
        assert_eq!(cloned.port, config.port);
        assert_eq!(cloned.config_path, config.config_path);
    }

    #[test]
    fn test_byob_server_config_debug() {
        let config = ByobServerConfig::default();
        let _ = format!("{config:?}");
    }

    #[tokio::test]
    async fn test_run_byob_server_invalid_config_path() {
        let config = ByobServerConfig {
            bind_address: None,
            port: None,
            config_path: Some("/nonexistent/path/that/does/not/exist/config.toml".to_string()),
        };
        let result = run_byob_server(config).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("config") || err.to_string().contains("read"));
    }
}
