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

//! Test data builders for ToadStool components
//!
//! This module provides builder patterns for creating test data with
//! fluent APIs and sensible defaults to make testing more readable.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
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

/// Builder for ExecutionRequest
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Set execution ID
    pub fn execution_id(mut self, id: Uuid) -> Self {
        self.execution_id = Some(id);
        self
    }

    /// Set workload specification
    pub fn workload(mut self, workload: WorkloadSpec) -> Self {
        self.workload = Some(workload);
        self
    }

    /// Set native workload
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
    pub fn container_workload(mut self, image: &str, command: Option<Vec<String>>) -> Self {
        self.workload = Some(WorkloadSpec::Container {
            image: image.to_string(),
            command,
            args: None,
            working_dir: Some(TestConstants::TEST_WORKING_DIR.to_string()),
            user: None,
            volumes: vec![],
            ports: vec![],
            registry_auth: None,
        });
        self
    }

    /// Set WASM workload
    pub fn wasm_workload(mut self, module_data: Vec<u8>) -> Self {
        self.workload = Some(WorkloadSpec::Wasm {
            module_source: WasmModuleSource::Bytes { data: module_data },
            wasi_config: None,
            host_functions: vec![],
            memory_limit: Some(TestConstants::DEFAULT_MEMORY_LIMIT),
        });
        self
    }

    /// Set runtime hint
    pub fn runtime_hint(mut self, runtime: RuntimeType) -> Self {
        self.runtime_hint = Some(runtime);
        self
    }

    /// Set resource requirements
    pub fn resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Set security context
    pub fn security_context(mut self, context: SecurityContext) -> Self {
        self.security_context = Some(context);
        self
    }

    /// Set timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Add environment variable
    pub fn env_var(mut self, key: &str, value: &str) -> Self {
        self.environment.insert(key.to_string(), value.to_string());
        self
    }

    /// Set environment variables
    pub fn environment(mut self, env: HashMap<String, String>) -> Self {
        self.environment = env;
        self
    }

    /// Set input data
    pub fn input_data(mut self, input: ExecutionInput) -> Self {
        self.input_data = Some(input);
        self
    }

    /// Build the ExecutionRequest
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
        }
    }
}

/// Builder for ExecutionResponse
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Set execution ID
    pub fn execution_id(mut self, id: Uuid) -> Self {
        self.execution_id = Some(id);
        self
    }

    /// Set status to success
    pub fn success(mut self) -> Self {
        self.status = Some(ExecutionStatus::Success);
        self
    }

    /// Set status to failed
    pub fn failed(mut self, error: &str) -> Self {
        self.status = Some(ExecutionStatus::Failed {
            error: error.to_string(),
        });
        self
    }

    /// Set status to timed out
    pub fn timed_out(mut self) -> Self {
        self.status = Some(ExecutionStatus::TimedOut);
        self
    }

    /// Set status to cancelled
    pub fn cancelled(mut self) -> Self {
        self.status = Some(ExecutionStatus::Cancelled);
        self
    }

    /// Set status to resource limit exceeded
    pub fn resource_limit_exceeded(mut self, resource: &str, limit: &str, actual: &str) -> Self {
        self.status = Some(ExecutionStatus::ResourceLimitExceeded {
            resource: resource.to_string(),
            limit: limit.to_string(),
            actual: actual.to_string(),
        });
        self
    }

    /// Set status to security violation
    pub fn security_violation(mut self, violation: &str) -> Self {
        self.status = Some(ExecutionStatus::SecurityViolation {
            violation: violation.to_string(),
        });
        self
    }

    /// Set execution output
    pub fn output(mut self, output: ExecutionOutput) -> Self {
        self.output = Some(output);
        self
    }

    /// Set runtime metrics
    pub fn metrics(mut self, metrics: RuntimeMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Set execution duration
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Set runtime used
    pub fn runtime_used(mut self, runtime: RuntimeType) -> Self {
        self.runtime_used = Some(runtime);
        self
    }

    /// Add warning
    pub fn warning(mut self, warning: &str) -> Self {
        self.warnings.push(warning.to_string());
        self
    }

    /// Set warnings
    pub fn warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }

    /// Build the ExecutionResponse
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

/// Builder for SecurityContext
#[derive(Debug, Clone, Default)]
pub struct SecurityContextBuilder {
    isolation_level: Option<IsolationLevel>,
    capabilities: HashSet<Capability>,
    policies: Vec<toadstool::security::SecurityPolicy>,
    user_context: Option<toadstool::security::UserContext>,
    network_security: Option<NetworkSecurity>,
    filesystem_security: Option<FilesystemSecurity>,
    custom_security: HashMap<String, serde_json::Value>,
}

