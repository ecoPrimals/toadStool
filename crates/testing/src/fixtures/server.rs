// SPDX-License-Identifier: AGPL-3.0-only
//! Server test fixtures for integration testing
//!
//! Test fixtures may use `expect()` for setup operations, as test setup
//! failures should fail fast.

#![allow(clippy::expect_used)] // Test fixtures may expect on setup

use std::net::SocketAddr;

/// Test server configuration builder
pub struct TestServerConfigBuilder {
    host: String,
    port: u16,
    enable_metrics: bool,
    log_level: String,
}

impl TestServerConfigBuilder {
    pub fn new() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 0, // OS will assign a free port
            enable_metrics: true,
            log_level: "debug".to_string(),
        }
    }

    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub const fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub const fn with_metrics(mut self, enabled: bool) -> Self {
        self.enable_metrics = enabled;
        self
    }

    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = level.into();
        self
    }

    pub fn build(self) -> serde_json::Value {
        serde_json::json!({
            "server": {
                "host": self.host,
                "port": self.port,
                "enable_metrics": self.enable_metrics,
            },
            "logging": {
                "level": self.log_level,
            }
        })
    }

    pub fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Invalid socket address")
    }
}

impl Default for TestServerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Test API request builder
pub struct TestApiRequestBuilder {
    method: String,
    path: String,
    body: Option<serde_json::Value>,
    headers: Vec<(String, String)>,
}

impl TestApiRequestBuilder {
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            body: None,
            headers: vec![],
        }
    }

    pub fn get(path: impl Into<String>) -> Self {
        Self::new("GET", path)
    }

    pub fn post(path: impl Into<String>) -> Self {
        Self::new("POST", path)
    }

    pub fn with_body(mut self, body: serde_json::Value) -> Self {
        self.body = Some(body);
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    pub fn with_json_content_type(self) -> Self {
        self.with_header("Content-Type", "application/json")
    }

    pub fn build(self) -> serde_json::Value {
        serde_json::json!({
            "method": self.method,
            "path": self.path,
            "body": self.body,
            "headers": self.headers,
        })
    }
}

/// Create a test workload execution request
pub fn create_test_execution_request() -> serde_json::Value {
    serde_json::json!({
        "workload_type": "Wasm",
        "config": {
            "module_path": "test_module.wasm",
            "entry_point": "main",
        },
        "resources": {
            "cpu_cores": 1.0,
            "memory_mb": 256,
        },
        "timeout_seconds": 30,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_builder() {
        let config = TestServerConfigBuilder::new()
            .with_port(8080)
            .with_log_level("info")
            .build();

        assert_eq!(config["server"]["port"], 8080);
        assert_eq!(config["logging"]["level"], "info");
    }

    #[test]
    fn test_api_request_builder() {
        let request = TestApiRequestBuilder::post("/api/v1/execute")
            .with_json_content_type()
            .with_body(serde_json::json!({"test": "data"}))
            .build();

        assert_eq!(request["method"], "POST");
        assert_eq!(request["path"], "/api/v1/execute");
    }
}
