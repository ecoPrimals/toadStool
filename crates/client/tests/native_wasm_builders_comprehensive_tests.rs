// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Native and WASM workload builders
//!
//! This test suite provides extensive coverage for:
//! - `NativeWorkloadBuilder` (all methods and patterns)
//! - `WasmWorkloadBuilder` (all methods and patterns)
//! - Builder pattern validation
//! - Error handling
//! - Edge cases

use std::collections::HashMap;
use std::time::Duration;
use toadstool_client::{
    JobPriority, NativeWorkloadBuilder, ResourceRequirements, WasmWorkloadBuilder, WorkloadType,
};

// ============================================================================
// NativeWorkloadBuilder Tests
// ============================================================================

#[test]
fn test_native_builder_new() {
    let builder = NativeWorkloadBuilder::new();

    // Builder should be created successfully
    // Can't inspect private fields, but we can verify it builds with required fields
    let result = builder.executable("/bin/echo").build();
    assert!(result.is_ok());
}

#[test]
fn test_native_builder_default() {
    let builder = NativeWorkloadBuilder::default();

    let result = builder.executable("/bin/ls").build();
    assert!(result.is_ok());
}

#[test]
fn test_native_builder_minimal_config() {
    let workload = NativeWorkloadBuilder::new()
        .executable("/bin/echo")
        .build()
        .expect("Should build with just executable");

    match workload.workload_type {
        WorkloadType::Native {
            executable,
            args,
            working_dir,
        } => {
            assert_eq!(executable, "/bin/echo");
            assert!(args.is_empty());
            assert!(working_dir.is_none());
        }
        _ => panic!("Expected Native workload type"),
    }

    assert_eq!(workload.runtime_hint, Some("native".to_string()));
}

#[test]
fn test_native_builder_with_args() {
    let args = vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()];

    let workload = NativeWorkloadBuilder::new()
        .executable("/bin/test")
        .args(args.clone())
        .build()
        .expect("Should build with args");

    match workload.workload_type {
        WorkloadType::Native {
            executable: _,
            args: workload_args,
            working_dir: _,
        } => {
            assert_eq!(workload_args, args);
        }
        _ => panic!("Expected Native workload type"),
    }
}

#[test]
fn test_native_builder_with_working_dir() {
    let workload = NativeWorkloadBuilder::new()
        .executable("/usr/bin/make")
        .working_dir("/home/user/project")
        .build()
        .expect("Should build with working_dir");

    match workload.workload_type {
        WorkloadType::Native {
            executable: _,
            args: _,
            working_dir,
        } => {
            assert_eq!(working_dir, Some("/home/user/project".to_string()));
        }
        _ => panic!("Expected Native workload type"),
    }
}

#[test]
fn test_native_builder_with_environment() {
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
    env.insert("HOME".to_string(), "/home/user".to_string());

    let workload = NativeWorkloadBuilder::new()
        .executable("/bin/bash")
        .environment(env.clone())
        .build()
        .expect("Should build with environment");

    assert_eq!(
        workload.environment.get("PATH"),
        Some(&"/usr/bin:/bin".to_string())
    );
    assert_eq!(
        workload.environment.get("HOME"),
        Some(&"/home/user".to_string())
    );
}

#[test]
fn test_native_builder_with_priority() {
    let workload = NativeWorkloadBuilder::new()
        .executable("/bin/nice")
        .priority(JobPriority::High)
        .build()
        .expect("Should build with priority");

    assert_eq!(workload.priority, Some(JobPriority::High));
}

#[test]
fn test_native_builder_with_timeout() {
    let timeout = Duration::from_secs(300);

    let workload = NativeWorkloadBuilder::new()
        .executable("/bin/sleep")
        .timeout(timeout)
        .build()
        .expect("Should build with timeout");

    assert_eq!(workload.timeout, Some(timeout));
}

#[test]
fn test_native_builder_with_resources() {
    let resources = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: Some(2048),
        disk_mb: Some(10240),
        gpu_required: Some(false),
    };

    let workload = NativeWorkloadBuilder::new()
        .executable("/usr/bin/gcc")
        .resources(resources.clone())
        .build()
        .expect("Should build with resources");

    assert_eq!(workload.resources, Some(resources));
}

#[test]
fn test_native_builder_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("project".to_string(), "toadstool".to_string());
    metadata.insert("version".to_string(), "0.1.0".to_string());

    let workload = NativeWorkloadBuilder::new()
        .executable("/bin/echo")
        .metadata(metadata.clone())
        .build()
        .expect("Should build with metadata");

    assert_eq!(
        workload.metadata.get("project"),
        Some(&"toadstool".to_string())
    );
    assert_eq!(workload.metadata.get("version"), Some(&"0.1.0".to_string()));
}

