// SPDX-License-Identifier: AGPL-3.0-or-later
// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Test data builders for `ToadStool` components
//!
//! This module provides builder patterns for creating test data with
//! fluent APIs and sensible defaults to make testing more readable.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use uuid::Uuid;

use toadstool::{
    execution::{
        ExecutionInput, ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus,
        RuntimeType,
    },
    resources::{
        CpuMetrics, MemoryMetrics, NetworkMetrics, ResourceRequirements, RuntimeMetrics,
        StorageMetrics, TimingMetrics,
    },
    security::{Capability, FilesystemSecurity, IsolationLevel, NetworkSecurity, SecurityContext},
    workload::{ExecutableSource, WasmModuleSource, WorkloadSpec},
};

use crate::fixtures::TestConstants;

/// Builder for `ExecutionRequest`
#[derive(Debug, Clone, Default)]
pub struct ExecutionRequestBuilder {
    execution_id: Option<Uuid>,
    workload: Option<WorkloadSpec>,
    runtime_hint: Option<RuntimeType>,
    resources: Option<ResourceRequirements>,
    security_context: Option<SecurityContext>,
    timeout: Option<Duration>,
    environment: HashMap<String, String>,
    input_data: Option<ExecutionInput>,
}

impl ExecutionRequestBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set execution ID
    #[must_use]
    pub fn execution_id(mut self, id: Uuid) -> Self {
        self.execution_id = Some(id);
        self
    }

    /// Set workload specification
    #[must_use]
    pub fn workload(mut self, workload: WorkloadSpec) -> Self {
        self.workload = Some(workload);
        self
    }

    /// Set native workload
    #[must_use]
    pub fn native_workload(mut self, executable: &str, args: Vec<String>) -> Self {
        self.workload = Some(WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: PathBuf::from(executable),
            },
            args: Some(args),
            working_dir: Some(PathBuf::from(TestConstants::TEST_WORKING_DIR)),
            env_vars: HashMap::new(),
            user: None,
        });
        self
    }

    /// Set container workload
    #[must_use]
    pub fn container_workload(mut self, image: &str, command: Option<Vec<String>>) -> Self {
        self.workload = Some(WorkloadSpec::Container {
            image: image.to_string(),
            command,
            args: None,
            env_vars: HashMap::new(),
            working_dir: Some(TestConstants::TEST_WORKING_DIR.to_string()),
            volumes: vec![],
            ports: vec![],
            registry_auth: None,
        });
        self
    }

    /// Set WASM workload
    #[must_use]
    pub fn wasm_workload(mut self, module_data: Vec<u8>) -> Self {
        self.workload = Some(WorkloadSpec::Wasm {
            module: WasmModuleSource::Bytes {
                data: module_data.into(),
            },
            args: None,
            wasi_config: None,
            env_vars: HashMap::new(),
        });
        self
    }

    /// Set runtime hint
    #[must_use]
    pub fn runtime_hint(mut self, runtime: RuntimeType) -> Self {
        self.runtime_hint = Some(runtime);
        self
    }

    /// Set resource requirements
    #[must_use]
    pub fn resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Set security context
    #[must_use]
    pub fn security_context(mut self, context: SecurityContext) -> Self {
        self.security_context = Some(context);
        self
    }

    /// Set timeout
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Add environment variable
    #[must_use]
    pub fn env_var(mut self, key: &str, value: &str) -> Self {
        self.environment.insert(key.to_string(), value.to_string());
        self
    }

    /// Set environment variables
    #[must_use]
    pub fn environment(mut self, env: HashMap<String, String>) -> Self {
        self.environment = env;
        self
    }

    /// Set input data
    #[must_use]
    pub fn input_data(mut self, input: ExecutionInput) -> Self {
        self.input_data = Some(input);
        self
    }

    /// Build the `ExecutionRequest`
    pub fn build(self) -> ExecutionRequest {
        ExecutionRequest {
            execution_id: self.execution_id.unwrap_or_else(Uuid::new_v4),
            workload: self.workload.unwrap_or_else(|| WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: PathBuf::from(TestConstants::TEST_EXECUTABLE_PATH),
                },
                args: Some(vec!["test".to_string()]),
                working_dir: Some(PathBuf::from(TestConstants::TEST_WORKING_DIR)),
                env_vars: HashMap::new(),
                user: None,
            }),
            runtime_hint: self.runtime_hint,
            resources: self
                .resources
                .unwrap_or_else(crate::fixtures::create_test_resource_requirements),
            security_context: self
                .security_context
                .unwrap_or_else(crate::fixtures::create_test_security_context),
            timeout: self
                .timeout
                .or_else(|| Some(Duration::from_secs(TestConstants::DEFAULT_TIMEOUT_SECS))),
            environment: self.environment,
            input_data: self
                .input_data
                .unwrap_or_else(crate::fixtures::create_test_execution_input),
            callback_config: None,
            encryption_config: None,
        }
    }
}

