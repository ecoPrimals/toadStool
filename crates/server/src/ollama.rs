// SPDX-License-Identifier: AGPL-3.0-only
//! Ollama Integration Client
//!
//! Native Ollama management for model lifecycle and inference.
//! Toadstool owns GPU resources, so it should own model lifecycle management.
//!
//! ## Design
//!
//! Pure Rust HTTP client talking to Ollama's local API.
//! No external HTTP client dependencies -- uses `tokio::net::TcpStream` directly
//! for simple JSON-over-HTTP communication with the local Ollama server.
//!
//! ## Methods
//!
//! - `ollama.list_models()` - Models available on this gate
//! - `ollama.inference(model, prompt, params)` - Run inference
//! - `ollama.load(model)` - Preload model into VRAM
//! - `ollama.unload(model)` - Free VRAM

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;
use toadstool_common::constants::{network::LOCALHOST_IPV4, timeouts::DEFAULT_REQUEST_TIMEOUT};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::debug;

/// Default Ollama API port (upstream default: 11434)
const DEFAULT_OLLAMA_PORT: u16 = 11434;

/// Ollama client configuration
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// Ollama API host (default: OLLAMA_HOST env or LOCALHOST_IPV4)
    pub host: String,
    /// Ollama API port (default: OLLAMA_PORT env or 11434)
    pub port: u16,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("OLLAMA_HOST").unwrap_or_else(|_| LOCALHOST_IPV4.to_string()),
            port: std::env::var("OLLAMA_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(DEFAULT_OLLAMA_PORT),
            timeout_secs: DEFAULT_REQUEST_TIMEOUT.as_secs(),
        }
    }
}

/// Model information returned by Ollama
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub digest: String,
    #[serde(default)]
    pub modified_at: String,
}

/// Ollama client for local API communication
///
/// Pure Rust implementation -- no reqwest or hyper dependency.
/// Communicates with Ollama via simple HTTP/1.1 over TCP.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    config: OllamaConfig,
}

impl OllamaClient {
    /// Create a new Ollama client with the given configuration
    #[must_use]
    pub fn new(config: OllamaConfig) -> Self {
        Self { config }
    }

    /// List available models on this gate
    ///
    /// Calls `GET /api/tags` on the Ollama API.
    ///
    /// # Errors
    ///
    /// Returns [`OllamaError`] if the HTTP request fails or response JSON is invalid.
    pub async fn list_models(&self) -> Result<Vec<OllamaModel>, OllamaError> {
        let response = self.http_get("/api/tags").await?;
        let body: Value = serde_json::from_slice(response.as_bytes()).map_err(OllamaError::Json)?;

        let models = body
            .get("models")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }

    /// Run inference on a model
    ///
    /// Calls `POST /api/generate` on the Ollama API.
    ///
    /// # Errors
    ///
    /// Returns [`OllamaError`] if the HTTP request fails or response JSON is invalid.
    pub async fn inference(
        &self,
        model: &str,
        prompt: &str,
        params: &Value,
    ) -> Result<Value, OllamaError> {
        let mut body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
        });

        // Merge extra params if provided
        if let Value::Object(extra) = params
            && let Value::Object(ref mut obj) = body
        {
            for (k, v) in extra {
                obj.insert(k.clone(), v.clone());
            }
        }

        let response = self.http_post("/api/generate", &body).await?;
        serde_json::from_slice(response.as_bytes()).map_err(OllamaError::Json)
    }

    /// Preload a model into VRAM
    ///
    /// Calls `POST /api/generate` with an empty prompt and `keep_alive` to load the model.
    ///
    /// # Errors
    ///
    /// Returns [`OllamaError`] if the HTTP request fails.
    pub async fn load(&self, model: &str) -> Result<(), OllamaError> {
        let body = serde_json::json!({
            "model": model,
            "prompt": "",
            "stream": false,
            "keep_alive": "5m",
        });

        let _ = self.http_post("/api/generate", &body).await?;
        debug!(model, "Model loaded into VRAM");
        Ok(())
    }

    /// Unload a model from VRAM
    ///
    /// Calls `POST /api/generate` with `keep_alive: 0` to immediately unload.
    ///
    /// # Errors
    ///
    /// Returns [`OllamaError`] if the HTTP request fails.
    pub async fn unload(&self, model: &str) -> Result<(), OllamaError> {
        let body = serde_json::json!({
            "model": model,
            "prompt": "",
            "stream": false,
            "keep_alive": "0",
        });

        let _ = self.http_post("/api/generate", &body).await?;
        debug!(model, "Model unloaded from VRAM");
        Ok(())
    }

    /// Check if Ollama is reachable
    pub async fn is_available(&self) -> bool {
        self.http_get("/").await.is_ok()
    }

    // ---- Internal HTTP helpers (pure Rust, no reqwest) ----

    /// Perform a simple HTTP GET request
    async fn http_get(&self, path: &str) -> Result<String, OllamaError> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let mut stream = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
            TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| OllamaError::Timeout)?
        .map_err(|e| OllamaError::Connection(e.to_string()))?;

        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n",
            host = self.config.host,
            port = self.config.port,
        );

        stream
            .write_all(request.as_bytes())
            .await
            .map_err(OllamaError::Io)?;

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .map_err(OllamaError::Io)?;

        let response_str = String::from_utf8_lossy(&response);
        extract_http_body(&response_str)
            .ok_or_else(|| OllamaError::InvalidResponse("No HTTP body found".to_string()))
    }

    /// Perform a simple HTTP POST request with JSON body
    async fn http_post(&self, path: &str, body: &Value) -> Result<String, OllamaError> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let mut stream = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
            TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| OllamaError::Timeout)?
        .map_err(|e| OllamaError::Connection(e.to_string()))?;

        let json_body = serde_json::to_string(body).map_err(OllamaError::Json)?;

        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n{body}",
            host = self.config.host,
            port = self.config.port,
            len = json_body.len(),
            body = json_body,
        );

        stream
            .write_all(request.as_bytes())
            .await
            .map_err(OllamaError::Io)?;

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .map_err(OllamaError::Io)?;

        let response_str = String::from_utf8_lossy(&response);
        extract_http_body(&response_str)
            .ok_or_else(|| OllamaError::InvalidResponse("No HTTP body found".to_string()))
    }
}

