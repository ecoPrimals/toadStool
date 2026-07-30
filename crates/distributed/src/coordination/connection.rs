// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coordination connection management

use toadstool::error::{ToadStoolError, ToadStoolResult};

use super::types::{
    ConnectionHealth, CoordinationConnection, CoordinationConnectionConfig, CoordinationTransport,
    ProtocolConfig,
};
use toadstool_common::auth::AuthType;

impl CoordinationConnection {
    /// Connect using config: pick a healthy endpoint, set auth token, and record health.
    pub async fn new(config: CoordinationConnectionConfig) -> ToadStoolResult<Self> {
        // Validate at least one endpoint is provided
        if config.endpoints.is_empty() {
            return Err(ToadStoolError::runtime(
                "No coordination endpoints provided",
            ));
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
        })
    }

    async fn test_endpoint_health(
        endpoint: &str,
        protocol_config: &ProtocolConfig,
    ) -> ToadStoolResult<()> {
        match protocol_config.protocol {
            CoordinationTransport::HTTP => {
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
                     HTTP is deprecated in ecoPrimals; use Unix socket RPC (unix://…)"
                )))
            }
            CoordinationTransport::MessageQueue => {
                if let Some(path) = endpoint
                    .strip_prefix("unix://")
                    .or_else(|| endpoint.strip_prefix("file://"))
                {
                    Self::probe_unix_socket(path).await
                } else {
                    Err(ToadStoolError::not_supported(format!(
                        "Message queue health check not implemented for {endpoint:?}: \
                         use Unix socket (unix://…) for coordination IPC. \
                         Broker-specific probing (AMQP/Kafka) is a future scaling target."
                    )))
                }
            }
        }
    }

    /// Verify a Unix domain socket is reachable by opening a connection.
    ///
    /// Only available on Unix targets. On non-Unix builds the probe returns
    /// an error because Unix socket IPC is unavailable on that platform.
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
            Err(ToadStoolError::not_supported(
                "unix socket health probe not available on this platform",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordination::types::{
        CoordinationConnectionConfig, HttpProtocolConfig, MessageQueueProtocolConfig,
    };
    use std::collections::HashMap;
    use toadstool_common::auth::{AuthCredentials, ServiceAuthConfig};
    use toadstool_common::config_bases::ConnectionPoolConfig;

    // Test endpoint constants (avoid hardcoded ports/IPs)
    const TEST_AMQP_ENDPOINT: &str = "amqp://localhost:5672";
    const TEST_HTTP_ENDPOINT_8080: &str = "http://localhost:8080";
    const TEST_HTTP_ENDPOINT_9000: &str = "http://localhost:9000";
    const TEST_PLACEHOLDER_ENDPOINT_A: &str = "http://a:1";
    const TEST_PLACEHOLDER_ENDPOINT_B: &str = "http://b:2";
    const TEST_MINIMAL_ENDPOINT: &str = "http://localhost:1";

    fn base_protocol_config(protocol: CoordinationTransport) -> ProtocolConfig {
        ProtocolConfig {
            protocol,
            http: HttpProtocolConfig {
                timeout_ms: 5000,
                max_retries: 3,
                headers: HashMap::new(),
            },
            message_queue: MessageQueueProtocolConfig {
                queue_name: "default".to_string(),
                exchange: "default".to_string(),
                routing_key: "default".to_string(),
            },
        }
    }

    #[tokio::test]
    async fn test_connection_empty_endpoints() {
        let config = CoordinationConnectionConfig {
            endpoints: vec![],
            protocol_config: base_protocol_config(CoordinationTransport::HTTP),
            auth_config: ServiceAuthConfig::default(),
            pool: ConnectionPoolConfig::default(),
        };

        let result = CoordinationConnection::new(config).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No coordination endpoints")
        );
    }

    #[tokio::test]
    async fn test_connection_http_tcp_endpoint_degraded() -> ToadStoolResult<()> {
        let config = CoordinationConnectionConfig {
            endpoints: vec![TEST_HTTP_ENDPOINT_8080.to_string()],
            protocol_config: base_protocol_config(CoordinationTransport::HTTP),
            auth_config: ServiceAuthConfig::default(),
            pool: ConnectionPoolConfig::default(),
        };

        let conn = CoordinationConnection::new(config).await?;
        assert_eq!(conn.active_endpoint, TEST_HTTP_ENDPOINT_8080);
        assert_eq!(conn.health_status, ConnectionHealth::Degraded);
        Ok(())
    }

    #[tokio::test]
    async fn test_connection_http_https_endpoint_degraded() -> ToadStoolResult<()> {
        let config = CoordinationConnectionConfig {
            endpoints: vec!["https://coordination.example.com:443".to_string()],
            protocol_config: base_protocol_config(CoordinationTransport::HTTP),
            auth_config: ServiceAuthConfig::default(),
            pool: ConnectionPoolConfig::default(),
        };

        let conn = CoordinationConnection::new(config).await?;
        assert_eq!(conn.health_status, ConnectionHealth::Degraded);
        Ok(())
    }

    #[tokio::test]
    async fn test_connection_message_queue_tcp_endpoint_rejected() -> ToadStoolResult<()> {
        let config = CoordinationConnectionConfig {
            endpoints: vec![TEST_AMQP_ENDPOINT.to_string()],
            protocol_config: base_protocol_config(CoordinationTransport::MessageQueue),
            auth_config: ServiceAuthConfig::default(),
            pool: ConnectionPoolConfig::default(),
        };

        let conn = CoordinationConnection::new(config).await?;
        assert_eq!(conn.health_status, ConnectionHealth::Degraded);
        assert_eq!(conn.active_endpoint, TEST_AMQP_ENDPOINT);
        Ok(())
    }

    #[tokio::test]
    async fn test_connection_auth_api_key() -> ToadStoolResult<()> {
        let creds = AuthCredentials {
            api_key: Some("secret-key".to_string()),
            ..Default::default()
        };
        let config = CoordinationConnectionConfig {
            endpoints: vec![TEST_HTTP_ENDPOINT_8080.to_string()],
            protocol_config: base_protocol_config(CoordinationTransport::HTTP),
            auth_config: ServiceAuthConfig {
                auth_type: AuthType::ApiKey,
                credentials: creds,
            },
            pool: ConnectionPoolConfig::default(),
        };

        let conn = CoordinationConnection::new(config).await?;
        assert_eq!(conn.auth_token, Some("secret-key".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_connection_auth_bearer() -> ToadStoolResult<()> {
        let creds = AuthCredentials {
            token: Some("bearer-token-123".to_string()),
            ..Default::default()
        };
        let config = CoordinationConnectionConfig {
            endpoints: vec![TEST_HTTP_ENDPOINT_8080.to_string()],
            protocol_config: base_protocol_config(CoordinationTransport::HTTP),
            auth_config: ServiceAuthConfig {
                auth_type: AuthType::Bearer,
                credentials: creds,
            },
            pool: ConnectionPoolConfig::default(),
        };

        let conn = CoordinationConnection::new(config).await?;
        assert_eq!(conn.auth_token, Some("bearer-token-123".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_connection_auth_oauth2() -> ToadStoolResult<()> {
        let creds = AuthCredentials {
            token: Some("oauth2-access-token".to_string()),
            ..Default::default()
        };
        let config = CoordinationConnectionConfig {
            endpoints: vec![TEST_HTTP_ENDPOINT_8080.to_string()],
            protocol_config: base_protocol_config(CoordinationTransport::HTTP),
            auth_config: ServiceAuthConfig {
                auth_type: AuthType::OAuth2,
                credentials: creds,
            },
            pool: ConnectionPoolConfig::default(),
        };

        let conn = CoordinationConnection::new(config).await?;
        assert_eq!(conn.auth_token, Some("oauth2-access-token".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_connection_http_all_tcp_endpoints_degrade() -> ToadStoolResult<()> {
        let config = CoordinationConnectionConfig {
            endpoints: vec![
                "invalid-endpoint".to_string(),
                TEST_HTTP_ENDPOINT_9000.to_string(),
            ],
            protocol_config: base_protocol_config(CoordinationTransport::HTTP),
            auth_config: ServiceAuthConfig::default(),
            pool: ConnectionPoolConfig::default(),
        };

        let conn = CoordinationConnection::new(config).await?;
        assert_eq!(conn.health_status, ConnectionHealth::Degraded);
        Ok(())
    }

    #[tokio::test]
    async fn test_connection_all_endpoints_fail_uses_first_degraded() -> ToadStoolResult<()> {
        let config = CoordinationConnectionConfig {
            endpoints: vec!["invalid".to_string(), "also-invalid".to_string()],
            protocol_config: base_protocol_config(CoordinationTransport::HTTP),
            auth_config: ServiceAuthConfig::default(),
            pool: ConnectionPoolConfig::default(),
        };

        let conn = CoordinationConnection::new(config).await?;
        assert_eq!(conn.active_endpoint, "invalid");
        assert_eq!(conn.health_status, ConnectionHealth::Degraded);
        Ok(())
    }

    #[tokio::test]
    async fn test_connection_http_plain_rejected() -> ToadStoolResult<()> {
        let config = CoordinationConnectionConfig {
            endpoints: vec![TEST_HTTP_ENDPOINT_8080.to_string()],
            protocol_config: base_protocol_config(CoordinationTransport::HTTP),
            auth_config: ServiceAuthConfig::default(),
            pool: ConnectionPoolConfig::default(),
        };

        let conn = CoordinationConnection::new(config).await?;
        assert_eq!(conn.health_status, ConnectionHealth::Degraded);
        assert_eq!(conn.active_endpoint, TEST_HTTP_ENDPOINT_8080);
        Ok(())
    }

    #[tokio::test]
    async fn test_connection_endpoints_preserved() -> ToadStoolResult<()> {
        let config = CoordinationConnectionConfig {
            endpoints: vec![
                TEST_PLACEHOLDER_ENDPOINT_A.to_string(),
                TEST_PLACEHOLDER_ENDPOINT_B.to_string(),
            ],
            protocol_config: base_protocol_config(CoordinationTransport::HTTP),
            auth_config: ServiceAuthConfig::default(),
            pool: ConnectionPoolConfig::default(),
        };

        let conn = CoordinationConnection::new(config).await?;
        assert_eq!(conn.endpoints.len(), 2);
        assert_eq!(conn.endpoints[0], TEST_PLACEHOLDER_ENDPOINT_A);
        assert_eq!(conn.endpoints[1], TEST_PLACEHOLDER_ENDPOINT_B);
        Ok(())
    }

    #[tokio::test]
    async fn test_connection_protocol_config_preserved() -> ToadStoolResult<()> {
        let config = CoordinationConnectionConfig {
            endpoints: vec![TEST_MINIMAL_ENDPOINT.to_string()],
            protocol_config: base_protocol_config(CoordinationTransport::HTTP),
            auth_config: ServiceAuthConfig::default(),
            pool: ConnectionPoolConfig::default(),
        };

        let conn = CoordinationConnection::new(config).await?;
        assert!(matches!(
            conn.protocol_config.protocol,
            CoordinationTransport::HTTP
        ));
        Ok(())
    }
}