/// Builder for `ExecutionResponse`
#[derive(Debug, Clone, Default)]
pub struct ExecutionResponseBuilder {
    execution_id: Option<Uuid>,
    status: Option<ExecutionStatus>,
    output: Option<ExecutionOutput>,
    metrics: Option<RuntimeMetrics>,
    duration: Option<Duration>,
    runtime_used: Option<RuntimeType>,
    warnings: Vec<String>,
}

impl ExecutionResponseBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set execution ID
    #[must_use]
    pub fn execution_id(mut self, id: Uuid) -> Self {
        self.execution_id = Some(id);
        self
    }

    /// Set status to success
    #[must_use]
    pub fn success(mut self) -> Self {
        self.status = Some(ExecutionStatus::Success);
        self
    }

    /// Set status to failed
    #[must_use]
    pub fn failed(mut self, error: &str) -> Self {
        self.status = Some(ExecutionStatus::Failed {
            error: error.to_string(),
        });
        self
    }

    /// Set status to timed out
    #[must_use]
    pub fn timed_out(mut self) -> Self {
        self.status = Some(ExecutionStatus::TimedOut);
        self
    }

    /// Set status to cancelled
    #[must_use]
    pub fn cancelled(mut self) -> Self {
        self.status = Some(ExecutionStatus::Cancelled);
        self
    }

    /// Set status to resource limit exceeded (using failed status)
    #[must_use]
    pub fn resource_limit_exceeded(mut self, resource: &str, limit: &str, actual: &str) -> Self {
        self.status = Some(ExecutionStatus::Failed {
            error: format!(
                "Resource limit exceeded: {resource} (limit: {limit}, actual: {actual})"
            ),
        });
        self
    }

    /// Set status to security violation (using failed status)
    #[must_use]
    pub fn security_violation(mut self, violation: &str) -> Self {
        self.status = Some(ExecutionStatus::Failed {
            error: format!("Security violation: {violation}"),
        });
        self
    }

    /// Set execution output
    #[must_use]
    pub fn output(mut self, output: ExecutionOutput) -> Self {
        self.output = Some(output);
        self
    }

    /// Set runtime metrics
    #[must_use]
    pub fn metrics(mut self, metrics: RuntimeMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Set execution duration
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Set runtime used
    #[must_use]
    pub fn runtime_used(mut self, runtime: RuntimeType) -> Self {
        self.runtime_used = Some(runtime);
        self
    }

    /// Add warning
    #[must_use]
    pub fn warning(mut self, warning: &str) -> Self {
        self.warnings.push(warning.to_string());
        self
    }

    /// Set warnings
    #[must_use]
    pub fn warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }

    /// Build the `ExecutionResponse`
    pub fn build(self) -> ExecutionResponse {
        ExecutionResponse {
            execution_id: self.execution_id.unwrap_or_else(Uuid::new_v4),
            status: self.status.unwrap_or(ExecutionStatus::Success),
            output: self
                .output
                .unwrap_or_else(crate::fixtures::create_test_execution_output),
            metrics: self
                .metrics
                .unwrap_or_else(crate::fixtures::create_test_runtime_metrics),
            duration: self.duration.unwrap_or_else(|| Duration::from_secs(5)),
            runtime_used: self.runtime_used.unwrap_or(RuntimeType::Native),
            warnings: self.warnings,
        }
    }
}

/// Builder for `SecurityContext`
#[derive(Debug, Clone)]
pub struct SecurityContextBuilder {
    isolation_level: IsolationLevel,
    capabilities: Vec<Capability>,
    policies: Vec<String>,
    _custom_security: HashMap<String, String>,
}

