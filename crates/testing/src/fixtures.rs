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

//! Test fixtures for ToadStool components
//!
//! This module provides consistent test data and fixtures to eliminate
//! hardcoded values in tests and ensure reproducible test scenarios.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use fake::{Fake, Faker};
use uuid::Uuid;

use toadstool::{
    execution::{ExecutionInput, ExecutionOutput, ExecutionRequest, RuntimeConfig},
    resources::{
        CpuMetrics, CpuRequirements, MemoryMetrics, MemoryRequirements, NetworkMetrics,
        NetworkRequirements, ResourceRequirements, RuntimeMetrics, StorageMetrics,
        StorageRequirements, TimingMetrics,
    },
    security::{Capability, FilesystemSecurity, IsolationLevel, NetworkSecurity, SecurityContext},
    workload::{ExecutableSource, WasmModuleSource, WorkloadSpec},
};

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
    }
}

/// Create a test native workload
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
pub fn create_test_wasm_workload() -> WorkloadSpec {
    WorkloadSpec::Wasm {
        module_source: WasmModuleSource::Bytes {
            data: create_minimal_wasm_module(),
        },
        wasi_config: None,
        host_functions: vec![],
        memory_limit: Some(TestConstants::DEFAULT_MEMORY_LIMIT),
    }
}

/// Create a test container workload
pub fn create_test_container_workload() -> WorkloadSpec {
    WorkloadSpec::Container {
        image: TestConstants::TEST_CONTAINER_IMAGE.to_string(),
        command: Some(vec!["echo".to_string()]),
        args: Some(vec!["Hello from container!".to_string()]),
        working_dir: Some(TestConstants::TEST_WORKING_DIR.to_string()),
        user: None,
        volumes: vec![],
        ports: vec![],
        registry_auth: None,
    }
}

/// Create minimal WASM module for testing
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
pub fn create_test_resource_requirements() -> ResourceRequirements {
    ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: TestConstants::DEFAULT_CPU_CORES,
            max_cores: Some(TestConstants::DEFAULT_CPU_CORES * 2.0),
            architecture: Some("x86_64".to_string()),
            min_frequency_mhz: Some(2000),
            required_features: vec!["sse4".to_string(), "avx".to_string()],
        },
        memory: MemoryRequirements {
            min_bytes: TestConstants::DEFAULT_MEMORY_LIMIT / 2,
            max_bytes: Some(TestConstants::DEFAULT_MEMORY_LIMIT),
            memory_type: None,
            allow_swap: false,
        },
        storage: StorageRequirements {
            min_bytes: TestConstants::DEFAULT_STORAGE_LIMIT / 10,
            max_bytes: Some(TestConstants::DEFAULT_STORAGE_LIMIT),
            storage_type: None,
            min_iops: Some(1000),
            min_bandwidth_mbps: Some(100),
        },
        network: NetworkRequirements {
            min_bandwidth_mbps: Some(TestConstants::DEFAULT_NETWORK_BANDWIDTH),
            max_bandwidth_mbps: Some(TestConstants::DEFAULT_NETWORK_BANDWIDTH * 2),
            max_latency_ms: Some(100),
            internet_access: true,
            internal_access: true,
        },
        gpu: None,
        custom: HashMap::new(),
    }
}

/// Create test security context
pub fn create_test_security_context() -> SecurityContext {
    SecurityContext {
        isolation_level: IsolationLevel::Standard,
        capabilities: vec![
            Capability::Execute,
            Capability::Read,
            Capability::WriteTemp,
            Capability::NetworkClient,
        ]
        .into_iter()
        .collect(),
        policies: vec![],
        user_context: None,
        network_security: NetworkSecurity {
            internet_access: true,
            internal_access: true,
            allowed_hosts: vec![],
            allowed_ports: vec![],
            denied_hosts: vec![],
            dns_servers: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
        },
        filesystem_security: FilesystemSecurity {
            read_only: false,
            read_paths: vec![PathBuf::from("/usr"), PathBuf::from("/bin")],
            write_paths: vec![PathBuf::from("/tmp")],
            temp_access: true,
            hidden_paths: vec![],
            max_file_size: Some(100 * 1024 * 1024), // 100MB
        },
        custom_security: HashMap::new(),
    }
}

/// Create test environment variables
pub fn create_test_environment() -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("TEST_ENV".to_string(), "test_value".to_string());
    env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
    env.insert("HOME".to_string(), "/tmp".to_string());
    env
}

/// Create test execution input
pub fn create_test_execution_input() -> ExecutionInput {
    ExecutionInput {
        data: b"test input data".to_vec(),
        parameters: {
            let mut params = HashMap::new();
            params.insert(
                "test_param".to_string(),
                serde_json::Value::String("test_value".to_string()),
            );
            params
        },
        format: Some("text/plain".to_string()),
    }
}

/// Create test execution output
pub fn create_test_execution_output() -> ExecutionOutput {
    ExecutionOutput {
        data: b"test output data".to_vec(),
        result: {
            let mut result = HashMap::new();
            result.insert(
                "status".to_string(),
                serde_json::Value::String("success".to_string()),
            );
            result
        },
        stdout: Some("Test execution successful".to_string()),
        stderr: None,
        exit_code: Some(0),
        format: Some("text/plain".to_string()),
    }
}

/// Create test runtime configuration
pub fn create_test_runtime_config() -> RuntimeConfig {
    RuntimeConfig {
        runtime_config: {
            let mut config = HashMap::new();
            config.insert(
                "timeout".to_string(),
                serde_json::Value::Number(serde_json::Number::from(30)),
            );
            config.insert("debug".to_string(), serde_json::Value::Bool(true));
            config
        },
        platform_optimizations: true,
        debug_mode: true,
        telemetry_enabled: true,
    }
}

