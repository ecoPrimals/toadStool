//! Songbird connection management

use toadstool::error::{ToadStoolError, ToadStoolResult};

use toadstool_common::auth::AuthType;
use super::types::{
    ConnectionHealth, ProtocolConfig, SongbirdConnection, SongbirdConnectionConfig,
    SongbirdProtocol,
};

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
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_millis(
                        protocol_config.http.timeout_ms,
                    ))
                    .build()
                    .map_err(|e| {
                        ToadStoolError::runtime(format!("Failed to create HTTP client: {e}"))
                    })?;

                let health_url = format!("{endpoint}/health");
                client
                    .get(&health_url)
                    .send()
                    .await
                    .map_err(|e| ToadStoolError::runtime(format!("Health check failed: {e}")))?;
                Ok(())
            }
            SongbirdProtocol::GRPC => {
                // For gRPC, we'll assume the endpoint is healthy if it's a valid URL
                if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
                    Ok(())
                } else {
                    Err(ToadStoolError::runtime("Invalid gRPC endpoint"))
                }
            }
            SongbirdProtocol::WebSocket => {
                // For WebSocket, test if the endpoint looks valid
                if endpoint.starts_with("ws://") || endpoint.starts_with("wss://") {
                    Ok(())
                } else {
                    Err(ToadStoolError::runtime("Invalid WebSocket endpoint"))
                }
            }
            SongbirdProtocol::MessageQueue => {
                // For message queues, we'll assume it's healthy if configured
                Ok(())
            }
        }
    }
}