impl Default for SecurityContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityContextBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            isolation_level: IsolationLevel::Standard,
            capabilities: vec![Capability::Execute],
            policies: vec![],
            _custom_security: HashMap::new(),
        }
    }

    #[must_use]
    pub fn with_isolation_level(mut self, level: IsolationLevel) -> Self {
        self.isolation_level = level;
        self
    }

    #[must_use]
    pub fn with_capabilities(mut self, capabilities: Vec<Capability>) -> Self {
        self.capabilities.extend(capabilities);
        self
    }

    #[must_use]
    pub fn with_policy(mut self, policy: String) -> Self {
        self.policies.push(policy);
        self
    }

    #[must_use]
    pub fn with_resource_limit_exceeded(self, _message: String) -> Self {
        // Note: SecurityContext doesn't have status, this is for builder compatibility
        self
    }

    #[must_use]
    pub fn with_security_violation(self, _message: String) -> Self {
        // Note: SecurityContext doesn't have status, this is for builder compatibility
        self
    }

    #[must_use]
    pub fn build(self) -> SecurityContext {
        SecurityContext {
            isolation_level: self.isolation_level,
            capabilities: self.capabilities,
            user_context: None,
            network_security: NetworkSecurity::default(),
            filesystem_security: FilesystemSecurity::default(),
        }
    }
}

/// Builder for `RuntimeMetrics`
#[derive(Debug, Clone, Default)]
pub struct RuntimeMetricsBuilder {
    cpu: Option<CpuMetrics>,
    memory: Option<MemoryMetrics>,
    storage: Option<StorageMetrics>,
    network: Option<NetworkMetrics>,
    timing: Option<TimingMetrics>,
    custom: HashMap<String, serde_json::Value>,
}

