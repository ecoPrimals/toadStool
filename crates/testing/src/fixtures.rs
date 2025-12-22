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

//! Test fixtures for `ToadStool` components
//!
//! This module provides consistent test data and fixtures to eliminate
//! hardcoded values in tests and ensure reproducible test scenarios.

// Export integration test fixtures
pub mod runtime;
pub mod security;
pub mod server;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use fake::{Fake, Faker};
use uuid::Uuid;

use toadstool::{
    execution::{ExecutionInput, ExecutionOutput, ExecutionRequest, RuntimeConfig},
    resources::{
        CpuMetrics, CpuRequirements, MemoryMetrics, MemoryRequirements, NetworkMetrics,
        NetworkRequirements, ResourceRequirements, RuntimeMetrics, StorageMetrics,
        StorageRequirements, TimingMetrics,
    },
    security::{
        Capability, FilesystemSecurity, IsolationLevel, NetworkSecurity, SecurityContext,
        UserContext,
    },
    workload::{ExecutableSource, WasmModuleSource, WorkloadSpec},
};

// Re-export common test environment
pub use security::TestEnvironment;

/// Test configuration constants
pub struct TestConstants;

impl TestConstants {
    /// Default test memory limit in bytes (1GB)
    pub const DEFAULT_MEMORY_LIMIT: u64 = 1024 * 1024 * 1024;

    /// Default test CPU cores
    pub const DEFAULT_CPU_CORES: f64 = 2.0;

    /// Default test timeout in seconds
    pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

    /// Default test storage limit in bytes (10GB)
    pub const DEFAULT_STORAGE_LIMIT: u64 = 10 * 1024 * 1024 * 1024;

    /// Default test network bandwidth in Mbps
    pub const DEFAULT_NETWORK_BANDWIDTH: u32 = 100;

    /// Test container image
    pub const TEST_CONTAINER_IMAGE: &'static str = "alpine:latest";

    /// Test executable path
    pub const TEST_EXECUTABLE_PATH: &'static str = "/bin/echo";

    /// Test working directory
    pub const TEST_WORKING_DIR: &'static str = "/tmp";
}

/// Create a basic test execution request
#[must_use]
pub fn create_test_execution_request() -> ExecutionRequest {
    ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: create_test_native_workload(),
        runtime_hint: None,
        resources: create_test_resource_requirements(),
        security_context: create_test_security_context(),
        timeout: Some(Duration::from_secs(TestConstants::DEFAULT_TIMEOUT_SECS)),
        environment: create_test_environment(),
        input_data: create_test_execution_input(),
        callback_config: None,
        encryption_config: None,
    }
}

/// Create a test native workload
#[must_use]
pub fn create_test_native_workload() -> WorkloadSpec {
    WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: PathBuf::from(TestConstants::TEST_EXECUTABLE_PATH),
        },
        args: Some(vec!["Hello, ToadStool!".to_string()]),
        working_dir: Some(PathBuf::from(TestConstants::TEST_WORKING_DIR)),
        env_vars: create_test_environment(),
        user: None,
    }
}

/// Create a test WASM workload
#[must_use]
pub fn create_test_wasm_workload() -> WorkloadSpec {
    WorkloadSpec::Wasm {
        module: WasmModuleSource::Bytes {
            data: b"test wasm module".to_vec(),
        },
        args: Some(vec!["--version".to_string()]),
        wasi_config: None,
        env_vars: HashMap::new(),
    }
}

/// Create a test container workload
#[must_use]
pub fn create_test_container_workload() -> WorkloadSpec {
    WorkloadSpec::Container {
        image: "alpine:latest".to_string(),
        command: Some(vec!["echo".to_string()]),
        args: Some(vec!["Hello, Container!".to_string()]),
        env_vars: HashMap::new(),
        working_dir: None,
        volumes: vec![],
        ports: vec![],
        registry_auth: None,
    }
}

/// Create minimal WASM module for testing
#[must_use]
pub fn create_minimal_wasm_module() -> Vec<u8> {
    // Minimal WASM module that exports a simple function
    vec![
        0x00, 0x61, 0x73, 0x6d, // WASM magic number
        0x01, 0x00, 0x00, 0x00, // WASM version
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00, // Type section
        0x03, 0x02, 0x01, 0x00, // Function section
        0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b, // Code section
    ]
}

/// Create test resource requirements
#[must_use]
pub fn create_test_resource_requirements() -> ResourceRequirements {
    ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 1.0,
            max_cores: Some(2.0),
            architecture: Some("x86_64".to_string()),
        },
        memory: MemoryRequirements {
            min_bytes: TestConstants::DEFAULT_MEMORY_LIMIT,
            max_bytes: Some(TestConstants::DEFAULT_MEMORY_LIMIT * 2),
        },
        storage: StorageRequirements {
            min_bytes: TestConstants::DEFAULT_STORAGE_LIMIT,
            max_bytes: Some(TestConstants::DEFAULT_STORAGE_LIMIT * 2),
            storage_type: Some("ssd".to_string()),
        },
        network: NetworkRequirements {
            min_bandwidth: Some(1000),
            max_bandwidth: Some(10000),
            max_latency_ms: Some(100),
        },
        gpu: None,
    }
}