#[test]
fn test_native_builder_full_configuration() {
    let mut env = HashMap::new();
    env.insert("VAR1".to_string(), "value1".to_string());

    let mut metadata = HashMap::new();
    metadata.insert("key1".to_string(), "val1".to_string());

    let resources = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: Some(1024),
        disk_mb: Some(5120),
        gpu_required: Some(false),
    };

    let args = vec!["--flag".to_string(), "value".to_string()];

    let workload = NativeWorkloadBuilder::new()
        .executable("/usr/local/bin/app")
        .args(args.clone())
        .working_dir("/opt/app")
        .environment(env.clone())
        .priority(JobPriority::Normal)
        .timeout(Duration::from_secs(60))
        .resources(resources.clone())
        .metadata(metadata.clone())
        .build()
        .expect("Should build with full configuration");

    match workload.workload_type {
        WorkloadType::Native {
            executable,
            args: workload_args,
            working_dir,
        } => {
            assert_eq!(executable, "/usr/local/bin/app");
            assert_eq!(workload_args, args);
            assert_eq!(working_dir, Some("/opt/app".to_string()));
        }
        _ => panic!("Expected Native workload type"),
    }

    assert_eq!(workload.runtime_hint, Some("native".to_string()));
    assert_eq!(workload.priority, Some(JobPriority::Normal));
    assert_eq!(workload.timeout, Some(Duration::from_secs(60)));
    assert_eq!(workload.resources, Some(resources));
    assert_eq!(
        workload.environment.get("VAR1"),
        Some(&"value1".to_string())
    );
    assert_eq!(workload.metadata.get("key1"), Some(&"val1".to_string()));
}

#[test]
fn test_native_builder_missing_executable_error() {
    let result = NativeWorkloadBuilder::new().build();

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_string = error.to_string();
    assert!(error_string.contains("Executable path is required"));
}

#[test]
fn test_native_builder_chaining() {
    // Test that builder methods can be chained fluently
    let _workload = NativeWorkloadBuilder::new()
        .executable("/bin/ls")
        .args(vec!["-la".to_string()])
        .working_dir("/tmp")
        .priority(JobPriority::Low)
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Chained builder should work");
}

#[test]
fn test_native_builder_empty_args() {
    let workload = NativeWorkloadBuilder::new()
        .executable("/bin/true")
        .args(vec![])
        .build()
        .expect("Should build with empty args");

    match workload.workload_type {
        WorkloadType::Native {
            executable: _,
            args,
            working_dir: _,
        } => {
            assert!(args.is_empty());
        }
        _ => panic!("Expected Native workload type"),
    }
}

#[test]
fn test_native_builder_empty_environment() {
    let workload = NativeWorkloadBuilder::new()
        .executable("/bin/env")
        .environment(HashMap::new())
        .build()
        .expect("Should build with empty environment");

    assert!(workload.environment.is_empty());
}

#[test]
fn test_native_builder_empty_metadata() {
    let workload = NativeWorkloadBuilder::new()
        .executable("/bin/echo")
        .metadata(HashMap::new())
        .build()
        .expect("Should build with empty metadata");

    assert!(workload.metadata.is_empty());
}

#[test]
fn test_native_builder_all_priorities() {
    let priorities = vec![
        JobPriority::Low,
        JobPriority::Normal,
        JobPriority::High,
        JobPriority::Critical,
    ];

    for priority in priorities {
        let workload = NativeWorkloadBuilder::new()
            .executable("/bin/echo")
            .priority(priority)
            .build()
            .expect("Should build with any priority");

        assert_eq!(workload.priority, Some(priority));
    }
}

#[test]
fn test_native_builder_zero_timeout() {
    let workload = NativeWorkloadBuilder::new()
        .executable("/bin/instant")
        .timeout(Duration::from_secs(0))
        .build()
        .expect("Should build with zero timeout");

    assert_eq!(workload.timeout, Some(Duration::from_secs(0)));
}

#[test]
fn test_native_builder_very_long_timeout() {
    let long_timeout = Duration::from_secs(86400); // 1 day

    let workload = NativeWorkloadBuilder::new()
        .executable("/bin/longrunning")
        .timeout(long_timeout)
        .build()
        .expect("Should build with long timeout");

    assert_eq!(workload.timeout, Some(long_timeout));
}

// ============================================================================
// WasmWorkloadBuilder Tests
// ============================================================================

