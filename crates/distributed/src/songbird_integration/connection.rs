//! Songbird connection management

use toadstool::error::{ToadStoolError, ToadStoolResult};

use super::types::{
    ConnectionHealth, ProtocolConfig, SongbirdConnection, SongbirdConnectionConfig,
    SongbirdProtocol,
};
use toadstool_common::auth::AuthType;

impl SongbirdConnection {
    pub async fn new(config: SongbirdConnectionConfig) -> ToadStoolResult<Self> {
        // Validate at least one endpoint is provided
        if config.endpoints.is_empty() {
            return Err(ToadStoolError::runtime("No Songbird endpoints provided"));
        }

        // Test connectivity to find the best active endpoint
        let mut active_endpoint = config.endpoints[0].clone();
        let mut health_status = ConnectionHealth::Unknown;

        for endpoint in &config.endpoints {
            match Self::test_endpoint_health(endpoint, &config.protocol_config).await {
                Ok(()) => {
                    active_endpoint = endpoint.clone();
                    health_status = ConnectionHealth::Healthy;
                    break;
                }
                Err(_) => continue,
            }
        }

        // If no endpoints are healthy, use the first one but mark as degraded
        if health_status == ConnectionHealth::Unknown {
            health_status = ConnectionHealth::Degraded;
        }

        let auth_token = match config.auth_config.auth_type {
            AuthType::ApiKey => config.auth_config.credentials.api_key.clone(),
            AuthType::Bearer => config.auth_config.credentials.token.clone(),
            AuthType::OAuth2 => config.auth_config.credentials.token.clone(), // OAuth2 uses token field
            _ => None,
        };

        Ok(Self {
            endpoints: config.endpoints,
            active_endpoint,
            auth_token,
            health_status,
            protocol_config: config.protocol_config,
            #[cfg(feature = "channels")]
            reply_channel: None,
        })
    }

    async fn test_endpoint_health(
        endpoint: &str,
        protocol_config: &ProtocolConfig,
    ) -> ToadStoolResult<()> {
        match protocol_config.protocol {
            SongbirdProtocol::HTTP => {
                // HTTP is deprecated in ecoPrimals — Unix socket RPC is the correct path.
                // If the caller passes a unix:// endpoint with HTTP protocol, honour it.
                // Otherwise reject: HTTP carries C-FFI (ring/openssl) risk and is not in uniBin.
                if let Some(sock_path) = endpoint
                    .strip_prefix("unix://")
                    .or_else(|| endpoint.strip_prefix("file://"))
                {
                    return Self::probe_unix_socket(sock_path).await;
                }
                // Plain HTTP URL — refuse with a clear error so callers migrate.
                Err(ToadStoolError::runtime(format!(
                    "HTTP health check rejected for {endpoint:?}: \
                     HTTP is deprecated in ecoPrimals; use Unix socket RPC (unix://…) or gRPC"
                )))
            }
            SongbirdProtocol::GRPC => {
                if endpoint.starts_with("unix://") || endpoint.starts_with("file://") {
                    let path = endpoint
                        .trim_start_matches("unix://")
                        .trim_start_matches("file://");
                    return Self::probe_unix_socket(path).await;
                }
                if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
                    Ok(())
                } else {
                    Err(ToadStoolError::runtime(format!(
                        "Invalid gRPC endpoint: {endpoint:?}"
                    )))
                }
            }
            SongbirdProtocol::MessageQueue => {
                // Message queue brokers are assumed healthy if reachable; a full
                // broker ping would require protocol-specific frames. Accept as-is
                // unless the endpoint is explicitly a Unix socket we can probe.
                if let Some(path) = endpoint
                    .strip_prefix("unix://")
                    .or_else(|| endpoint.strip_prefix("file://"))
                {
                    Self::probe_unix_socket(path).await
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Verify a Unix domain socket is reachable by opening a connection.
    ///
    /// Only available on Unix targets. On non-Unix builds the probe always
    /// succeeds (Windows/WASM environments don't use Unix sockets).
    async fn probe_unix_socket(path: &str) -> ToadStoolResult<()> {
        #[cfg(unix)]
        {
            use tokio::net::UnixStream;
            UnixStream::connect(path).await.map(|_| ()).map_err(|e| {
                ToadStoolError::runtime(format!(
                    "Unix socket health check failed for {path:?}: {e}"
                ))
            })
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Ok(())
        }
    }
}