/// Create test security context
#[must_use]
pub fn create_test_security_context() -> SecurityContext {
    SecurityContext {
        isolation_level: IsolationLevel::Standard,
        capabilities: vec![Capability::Execute, Capability::Read],
        user_context: Some(UserContext {
            username: Some("test_user".to_string()),
            uid: Some(1000),
            gid: Some(1000),
            groups: vec![],
        }),
        network_security: NetworkSecurity {
            allow_outbound: true,
            allow_inbound: true,
            allowed_domains: vec![],
            blocked_domains: vec![],
            allowed_ports: vec![],
            blocked_ports: vec![],
        },
        filesystem_security: FilesystemSecurity {
            read_only: false,
            allowed_read_paths: vec!["/usr".to_string(), "/bin".to_string()],
            allowed_write_paths: vec!["/tmp".to_string()],
            blocked_paths: vec![],
        },
    }
}

/// Create test environment variables
#[must_use]
pub fn create_test_environment() -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("TEST_ENV".to_string(), "test_value".to_string());
    env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
    env.insert("HOME".to_string(), "/tmp".to_string());
    env
}

/// Create test execution input
#[must_use]
pub fn create_test_execution_input() -> ExecutionInput {
    ExecutionInput {
        data: b"test input data".to_vec(),
        format: Some("text/plain".to_string()),
        metadata: {
            let mut map = HashMap::new();
            map.insert("test_key".to_string(), "test_value".to_string());
            map
        },
    }
}

/// Create test execution output
#[must_use]
pub fn create_test_execution_output() -> ExecutionOutput {
    ExecutionOutput {
        data: b"test output data".to_vec(),
        result: {
            let mut result = HashMap::new();
            result.insert("status".to_string(), "success".to_string());
            result
        },
        stdout: Some("Test execution successful".to_string()),
        stderr: None,
        exit_code: Some(0),
        format: Some("text/plain".to_string()),
        metadata: HashMap::new(),
    }
}

/// Create test runtime configuration
#[must_use]
pub fn create_test_runtime_config() -> RuntimeConfig {
    RuntimeConfig {
        settings: {
            let mut config = HashMap::new();
            config.insert(
                "timeout".to_string(),
                serde_json::Value::String("30".to_string()),
            );
            config
        },
        resource_limits: None,
        security_settings: None,
        logging: None,
    }
}

/// Create test runtime metrics
#[must_use]
pub fn create_test_runtime_metrics() -> RuntimeMetrics {
    RuntimeMetrics {
        cpu: CpuMetrics {
            usage_percent: 25.0,
            cores_used: 0.5,
            cpu_time_seconds: 1.0,
        },
        memory: MemoryMetrics {
            usage_percent: 75.0,
            used_bytes: TestConstants::DEFAULT_MEMORY_LIMIT / 4,
            peak_bytes: TestConstants::DEFAULT_MEMORY_LIMIT / 2,
        },
        storage: StorageMetrics {
            usage_percent: 25.0,
            used_bytes: TestConstants::DEFAULT_STORAGE_LIMIT / 8,
            bytes_read: 1024,
            bytes_written: 512,
        },
        network: NetworkMetrics {
            bytes_sent: 512,
            bytes_received: 1024,
            packets_sent: 4,
            packets_received: 8,
        },
        gpu: None,
        timing: TimingMetrics {
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
            duration: chrono::Duration::seconds(5),
        },
    }
}

/// Generate random test data using the Faker library
pub mod random {
    use super::{
        create_test_security_context, CpuRequirements, Duration, ExecutableSource, ExecutionInput,
        ExecutionRequest, Fake, Faker, HashMap, MemoryRequirements, NetworkRequirements, PathBuf,
        ResourceRequirements, StorageRequirements, Uuid, WasmModuleSource, WorkloadSpec,
    };

    /// Generate a random execution request
    #[must_use]
    pub fn execution_request() -> ExecutionRequest {
        ExecutionRequest {
            execution_id: Uuid::new_v4(),
            workload: match rand::random::<u8>() % 3 {
                0 => random_native_workload(),
                1 => random_wasm_workload(),
                _ => random_container_workload(),
            },
            runtime_hint: None,
            resources: random_resource_requirements(),
            security_context: create_test_security_context(),
            timeout: Some(Duration::from_secs((10..300).fake::<u64>())),
            environment: random_environment(),
            input_data: random_execution_input(),
            callback_config: None,
            encryption_config: None,
        }
    }