impl SecurityContextBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set isolation level
    pub fn isolation_level(mut self, level: IsolationLevel) -> Self {
        self.isolation_level = Some(level);
        self
    }

    /// Add capability
    pub fn capability(mut self, capability: Capability) -> Self {
        self.capabilities.insert(capability);
        self
    }

    /// Add multiple capabilities
    pub fn capabilities(mut self, capabilities: impl IntoIterator<Item = Capability>) -> Self {
        self.capabilities.extend(capabilities);
        self
    }

    /// Set network security
    pub fn network_security(mut self, network: NetworkSecurity) -> Self {
        self.network_security = Some(network);
        self
    }

    /// Set filesystem security
    pub fn filesystem_security(mut self, filesystem: FilesystemSecurity) -> Self {
        self.filesystem_security = Some(filesystem);
        self
    }

    /// Build the SecurityContext
    pub fn build(self) -> SecurityContext {
        let isolation_level = self.isolation_level.unwrap_or(IsolationLevel::Standard);

        SecurityContext {
            isolation_level,
            capabilities: if self.capabilities.is_empty() {
                isolation_level.default_capabilities()
            } else {
                self.capabilities
            },
            policies: self.policies,
            user_context: self.user_context,
            network_security: self.network_security.unwrap_or_default(),
            filesystem_security: self.filesystem_security.unwrap_or_default(),
            custom_security: self.custom_security,
        }
    }
}

/// Builder for RuntimeMetrics
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Set CPU metrics
    pub fn cpu_metrics(mut self, cpu: CpuMetrics) -> Self {
        self.cpu = Some(cpu);
        self
    }

    /// Set memory metrics
    pub fn memory_metrics(mut self, memory: MemoryMetrics) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Set storage metrics
    pub fn storage_metrics(mut self, storage: StorageMetrics) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Set network metrics
    pub fn network_metrics(mut self, network: NetworkMetrics) -> Self {
        self.network = Some(network);
        self
    }

    /// Set timing metrics
    pub fn timing_metrics(mut self, timing: TimingMetrics) -> Self {
        self.timing = Some(timing);
        self
    }

    /// Add custom metric
    pub fn custom_metric(mut self, key: &str, value: serde_json::Value) -> Self {
        self.custom.insert(key.to_string(), value);
        self
    }

    /// Build the RuntimeMetrics
    pub fn build(self) -> RuntimeMetrics {
        RuntimeMetrics {
            cpu: self.cpu.unwrap_or(CpuMetrics {
                usage_percent: 25.0,
                peak_usage_percent: 50.0,
                average_usage_percent: 30.0,
                cpu_time_ms: 1000,
                cpu_cycles: Some(500000),
                throttle_events: 0,
            }),
            memory: self.memory.unwrap_or(MemoryMetrics {
                usage_bytes: TestConstants::DEFAULT_MEMORY_LIMIT / 4,
                peak_usage_bytes: TestConstants::DEFAULT_MEMORY_LIMIT / 2,
                average_usage_bytes: TestConstants::DEFAULT_MEMORY_LIMIT / 3,
                allocation_count: 100,
                deallocation_count: 80,
                page_faults: 5,
                swap_usage_bytes: 0,
            }),
            storage: self.storage.unwrap_or(StorageMetrics {
                bytes_read: 1024 * 1024,
                bytes_written: 512 * 1024,
                read_ops: 50,
                write_ops: 25,
                read_iops: 500.0,
                write_iops: 250.0,
                avg_read_latency_us: 100.0,
                avg_write_latency_us: 200.0,
            }),
            network: self.network.unwrap_or(NetworkMetrics {
                bytes_received: 2048,
                bytes_transmitted: 1024,
                packets_received: 10,
                packets_transmitted: 8,
                errors: 0,
                drops: 0,
                avg_latency_us: 50.0,
            }),
            gpu: None,
            timing: self.timing.unwrap_or_else(|| TimingMetrics {
                start_time: Utc::now(),
                end_time: Some(Utc::now()),
                duration: Duration::from_secs(5),
                init_duration: Duration::from_millis(100),
                cleanup_duration: Duration::from_millis(50),
                queue_wait_duration: Duration::from_millis(10),
            }),
            custom: self.custom,
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
            .isolation_level(IsolationLevel::Enhanced)
            .capability(Capability::Execute)
            .capability(Capability::Read)
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

        assert_eq!(
            metrics.custom.get("test_metric"),
            Some(&serde_json::Value::Number(serde_json::Number::from(42)))
        );
        assert!(metrics.cpu.usage_percent > 0.0);
    }
}
