// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};
use toadstool::{ExecutionRequest, ExecutionResponse, ToadStoolResult};

/// Universal adapter for different execution environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalAdapter {
    /// Adapter configuration
    pub config: AdapterConfig,
}

/// Configuration for the universal adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Enable adapter
    pub enabled: bool,
    /// Supported environments
    pub environments: Vec<String>,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            environments: vec!["native".to_string(), "container".to_string()],
        }
    }
}

impl UniversalAdapter {
    /// Create a new adapter
    #[must_use]
    pub const fn new(config: AdapterConfig) -> Self {
        Self { config }
    }

    /// Adapt execution request for the target environment.
    ///
    /// Validates that the requested runtime is supported by this adapter's
    /// configured environments, and injects default timeout if missing.
    pub fn adapt_request(
        &self,
        mut request: ExecutionRequest,
    ) -> ToadStoolResult<ExecutionRequest> {
        if !self.config.enabled {
            return Err(toadstool::ToadStoolError::not_supported(
                "Adapter is disabled",
            ));
        }

        if let Some(ref hint) = request.runtime_hint {
            let runtime_name = format!("{hint:?}").to_lowercase();
            let is_supported = self
                .config
                .environments
                .iter()
                .any(|env| runtime_name.contains(env.as_str()) || env == "native");
            if !is_supported {
                tracing::warn!(
                    runtime = %runtime_name,
                    supported = ?self.config.environments,
                    "Runtime not in adapter environments, proceeding with native fallback"
                );
            }
        }

        if request.timeout.is_none() {
            request.timeout = Some(std::time::Duration::from_secs(300));
        }

        Ok(request)
    }

    /// Adapt execution response from the target environment.
    ///
    /// Passes through the response unchanged — adaptation is a request-side
    /// concern. Response validation belongs at the caller boundary.
    pub const fn adapt_response(
        &self,
        response: ExecutionResponse,
    ) -> ToadStoolResult<ExecutionResponse> {
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::collections::HashMap;
    use std::time::Duration;
    use toadstool::{ExecutionOutput, ExecutionStatus, RuntimeType};
    use uuid::Uuid;

    fn default_adapter() -> UniversalAdapter {
        UniversalAdapter::new(AdapterConfig::default())
    }

    #[test]
    fn test_adapt_request_default() {
        let adapter = default_adapter();
        let request = toadstool::ExecutionRequest::default();
        let exec_id = request.execution_id;
        let result = adapter.adapt_request(request);
        assert!(result.is_ok());
        let adapted = result.unwrap();
        assert_eq!(adapted.execution_id, exec_id);
    }

    #[test]
    fn test_adapt_request_with_runtime_hint() {
        let adapter = default_adapter();
        let request = toadstool::ExecutionRequest {
            runtime_hint: Some(RuntimeType::Wasm),
            ..toadstool::ExecutionRequest::default()
        };
        let result = adapter.adapt_request(request);
        assert!(result.is_ok());
        let adapted = result.unwrap();
        assert_eq!(adapted.runtime_hint, Some(RuntimeType::Wasm));
    }

    #[test]
    fn test_adapt_request_with_environment() {
        let adapter = default_adapter();
        let mut request = toadstool::ExecutionRequest::default();
        request
            .environment
            .insert("KEY".to_string(), "value".to_string());
        let result = adapter.adapt_request(request.clone());
        assert!(result.is_ok());
        let adapted = result.unwrap();
        assert_eq!(adapted.environment.get("KEY"), Some(&"value".to_string()));
    }

    #[test]
    fn test_adapt_request_with_timeout() {
        let adapter = default_adapter();
        let request = toadstool::ExecutionRequest {
            timeout: Some(Duration::from_secs(60)),
            ..toadstool::ExecutionRequest::default()
        };
        let result = adapter.adapt_request(request);
        assert!(result.is_ok());
        let adapted = result.unwrap();
        assert_eq!(adapted.timeout, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_adapt_response_default() {
        let adapter = default_adapter();
        let response = toadstool::ExecutionResponse::default();
        let result = adapter.adapt_response(response.clone());
        assert!(result.is_ok());
        let adapted = result.unwrap();
        assert_eq!(adapted.execution_id, response.execution_id);
        assert_eq!(adapted.status, response.status);
    }

    #[test]
    fn test_adapt_response_success() {
        let adapter = default_adapter();
        let response = toadstool::ExecutionResponse {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Success,
            output: ExecutionOutput::default(),
            metrics: toadstool::RuntimeMetrics::default(),
            duration: Duration::from_secs(5),
            runtime_used: RuntimeType::Native,
            warnings: vec![],
        };
        let result = adapter.adapt_response(response.clone());
        assert!(result.is_ok());
        let adapted = result.unwrap();
        assert_eq!(adapted.status, ExecutionStatus::Success);
        assert_eq!(adapted.duration, Duration::from_secs(5));
    }

    #[test]
    fn test_adapt_response_failed() {
        let adapter = default_adapter();
        let response = toadstool::ExecutionResponse {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Failed {
                error: std::borrow::Cow::Borrowed("test error"),
            },
            output: ExecutionOutput::default(),
            metrics: toadstool::RuntimeMetrics::default(),
            duration: Duration::from_secs(1),
            runtime_used: RuntimeType::Native,
            warnings: vec![],
        };
        let result = adapter.adapt_response(response.clone());
        assert!(result.is_ok());
        let adapted = result.unwrap();
        assert!(matches!(
            adapted.status,
            ExecutionStatus::Failed { error } if error == "test error"
        ));
    }

    #[test]
    fn test_adapt_response_with_output_data() {
        let adapter = default_adapter();
        let response = toadstool::ExecutionResponse {
            output: ExecutionOutput {
                data: Bytes::from("output bytes"),
                stdout: Some("stdout".to_string()),
                stderr: Some("stderr".to_string()),
                exit_code: Some(0),
                format: None,
                result: HashMap::new(),
                metadata: HashMap::new(),
            },
            ..toadstool::ExecutionResponse::default()
        };
        let result = adapter.adapt_response(response);
        assert!(result.is_ok());
        let adapted = result.unwrap();
        assert_eq!(adapted.output.data, Bytes::from("output bytes"));
        assert_eq!(adapted.output.stdout, Some("stdout".to_string()));
        assert_eq!(adapted.output.exit_code, Some(0));
    }

    #[test]
    fn test_adapt_request_response_passthrough_identity() {
        let adapter = default_adapter();
        let request = toadstool::ExecutionRequest::default();
        let adapted_req = adapter.adapt_request(request).unwrap();
        let response = toadstool::ExecutionResponse {
            execution_id: adapted_req.execution_id,
            ..toadstool::ExecutionResponse::default()
        };
        let adapted_resp = adapter.adapt_response(response.clone()).unwrap();
        assert_eq!(adapted_resp.execution_id, response.execution_id);
    }

    #[test]
    fn test_adapter_config_default_environments() {
        let config = AdapterConfig::default();
        assert!(config.enabled);
        assert!(config.environments.contains(&"native".to_string()));
        assert!(config.environments.contains(&"container".to_string()));
    }

    #[test]
    fn test_adapter_new() {
        let config = AdapterConfig {
            enabled: false,
            environments: vec!["custom".to_string()],
        };
        let adapter = UniversalAdapter::new(config.clone());
        assert_eq!(adapter.config.enabled, config.enabled);
        assert_eq!(adapter.config.environments, config.environments);
    }
}