/// Create test runtime metrics
pub fn create_test_runtime_metrics() -> RuntimeMetrics {
    RuntimeMetrics {
        cpu: CpuMetrics {
            usage_percent: 25.5,
            peak_usage_percent: 45.2,
            average_usage_percent: 30.1,
            cpu_time_ms: 1500,
            cpu_cycles: Some(1000000),
            throttle_events: 0,
        },
        memory: MemoryMetrics {
            usage_bytes: TestConstants::DEFAULT_MEMORY_LIMIT / 4,
            peak_usage_bytes: TestConstants::DEFAULT_MEMORY_LIMIT / 2,
            average_usage_bytes: TestConstants::DEFAULT_MEMORY_LIMIT / 3,
            allocation_count: 150,
            deallocation_count: 120,
            page_faults: 5,
            swap_usage_bytes: 0,
        },
        storage: StorageMetrics {
            bytes_read: 1024 * 1024,
            bytes_written: 512 * 1024,
            read_ops: 100,
            write_ops: 50,
            read_iops: 1000.0,
            write_iops: 500.0,
            avg_read_latency_us: 100.5,
            avg_write_latency_us: 200.3,
        },
        network: NetworkMetrics {
            bytes_received: 2048,
            bytes_transmitted: 1024,
            packets_received: 20,
            packets_transmitted: 15,
            errors: 0,
            drops: 0,
            avg_latency_us: 50.2,
        },
        gpu: None,
        timing: TimingMetrics {
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            duration: Duration::from_secs(5),
            init_duration: Duration::from_millis(100),
            cleanup_duration: Duration::from_millis(50),
            queue_wait_duration: Duration::from_millis(10),
        },
        custom: HashMap::new(),
    }
}

/// Generate random test data using the Faker library
pub mod random {
    use super::*;

    /// Generate a random execution request
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
        }
    }

    /// Generate random native workload
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
    pub fn random_wasm_workload() -> WorkloadSpec {
        WorkloadSpec::Wasm {
            module_source: WasmModuleSource::Bytes {
                data: Faker.fake::<Vec<u8>>(),
            },
            wasi_config: None,
            host_functions: vec![],
            memory_limit: Some((1024 * 1024 * 1024).fake::<u64>()),
        }
    }

    /// Generate random container workload
    pub fn random_container_workload() -> WorkloadSpec {
        WorkloadSpec::Container {
            image: format!("{}:{}", Faker.fake::<String>(), Faker.fake::<String>()),
            command: Some(vec![Faker.fake::<String>()]),
            args: Some(vec![Faker.fake::<String>()]),
            working_dir: Some(format!("/app/{}", Faker.fake::<String>())),
            user: None,
            volumes: vec![],
            ports: vec![],
            registry_auth: None,
        }
    }

    /// Generate random resource requirements
    pub fn random_resource_requirements() -> ResourceRequirements {
        ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: (0.5..16.0).fake::<f64>(),
                max_cores: Some((1.0..32.0).fake::<f64>()),
                architecture: Some("x86_64".to_string()),
                min_frequency_mhz: Some((1000..4000).fake::<u32>()),
                required_features: vec![Faker.fake::<String>()],
            },
            memory: MemoryRequirements {
                min_bytes: (1024 * 1024).fake::<u64>(),
                max_bytes: Some((1024 * 1024 * 1024).fake::<u64>()),
                memory_type: None,
                allow_swap: rand::random::<bool>(),
            },
            storage: StorageRequirements {
                min_bytes: (1024 * 1024).fake::<u64>(),
                max_bytes: Some((1024 * 1024 * 1024).fake::<u64>()),
                storage_type: None,
                min_iops: Some((100..10000).fake::<u32>()),
                min_bandwidth_mbps: Some((10..1000).fake::<u32>()),
            },
            network: NetworkRequirements {
                min_bandwidth_mbps: Some((1..1000).fake::<u32>()),
                max_bandwidth_mbps: Some((100..10000).fake::<u32>()),
                max_latency_ms: Some((1..1000).fake::<u32>()),
                internet_access: rand::random::<bool>(),
                internal_access: rand::random::<bool>(),
            },
            gpu: None,
            custom: HashMap::new(),
        }
    }

    /// Generate random environment variables
    pub fn random_environment() -> HashMap<String, String> {
        let mut env = HashMap::new();
        for _ in 0..5 {
            env.insert(Faker.fake::<String>(), Faker.fake::<String>());
        }
        env
    }

    /// Generate random execution input
    pub fn random_execution_input() -> ExecutionInput {
        ExecutionInput {
            data: Faker.fake::<Vec<u8>>(),
            parameters: {
                let mut params = HashMap::new();
                for _ in 0..3 {
                    params.insert(
                        Faker.fake::<String>(),
                        serde_json::Value::String(Faker.fake::<String>()),
                    );
                }
                params
            },
            format: Some("application/octet-stream".to_string()),
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
            request.timeout.unwrap().as_secs(),
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
        assert_eq!(requirements.cpu.min_cores, TestConstants::DEFAULT_CPU_CORES);
        assert_eq!(
            requirements.memory.min_bytes,
            TestConstants::DEFAULT_MEMORY_LIMIT / 2
        );
        assert!(requirements.network.internet_access);
    }

    #[test]
    fn test_create_test_security_context() {
        let context = create_test_security_context();
        assert_eq!(context.isolation_level, IsolationLevel::Standard);
        assert!(context.capabilities.contains(&Capability::Execute));
        assert!(context.network_security.internet_access);
        assert!(context.filesystem_security.temp_access);
    }

    #[test]
    fn test_random_generation() {
        let request1 = random::execution_request();
        let request2 = random::execution_request();

        // Should generate different requests
        assert_ne!(request1.execution_id, request2.execution_id);
    }
}