    /// Generate random native workload
    #[must_use]
    pub fn random_native_workload() -> WorkloadSpec {
        WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: PathBuf::from(format!("/bin/{}", Faker.fake::<String>())),
            },
            args: Some(vec![Faker.fake::<String>(), Faker.fake::<String>()]),
            working_dir: Some(PathBuf::from(format!("/tmp/{}", Faker.fake::<String>()))),
            env_vars: random_environment(),
            user: None,
        }
    }

    /// Generate random WASM workload
    #[must_use]
    pub fn random_wasm_workload() -> WorkloadSpec {
        WorkloadSpec::Wasm {
            module: WasmModuleSource::Bytes {
                data: Faker.fake::<Vec<u8>>(),
            },
            args: Some(vec![Faker.fake::<String>()]),
            wasi_config: None,
            env_vars: HashMap::new(),
        }
    }

    /// Generate random container workload
    #[must_use]
    pub fn random_container_workload() -> WorkloadSpec {
        WorkloadSpec::Container {
            image: Faker.fake::<String>(),
            command: Some(vec![Faker.fake::<String>()]),
            args: Some(vec![Faker.fake::<String>()]),
            env_vars: HashMap::new(),
            working_dir: Some(Faker.fake::<String>()),
            volumes: vec![],
            ports: vec![],
            registry_auth: None,
        }
    }

    /// Generate random resource requirements
    #[must_use]
    pub fn random_resource_requirements() -> ResourceRequirements {
        ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: (0.5..8.0).fake::<f64>(),
                max_cores: Some((0.5..8.0).fake::<f64>()),
                architecture: Some(Faker.fake::<String>()),
            },
            memory: MemoryRequirements {
                min_bytes: (1024 * 1024 * 512..1024 * 1024 * 1024 * 8).fake::<u64>(),
                max_bytes: Some((1024 * 1024 * 1024 * 8..1024 * 1024 * 1024 * 16).fake::<u64>()),
            },
            storage: StorageRequirements {
                min_bytes: (1024 * 1024 * 1024..1024 * 1024 * 1024 * 100).fake::<u64>(),
                max_bytes: Some(
                    (1024 * 1024 * 1024 * 100..1024 * 1024 * 1024 * 1000).fake::<u64>(),
                ),
                storage_type: Some(Faker.fake::<String>()),
            },
            network: NetworkRequirements {
                min_bandwidth: Some((1..1000).fake::<u64>()),
                max_bandwidth: Some((100..10000).fake::<u64>()),
                max_latency_ms: Some((1..1000).fake::<u64>()),
            },
            gpu: None,
        }
    }

    /// Generate random environment variables
    #[must_use]
    pub fn random_environment() -> HashMap<String, String> {
        let mut env = HashMap::new();
        for _ in 0..5 {
            env.insert(Faker.fake::<String>(), Faker.fake::<String>());
        }
        env
    }

    /// Generate random execution input
    #[must_use]
    pub fn random_execution_input() -> ExecutionInput {
        ExecutionInput {
            data: Faker.fake::<Vec<u8>>(),
            format: Some("application/octet-stream".to_string()),
            metadata: {
                let mut params = HashMap::new();
                for _ in 0..3 {
                    params.insert(Faker.fake::<String>(), Faker.fake::<String>());
                }
                params
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_execution_request() {
        let request = create_test_execution_request();
        assert!(!request.execution_id.is_nil());
        assert!(request.timeout.is_some());
        assert_eq!(
            request
                .timeout
                .expect("Test request should have timeout configured")
                .as_secs(),
            TestConstants::DEFAULT_TIMEOUT_SECS
        );
    }

    #[test]
    fn test_create_minimal_wasm_module() {
        let module = create_minimal_wasm_module();
        assert!(!module.is_empty());
        // Check WASM magic number
        assert_eq!(&module[0..4], &[0x00, 0x61, 0x73, 0x6d]);
    }

    #[test]
    fn test_create_test_resource_requirements() {
        let requirements = create_test_resource_requirements();
        assert_eq!(requirements.cpu.min_cores, 1.0);
        assert_eq!(
            requirements.memory.min_bytes,
            TestConstants::DEFAULT_MEMORY_LIMIT
        );
        assert!(requirements.network.min_bandwidth.is_some());
    }

    #[test]
    fn test_create_test_security_context() {
        let context = create_test_security_context();
        assert_eq!(context.isolation_level, IsolationLevel::Standard);
        assert!(context.capabilities.contains(&Capability::Execute));
        assert!(context.network_security.allow_outbound);
        assert!(!context.filesystem_security.read_only);
    }

    #[test]
    fn test_random_generation() {
        let request1 = random::execution_request();
        let request2 = random::execution_request();

        // Should generate different requests
        assert_ne!(request1.execution_id, request2.execution_id);
    }
}