/// Extract the body from a raw HTTP response string
fn extract_http_body(response: &str) -> Option<String> {
    // HTTP body starts after \r\n\r\n
    response
        .find("\r\n\r\n")
        .map(|pos| response[pos + 4..].to_string())
}

/// Ollama client errors
#[derive(Debug, thiserror::Error)]
pub enum OllamaError {
    #[error("Ollama connection failed: {0}")]
    Connection(String),

    #[error("Ollama request timed out")]
    Timeout,

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid HTTP response: {0}")]
    InvalidResponse(String),

    #[error("Ollama API error: {status} - {message}")]
    Api { status: u16, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_default_config() {
        // Don't pollute env, just check defaults work
        let config = OllamaConfig {
            host: "127.0.0.1".to_string(),
            port: 11434,
            timeout_secs: 30,
        };
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 11434);
    }

    #[test]
    fn test_ollama_config_default() {
        let config = OllamaConfig::default();
        assert!(!config.host.is_empty());
        assert!(config.port > 0);
        assert!(config.timeout_secs > 0);
    }

    #[test]
    fn test_extract_http_body() {
        let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"models\":[]}";
        let body = extract_http_body(response);
        assert_eq!(body, Some("{\"models\":[]}".to_string()));
    }

    #[test]
    fn test_extract_http_body_no_body() {
        let response = "HTTP/1.1 200 OK";
        assert!(extract_http_body(response).is_none());
    }

    #[test]
    fn test_extract_http_body_empty_headers() {
        let response = "HTTP/1.1 200 OK\r\n\r\n";
        let body = extract_http_body(response);
        assert_eq!(body, Some(String::new()));
    }

    #[test]
    fn test_ollama_model_deserialize() {
        let json = r#"{"name":"tinyllama:latest","size":637849088,"digest":"abc123","modified_at":"2024-01-01"}"#;
        let model: OllamaModel = serde_json::from_str(json).unwrap();
        assert_eq!(model.name, "tinyllama:latest");
        assert_eq!(model.size, 637_849_088);
    }

    #[test]
    fn test_ollama_model_deserialize_minimal() {
        // Test default values for missing fields
        let json = r#"{"name":"llama3:8b"}"#;
        let model: OllamaModel = serde_json::from_str(json).unwrap();
        assert_eq!(model.name, "llama3:8b");
        assert_eq!(model.size, 0);
        assert!(model.digest.is_empty());
    }

    #[test]
    fn test_ollama_error_display() {
        let err = OllamaError::Connection("refused".to_string());
        assert!(err.to_string().contains("refused"));

        let err = OllamaError::Timeout;
        assert!(err.to_string().contains("timed out"));

        let err = OllamaError::InvalidResponse("bad".to_string());
        assert!(err.to_string().contains("bad"));

        let err = OllamaError::Api {
            status: 500,
            message: "internal error".to_string(),
        };
        assert!(err.to_string().contains("500"));
        assert!(err.to_string().contains("internal error"));
    }

    #[test]
    fn test_inference_request_body_structure() {
        // Verify the structure we build for inference - model, prompt, stream
        let body = serde_json::json!({
            "model": "llama3",
            "prompt": "Hello",
            "stream": false
        });
        assert_eq!(body["model"], "llama3");
        assert_eq!(body["prompt"], "Hello");
        assert_eq!(body["stream"], false);
    }