impl RuntimeMetricsBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set CPU metrics
    #[must_use]
    pub fn cpu_metrics(mut self, cpu: CpuMetrics) -> Self {
        self.cpu = Some(cpu);
        self
    }

    /// Set memory metrics
    #[must_use]
    pub fn memory_metrics(mut self, memory: MemoryMetrics) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Set storage metrics
    #[must_use]
    pub fn storage_metrics(mut self, storage: StorageMetrics) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Set network metrics
    #[must_use]
    pub fn network_metrics(mut self, network: NetworkMetrics) -> Self {
        self.network = Some(network);
        self
    }

    /// Set timing metrics
    #[must_use]
    pub fn timing_metrics(mut self, timing: TimingMetrics) -> Self {
        self.timing = Some(timing);
        self
    }

    /// Add custom metric
    #[must_use]
    pub fn custom_metric(mut self, key: &str, value: serde_json::Value) -> Self {
        self.custom.insert(key.to_string(), value);
        self
    }

    /// Build the `RuntimeMetrics`
    #[must_use]
    pub fn build(self) -> RuntimeMetrics {
        RuntimeMetrics {
            cpu: self.cpu.unwrap_or(CpuMetrics {
                usage_percent: 30.0,
                cores_used: 1.5,
                cpu_time_seconds: 2.0,
            }),
            memory: self.memory.unwrap_or(MemoryMetrics {
                usage_percent: 60.0,
                used_bytes: TestConstants::DEFAULT_MEMORY_LIMIT / 4,
                peak_bytes: TestConstants::DEFAULT_MEMORY_LIMIT / 2,
            }),
            storage: self.storage.unwrap_or(StorageMetrics {
                usage_percent: 30.0,
                used_bytes: TestConstants::DEFAULT_STORAGE_LIMIT / 10,
                bytes_read: 1024 * 1024,
                bytes_written: 512 * 1024,
            }),
            network: self.network.unwrap_or(NetworkMetrics {
                bytes_sent: 1024,
                bytes_received: 2048,
                packets_sent: 8,
                packets_received: 16,
            }),
            gpu: None,
            timing: self.timing.unwrap_or(TimingMetrics {
                start_time: std::time::SystemTime::now(),
                end_time: Some(std::time::SystemTime::now()),
                duration: std::time::Duration::from_secs(5),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_request_builder() {
        let request = ExecutionRequestBuilder::new()
            .native_workload("/bin/echo", vec!["hello".to_string()])
            .timeout(Duration::from_secs(60))
            .env_var("TEST", "value")
            .build();

        assert!(matches!(request.workload, WorkloadSpec::Native { .. }));
        assert_eq!(request.timeout, Some(Duration::from_secs(60)));
        assert_eq!(request.environment.get("TEST"), Some(&"value".to_string()));
    }

    #[test]
    fn test_execution_response_builder() {
        let response = ExecutionResponseBuilder::new()
            .success()
            .duration(Duration::from_secs(10))
            .runtime_used(RuntimeType::Container)
            .warning("Test warning")
            .build();

        assert_eq!(response.status, ExecutionStatus::Success);
        assert_eq!(response.duration, Duration::from_secs(10));
        assert_eq!(response.runtime_used, RuntimeType::Container);
        assert_eq!(response.warnings, vec!["Test warning".to_string()]);
    }

    #[test]
    fn test_security_context_builder() {
        let context = SecurityContextBuilder::new()
            .with_isolation_level(IsolationLevel::Enhanced)
            .with_capabilities(vec![Capability::Execute, Capability::Read])
            .build();

        assert_eq!(context.isolation_level, IsolationLevel::Enhanced);
        assert!(context.capabilities.contains(&Capability::Execute));
        assert!(context.capabilities.contains(&Capability::Read));
    }

    #[test]
    fn test_runtime_metrics_builder() {
        let metrics = RuntimeMetricsBuilder::new()
            .custom_metric(
                "test_metric",
                serde_json::Value::Number(serde_json::Number::from(42)),
            )
            .build();

        // Custom metrics are not directly accessible via fields
        // Test passes if we can build metrics
        assert!(metrics.cpu.usage_percent > 0.0);
    }

    #[test]
    fn test_execution_request_builder_defaults() {
        let request = ExecutionRequestBuilder::new().build();
        assert!(request.execution_id != uuid::Uuid::nil());
        assert!(matches!(request.workload, WorkloadSpec::Native { .. }));
        assert_eq!(
            request.timeout,
            Some(Duration::from_secs(TestConstants::DEFAULT_TIMEOUT_SECS))
        );
    }

    #[test]
    fn test_execution_request_builder_container_workload() {
        let request = ExecutionRequestBuilder::new()
            .container_workload("alpine:latest", Some(vec!["sh".to_string()]))
            .build();
        assert!(matches!(request.workload, WorkloadSpec::Container { .. }));
        if let WorkloadSpec::Container { image, .. } = &request.workload {
            assert_eq!(image, "alpine:latest");
        }
    }

    #[test]
    fn test_execution_request_builder_wasm_workload() {
        let request = ExecutionRequestBuilder::new()
            .wasm_workload(vec![0x00, 0x61, 0x73, 0x6d])
            .build();
        assert!(matches!(request.workload, WorkloadSpec::Wasm { .. }));
    }

    #[test]
    fn test_execution_request_builder_execution_id() {
        let id = uuid::Uuid::new_v4();
        let request = ExecutionRequestBuilder::new().execution_id(id).build();
        assert_eq!(request.execution_id, id);
    }

    #[test]
    fn test_execution_response_builder_failed() {
        let response = ExecutionResponseBuilder::new()
            .failed("Test error message")
            .build();
        assert!(matches!(response.status, ExecutionStatus::Failed { .. }));
        if let ExecutionStatus::Failed { error } = &response.status {
            assert_eq!(error, "Test error message");
        }
    }

    #[test]
    fn test_execution_response_builder_cancelled() {
        let response = ExecutionResponseBuilder::new().cancelled().build();
        assert_eq!(response.status, ExecutionStatus::Cancelled);
    }

    #[test]
    fn test_execution_response_builder_timed_out() {
        let response = ExecutionResponseBuilder::new().timed_out().build();
        assert_eq!(response.status, ExecutionStatus::TimedOut);
    }

    #[test]
    fn test_security_context_builder_default() {
        let context = SecurityContextBuilder::new().build();
        assert_eq!(context.isolation_level, IsolationLevel::Standard);
        assert!(context.capabilities.contains(&Capability::Execute));
    }

    #[test]
    fn test_security_context_builder_with_policy() {
        let context = SecurityContextBuilder::new()
            .with_policy("deny-all".to_string())
            .build();
        assert_eq!(context.isolation_level, IsolationLevel::Standard);
    }

    #[test]
    fn test_runtime_metrics_builder_with_cpu() {
        let metrics = RuntimeMetricsBuilder::new()
            .cpu_metrics(CpuMetrics {
                usage_percent: 99.0,
                cores_used: 4.0,
                cpu_time_seconds: 10.0,
            })
            .build();
        assert_eq!(metrics.cpu.usage_percent, 99.0);
        assert_eq!(metrics.cpu.cores_used, 4.0);
    }

    #[test]
    fn test_runtime_metrics_builder_with_memory() {
        let metrics = RuntimeMetricsBuilder::new()
            .memory_metrics(MemoryMetrics {
                usage_percent: 80.0,
                used_bytes: 1024 * 1024 * 512,
                peak_bytes: 1024 * 1024 * 768,
            })
            .build();
        assert_eq!(metrics.memory.usage_percent, 80.0);
        assert_eq!(metrics.memory.used_bytes, 1024 * 1024 * 512);
    }
}
