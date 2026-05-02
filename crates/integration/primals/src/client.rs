// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::error::{PrimalError, PrimalResult};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Client for interacting with primals via Unix socket JSON-RPC
#[derive(Debug, Clone)]
pub struct PrimalClient {
    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
}

/// Request to a primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalRequest {
    pub action: String,
    pub payload: serde_json::Value,
}

/// Response from a primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalResponse {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

impl PrimalClient {
    /// Create a new primal client for the given Unix socket path
    ///
    /// The endpoint should be a path to a Unix domain socket (e.g. `/run/toadstool/socket`).
    pub fn new(endpoint: impl Into<std::path::PathBuf>) -> Self {
        Self {
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(endpoint),
        }
    }

    /// Send a JSON-RPC request to the primal
    ///
    /// Maps `PrimalRequest.action` to the JSON-RPC method and `payload` to params.
    /// Returns proper errors on connection failure, socket not found, or JSON-RPC errors.
    pub async fn send_request(&self, request: PrimalRequest) -> PrimalResult<PrimalResponse> {
        debug!("Sending JSON-RPC request: method={}", request.action);

        let result = self
            .rpc_client
            .call(&request.action, request.payload)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("connect") || msg.contains("Connection refused") {
                    PrimalError::Network {
                        endpoint: self.rpc_client.socket_path().to_string_lossy().to_string(),
                        reason: msg,
                    }
                } else if msg.contains("No such file") || msg.contains("not found") {
                    PrimalError::ServiceUnavailable {
                        service: format!("primal at {:?}", self.rpc_client.socket_path()),
                    }
                } else {
                    PrimalError::Integration {
                        primal: "unknown".to_string(),
                        message: msg,
                    }
                }
            })?;

        // Parse result into PrimalResponse if it's an object with success/data/error
        let response = if let Some(obj) = result.as_object() {
            PrimalResponse {
                success: obj.get("success").and_then(|v| v.as_bool()).unwrap_or(true),
                data: obj.get("data").cloned().unwrap_or(serde_json::Value::Null),
                error: obj
                    .get("error")
                    .and_then(|v| v.as_str())
                    .map(std::string::ToString::to_string),
            }
        } else {
            PrimalResponse {
                success: true,
                data: result,
                error: None,
            }
        };

        Ok(response)
    }

    /// Perform a health check by calling `health.check` JSON-RPC method
    ///
    /// Returns `Ok(true)` if the primal is healthy, `Ok(false)` if unhealthy,
    /// or an error if the socket is unreachable or the call fails.
    pub async fn health_check(&self) -> PrimalResult<bool> {
        let request = PrimalRequest {
            action: "health.check".to_string(),
            payload: serde_json::json!({}),
        };

        let response = self.send_request(request).await?;

        if let Some(err) = &response.error {
            return Err(PrimalError::Integration {
                primal: "health".to_string(),
                message: err.clone(),
            });
        }

        Ok(response.success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primal_client_new_stores_path() {
        let client = PrimalClient::new("/tmp/test.sock");
        assert_eq!(
            client.rpc_client.socket_path(),
            std::path::Path::new("/tmp/test.sock")
        );
    }

    #[test]
    fn primal_request_roundtrips_serde() {
        let req = PrimalRequest {
            action: "compute.dispatch".to_string(),
            payload: serde_json::json!({"input": [1, 2, 3]}),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: PrimalRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.action, "compute.dispatch");
        assert_eq!(back.payload["input"], serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn primal_response_roundtrips_serde() {
        let resp = PrimalResponse {
            success: false,
            data: serde_json::json!({"status": "error"}),
            error: Some("timeout".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: PrimalResponse = serde_json::from_str(&json).unwrap();
        assert!(!back.success);
        assert_eq!(back.error.as_deref(), Some("timeout"));
    }

    #[tokio::test]
    async fn send_request_to_nonexistent_socket_returns_error() {
        let client = PrimalClient::new("/tmp/__no_such_socket_primal_test__.sock");
        let req = PrimalRequest {
            action: "test.ping".to_string(),
            payload: serde_json::json!({}),
        };
        let err = client.send_request(req).await.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("No such file") || msg.contains("connect") || msg.contains("unavailable"),
            "unexpected error: {msg}"
        );
    }

    #[tokio::test]
    async fn health_check_to_nonexistent_socket_returns_error() {
        let client = PrimalClient::new("/tmp/__no_such_socket_health_test__.sock");
        assert!(client.health_check().await.is_err());
    }
}