#[test]
fn test_wasm_builder_new() {
    let builder = WasmWorkloadBuilder::new();

    // Builder should be created successfully
    let module_data = vec![0x00, 0x61, 0x73, 0x6D]; // WASM magic number
    let _workload = builder.module_data(module_data).build();
}

#[test]
fn test_wasm_builder_default() {
    let builder = WasmWorkloadBuilder::default();

    let module_data = vec![0x00, 0x61, 0x73, 0x6D];
    let _workload = builder.module_data(module_data).build();
}

#[test]
fn test_wasm_builder_minimal_config() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

    let workload = WasmWorkloadBuilder::new()
        .module_data(module_data.clone())
        .build()
        .expect("Failed to build workload");

    match workload.workload_type {
        WorkloadType::Wasm {
            module_data: wasm_data,
            args,
        } => {
            assert_eq!(wasm_data, module_data);
            assert!(args.is_empty());
        }
        _ => panic!("Expected Wasm workload type"),
    }

    assert_eq!(workload.runtime_hint, Some("wasm".to_string()));
}

#[test]
fn test_wasm_builder_with_args() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6D];
    let args = vec!["arg1".to_string(), "arg2".to_string()];

    let workload = WasmWorkloadBuilder::new()
        .module_data(module_data)
        .args(args.clone())
        .build()
        .expect("Should build wasm workload");

    match workload.workload_type {
        WorkloadType::Wasm {
            module_data: _,
            args: wasm_args,
        } => {
            assert_eq!(wasm_args, args);
        }
        _ => panic!("Expected Wasm workload type"),
    }
}

#[test]
fn test_wasm_builder_with_environment() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6D];
    let mut env = HashMap::new();
    env.insert("KEY1".to_string(), "value1".to_string());
    env.insert("KEY2".to_string(), "value2".to_string());

    let workload = WasmWorkloadBuilder::new()
        .module_data(module_data)
        .environment(env.clone())
        .build()
        .expect("Should build wasm workload");

    assert_eq!(
        workload.environment.get("KEY1"),
        Some(&"value1".to_string())
    );
    assert_eq!(
        workload.environment.get("KEY2"),
        Some(&"value2".to_string())
    );
}

#[test]
fn test_wasm_builder_with_priority() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6D];

    let workload = WasmWorkloadBuilder::new()
        .module_data(module_data)
        .priority(JobPriority::High)
        .build()
        .expect("Should build wasm workload");

    assert_eq!(workload.priority, Some(JobPriority::High));
}

#[test]
fn test_wasm_builder_with_timeout() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6D];
    let timeout = Duration::from_secs(120);

    let workload = WasmWorkloadBuilder::new()
        .module_data(module_data)
        .timeout(timeout)
        .build()
        .expect("Should build wasm workload");

    assert_eq!(workload.timeout, Some(timeout));
}

#[test]
fn test_wasm_builder_with_resources() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6D];
    let resources = ResourceRequirements {
        cpu_cores: Some(1),
        memory_mb: Some(512),
        disk_mb: Some(2048),
        gpu_required: Some(false),
    };

    let workload = WasmWorkloadBuilder::new()
        .module_data(module_data)
        .resources(resources.clone())
        .build()
        .expect("Should build wasm workload");

    assert_eq!(workload.resources, Some(resources));
}

#[test]
fn test_wasm_builder_with_metadata() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6D];
    let mut metadata = HashMap::new();
    metadata.insert("author".to_string(), "toadstool".to_string());
    metadata.insert("version".to_string(), "1.0.0".to_string());

    let workload = WasmWorkloadBuilder::new()
        .module_data(module_data)
        .metadata(metadata.clone())
        .build()
        .expect("Should build wasm workload");

    assert_eq!(
        workload.metadata.get("author"),
        Some(&"toadstool".to_string())
    );
    assert_eq!(workload.metadata.get("version"), Some(&"1.0.0".to_string()));
}

#[test]
fn test_wasm_builder_full_configuration() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
    let args = vec!["--wasm-flag".to_string()];

    let mut env = HashMap::new();
    env.insert("WASM_VAR".to_string(), "wasm_value".to_string());

    let mut metadata = HashMap::new();
    metadata.insert("module".to_string(), "test".to_string());

    let resources = ResourceRequirements {
        cpu_cores: Some(1),
        memory_mb: Some(256),
        disk_mb: Some(1024),
        gpu_required: Some(false),
    };

    let workload = WasmWorkloadBuilder::new()
        .module_data(module_data.clone())
        .args(args.clone())
        .environment(env.clone())
        .priority(JobPriority::Normal)
        .timeout(Duration::from_secs(30))
        .resources(resources.clone())
        .metadata(metadata.clone())
        .build()
        .expect("Should build wasm workload");

    match workload.workload_type {
        WorkloadType::Wasm {
            module_data: wasm_data,
            args: wasm_args,
        } => {
            assert_eq!(wasm_data, module_data);
            assert_eq!(wasm_args, args);
        }
        _ => panic!("Expected Wasm workload type"),
    }

    assert_eq!(workload.runtime_hint, Some("wasm".to_string()));
    assert_eq!(workload.priority, Some(JobPriority::Normal));
    assert_eq!(workload.timeout, Some(Duration::from_secs(30)));
    assert_eq!(workload.resources, Some(resources));
    assert_eq!(
        workload.environment.get("WASM_VAR"),
        Some(&"wasm_value".to_string())
    );
    assert_eq!(workload.metadata.get("module"), Some(&"test".to_string()));
}