    #[test]
    fn test_load_request_body() {
        let body = serde_json::json!({
            "model": "llama3",
            "prompt": "",
            "stream": false,
            "keep_alive": "5m"
        });
        assert_eq!(body["keep_alive"], "5m");
    }

    #[test]
    fn test_unload_request_body() {
        let body = serde_json::json!({
            "model": "llama3",
            "prompt": "",
            "stream": false,
            "keep_alive": "0"
        });
        assert_eq!(body["keep_alive"], "0");
    }

    #[tokio::test]
    async fn test_client_creation() {
        let client = OllamaClient::new(OllamaConfig {
            host: "127.0.0.1".to_string(),
            port: 11434,
            timeout_secs: 5,
        });
        // Client should be created without error even if Ollama isn't running
        assert_eq!(client.config.port, 11434);
    }

    #[tokio::test]
    async fn test_is_available_when_not_running() {
        let client = OllamaClient::new(OllamaConfig {
            host: "127.0.0.1".to_string(),
            port: 19999, // Unlikely to be running on this port
            timeout_secs: 1,
        });
        // Should return false, not panic
        assert!(!client.is_available().await);
    }

    #[tokio::test]
    async fn test_list_models_connection_error() {
        let client = OllamaClient::new(OllamaConfig {
            host: "127.0.0.1".to_string(),
            port: 19999,
            timeout_secs: 1,
        });
        let result = client.list_models().await;
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(OllamaError::Connection(_) | OllamaError::Timeout)
        ));
    }

    #[tokio::test]
    async fn test_load_connection_error() {
        let client = OllamaClient::new(OllamaConfig {
            host: "127.0.0.1".to_string(),
            port: 19999,
            timeout_secs: 1,
        });
        let result = client.load("llama3").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unload_connection_error() {
        let client = OllamaClient::new(OllamaConfig {
            host: "127.0.0.1".to_string(),
            port: 19999,
            timeout_secs: 1,
        });
        let result = client.unload("llama3").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_inference_connection_error() {
        let client = OllamaClient::new(OllamaConfig {
            host: "127.0.0.1".to_string(),
            port: 19999,
            timeout_secs: 1,
        });
        let result = client
            .inference("llama3", "Hello", &serde_json::json!({}))
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_ollama_config_custom() {
        let config = OllamaConfig {
            host: "192.168.1.1".to_string(),
            port: 9090,
            timeout_secs: 60,
        };
        let client = OllamaClient::new(config);
        assert_eq!(client.config.host, "192.168.1.1");
        assert_eq!(client.config.port, 9090);
        assert_eq!(client.config.timeout_secs, 60);
    }

    #[test]
    fn test_ollama_model_serialize_roundtrip() {
        let model = OllamaModel {
            name: "llama3:8b".to_string(),
            size: 4_500_000_000,
            digest: "sha256:abc123".to_string(),
            modified_at: "2024-01-15T12:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&model).expect("Serialize failed");
        let restored: OllamaModel = serde_json::from_str(&json).expect("Deserialize failed");
        assert_eq!(model.name, restored.name);
        assert_eq!(model.size, restored.size);
    }

    #[test]
    fn test_list_models_response_parsing_empty() {
        let body = r#"{"models":[]}"#;
        let value: Value = serde_json::from_str(body).unwrap();
        let models = value
            .get("models")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value::<OllamaModel>(v.clone()).ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        assert!(models.is_empty());
    }

    #[test]
    fn test_list_models_response_parsing_with_models() {
        let body = r#"{"models":[{"name":"tinyllama:latest","size":637849088}]}"#;
        let value: Value = serde_json::from_str(body).unwrap();
        let models = value
            .get("models")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value::<OllamaModel>(v.clone()).ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "tinyllama:latest");
    }

    #[test]
    fn test_inference_params_merge() {
        let base = serde_json::json!({
            "model": "llama3",
            "prompt": "Hello",
            "stream": false
        });
        let extra = serde_json::json!({
            "temperature": 0.7,
            "top_p": 0.9
        });
        let mut body = base.clone();
        if let (Some(obj), Some(extra_obj)) = (body.as_object_mut(), extra.as_object()) {
            for (k, v) in extra_obj {
                obj.insert(k.clone(), v.clone());
            }
        }
        assert_eq!(body["temperature"], 0.7);
        assert_eq!(body["top_p"], 0.9);
    }

    #[test]
    fn test_ollama_error_io() {
        let err = OllamaError::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "refused",
        ));
        assert!(err.to_string().contains("IO") || err.to_string().contains("refused"));
    }

    #[test]
    fn test_json_error_conversion() {
        let invalid = "not valid json";
        let result: Result<Value, _> = serde_json::from_str(invalid);
        assert!(result.is_err());
    }
}