#[test]
fn test_wasm_builder_missing_module_data() {
    let result = WasmWorkloadBuilder::new().build();
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Module data is required"));
}

#[test]
fn test_wasm_builder_chaining() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6D];

    let _workload = WasmWorkloadBuilder::new()
        .module_data(module_data)
        .args(vec!["arg".to_string()])
        .priority(JobPriority::Low)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Should build wasm workload");
}

#[test]
fn test_wasm_builder_empty_module_data() {
    let workload = WasmWorkloadBuilder::new()
        .module_data(vec![])
        .build()
        .expect("Should build wasm workload");

    match workload.workload_type {
        WorkloadType::Wasm {
            module_data,
            args: _,
        } => {
            assert!(module_data.is_empty());
        }
        _ => panic!("Expected Wasm workload type"),
    }
}

#[test]
fn test_wasm_builder_large_module_data() {
    let large_module = vec![0u8; 1024 * 1024]; // 1 MB

    let workload = WasmWorkloadBuilder::new()
        .module_data(large_module.clone())
        .build()
        .expect("Should build wasm workload");

    match workload.workload_type {
        WorkloadType::Wasm {
            module_data,
            args: _,
        } => {
            assert_eq!(module_data.len(), large_module.len());
        }
        _ => panic!("Expected Wasm workload type"),
    }
}

#[test]
fn test_wasm_builder_all_priorities() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6D];
    let priorities = vec![
        JobPriority::Low,
        JobPriority::Normal,
        JobPriority::High,
        JobPriority::Critical,
    ];

    for priority in priorities {
        let workload = WasmWorkloadBuilder::new()
            .module_data(module_data.clone())
            .priority(priority)
            .build()
            .expect("Should build wasm workload");

        assert_eq!(workload.priority, Some(priority));
    }
}

#[test]
fn test_wasm_builder_empty_args() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6D];

    let workload = WasmWorkloadBuilder::new()
        .module_data(module_data)
        .args(vec![])
        .build()
        .expect("Should build wasm workload");

    match workload.workload_type {
        WorkloadType::Wasm {
            module_data: _,
            args,
        } => {
            assert!(args.is_empty());
        }
        _ => panic!("Expected Wasm workload type"),
    }
}

#[test]
fn test_wasm_builder_empty_environment() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6D];

    let workload = WasmWorkloadBuilder::new()
        .module_data(module_data)
        .environment(HashMap::new())
        .build()
        .expect("Should build wasm workload");

    assert!(workload.environment.is_empty());
}

#[test]
fn test_wasm_builder_empty_metadata() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6D];

    let workload = WasmWorkloadBuilder::new()
        .module_data(module_data)
        .metadata(HashMap::new())
        .build()
        .expect("Should build wasm workload");

    assert!(workload.metadata.is_empty());
}

// ============================================================================
// Comparison Tests
// ============================================================================

#[test]
fn test_native_vs_wasm_runtime_hints() {
    let native = NativeWorkloadBuilder::new()
        .executable("/bin/echo")
        .build()
        .expect("Should build native");

    let wasm = WasmWorkloadBuilder::new()
        .module_data(vec![0x00, 0x61, 0x73, 0x6D])
        .build()
        .expect("Should build wasm workload");

    assert_eq!(native.runtime_hint, Some("native".to_string()));
    assert_eq!(wasm.runtime_hint, Some("wasm".to_string()));
}

#[test]
fn test_native_vs_wasm_workload_types() {
    let native = NativeWorkloadBuilder::new()
        .executable("/bin/test")
        .build()
        .expect("Should build native");

    let wasm = WasmWorkloadBuilder::new()
        .module_data(vec![0x00])
        .build()
        .expect("Should build wasm workload");

    assert!(matches!(native.workload_type, WorkloadType::Native { .. }));
    assert!(matches!(wasm.workload_type, WorkloadType::Wasm { .. }));
}
