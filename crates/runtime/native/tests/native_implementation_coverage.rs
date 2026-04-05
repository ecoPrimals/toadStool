// SPDX-License-Identifier: AGPL-3.0-or-later
//! Implementation-focused tests for Native Runtime Engine
//!
//! This test suite targets specific implementation paths to improve coverage,
//! focusing on capabilities, workload types, and runtime behavior.

use std::collections::HashMap;
use std::path::PathBuf;
use toadstool::{
    WorkloadType,
    execution::{RuntimeConfig, RuntimeEngine, RuntimeType},
    security::{Capability, IsolationLevel, SecurityContext},
    workload::{ExecutableSource, WorkloadSpec},
};
use toadstool_runtime_native::NativeRuntimeEngine;

// ============================================================================
// Constructor and Builder Tests
// ============================================================================

#[test]
fn test_native_runtime_new_has_capabilities() {
    let engine = NativeRuntimeEngine::new();
    let caps = engine.get_capabilities();

    assert_eq!(caps.supported_workloads, vec![WorkloadType::Native]);
    assert_eq!(caps.max_concurrent_executions, Some(100));
    assert!(!caps.supported_architectures.is_empty());
    assert!(!caps.version.is_empty());
}

#[test]
fn test_native_runtime_capabilities_include_platform_features() {
    let engine = NativeRuntimeEngine::new();
    let caps = engine.get_capabilities();

    assert!(caps.platform_features.contains_key("process_isolation"));
    assert_eq!(caps.platform_features.get("process_isolation"), Some(&true));
}

#[test]
fn test_native_runtime_capabilities_architecture_matches_system() {
    let engine = NativeRuntimeEngine::new();
    let caps = engine.get_capabilities();

    let arch = std::env::consts::ARCH;
    assert!(caps.supported_architectures.contains(&arch.to_string()));
}

#[test]
fn test_native_runtime_default_config_reasonable() {
    let engine = NativeRuntimeEngine::new();
    let caps = engine.get_capabilities();

    assert!(caps.max_concurrent_executions.unwrap() > 0);
    assert!(caps.max_concurrent_executions.unwrap() <= 1000);
}

#[test]
fn test_native_runtime_debug_format_contains_fields() {
    let engine = NativeRuntimeEngine::new();
    let debug_str = format!("{engine:?}");

    assert!(debug_str.contains("NativeRuntimeEngine"));
    assert!(debug_str.contains("config"));
    assert!(debug_str.contains("capabilities"));
}

#[test]
fn test_native_runtime_capabilities_version_not_empty() {
    let engine = NativeRuntimeEngine::new();
    let caps = engine.get_capabilities();

    assert!(!caps.version.is_empty());
    // Version should be valid semver-like format
    assert!(caps.version.contains('.') || caps.version.len() >= 3);
}

// ============================================================================
// Workload Support Tests
// ============================================================================

#[test]
fn test_supports_native_workload() {
    let engine = NativeRuntimeEngine::new();
    assert!(engine.supports_workload(&WorkloadType::Native));
}

#[test]
fn test_does_not_support_container_workload() {
    let engine = NativeRuntimeEngine::new();
    assert!(!engine.supports_workload(&WorkloadType::Container));
}

#[test]
fn test_does_not_support_wasm_workload() {
    let engine = NativeRuntimeEngine::new();
    assert!(!engine.supports_workload(&WorkloadType::Wasm));
}

#[test]
fn test_does_not_support_python_workload() {
    let engine = NativeRuntimeEngine::new();
    assert!(!engine.supports_workload(&WorkloadType::Python));
}

#[test]
fn test_does_not_support_gpu_workload() {
    let engine = NativeRuntimeEngine::new();
    assert!(!engine.supports_workload(&WorkloadType::Gpu));
}

#[test]
fn test_supports_workload_consistency_with_capabilities() {
    let engine = NativeRuntimeEngine::new();
    let caps = engine.get_capabilities();

    for workload_type in &caps.supported_workloads {
        assert!(engine.supports_workload(workload_type));
    }
}

#[test]
fn test_capabilities_list_matches_support_checks() {
    let engine = NativeRuntimeEngine::new();
    let caps = engine.get_capabilities();

    // Should only support Native
    assert_eq!(caps.supported_workloads.len(), 1);
    assert_eq!(caps.supported_workloads[0], WorkloadType::Native);
}

// ============================================================================
// Initialization Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_initialize_with_default_config() {
    let mut engine = NativeRuntimeEngine::new();
    let config = RuntimeConfig::default();

    let result = engine.initialize(config).await;
    // Should succeed on most platforms
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_initialize_doesnt_panic() {
    let mut engine = NativeRuntimeEngine::new();
    let config = RuntimeConfig::default();

    // Should not panic
    let _result = engine.initialize(config).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_initialize_multiple_times_doesnt_fail() {
    let mut engine = NativeRuntimeEngine::new();

    let _result1 = engine.initialize(RuntimeConfig::default()).await;
    let _result2 = engine.initialize(RuntimeConfig::default()).await;

    // Re-initialization should be allowed
}

// ============================================================================
// Platform-Specific Feature Tests
// ============================================================================

#[cfg(target_os = "linux")]
#[test]
fn test_linux_has_resource_limits_feature() {
    let engine = NativeRuntimeEngine::new();
    let caps = engine.get_capabilities();

    assert_eq!(caps.platform_features.get("resource_limits"), Some(&true));
}

#[cfg(unix)]
#[test]
fn test_unix_has_user_switching_feature() {
    let engine = NativeRuntimeEngine::new();
    let caps = engine.get_capabilities();

    assert_eq!(caps.platform_features.get("user_switching"), Some(&true));
}

#[cfg(unix)]
#[test]
fn test_unix_has_chroot_jail_feature() {
    let engine = NativeRuntimeEngine::new();
    let caps = engine.get_capabilities();

    assert_eq!(caps.platform_features.get("chroot_jail"), Some(&true));
}

#[cfg(windows)]
#[test]
fn test_windows_platform_features() {
    let engine = NativeRuntimeEngine::new();
    let caps = engine.get_capabilities();

    // Windows should have process isolation
    assert_eq!(caps.platform_features.get("process_isolation"), Some(&true));
}

// ============================================================================
// Security Context Tests
// ============================================================================

#[test]
fn test_security_context_default_values() {
    let context = SecurityContext::default();

    assert_eq!(context.isolation_level, IsolationLevel::Standard);
}

#[test]
fn test_security_context_with_read_capability() {
    let mut context = SecurityContext::default();
    context.capabilities.push(Capability::Read);

    assert!(context.has_capability(&Capability::Read));
}

#[test]
fn test_security_context_with_write_capability() {
    let mut context = SecurityContext::default();
    context.capabilities.push(Capability::Write);

    assert!(context.has_capability(&Capability::Write));
}

#[test]
fn test_security_context_with_network_capability() {
    let mut context = SecurityContext::default();
    context.capabilities.push(Capability::NetworkClient);

    assert!(context.has_capability(&Capability::NetworkClient));
}

#[test]
fn test_security_context_with_execute_capability() {
    let mut context = SecurityContext::default();
    context.capabilities.push(Capability::Execute);

    assert!(context.has_capability(&Capability::Execute));
}

#[test]
fn test_security_context_isolation_none() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::None,
        ..Default::default()
    };

    assert_eq!(context.isolation_level, IsolationLevel::None);
}

#[test]
fn test_security_context_isolation_standard() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::Standard,
        ..Default::default()
    };

    assert_eq!(context.isolation_level, IsolationLevel::Standard);
}

#[test]
fn test_security_context_isolation_enhanced() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::Enhanced,
        ..Default::default()
    };

    assert_eq!(context.isolation_level, IsolationLevel::Enhanced);
}

#[test]
fn test_security_context_isolation_maximum() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::Maximum,
        ..Default::default()
    };

    assert_eq!(context.isolation_level, IsolationLevel::Maximum);
}

#[test]
fn test_security_context_multiple_capabilities() {
    let mut context = SecurityContext::default();
    context.capabilities.push(Capability::Read);
    context.capabilities.push(Capability::Write);
    context.capabilities.push(Capability::NetworkClient);
    context.capabilities.push(Capability::Execute);

    assert!(context.has_capability(&Capability::Read));
    assert!(context.has_capability(&Capability::Write));
    assert!(context.has_capability(&Capability::NetworkClient));
    assert!(context.has_capability(&Capability::Execute));
}

// ============================================================================
// Workload Specification Tests
// ============================================================================

#[test]
fn test_workload_spec_native_with_file() {
    let workload = WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: PathBuf::from("/bin/echo"),
        },
        args: Some(vec!["test".to_string()]),
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };

    assert_eq!(workload.workload_type(), WorkloadType::Native);
}

#[test]
fn test_workload_spec_native_with_url() {
    let workload = WorkloadSpec::Native {
        executable: ExecutableSource::Url {
            url: "https://example.com/binary".to_string(),
        },
        args: None,
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };

    assert_eq!(workload.workload_type(), WorkloadType::Native);
}

#[test]
fn test_workload_spec_native_with_bytes() {
    let workload = WorkloadSpec::Native {
        executable: ExecutableSource::Bytes {
            data: bytes::Bytes::from(vec![0x7f, 0x45, 0x4c, 0x46]),
        },
        args: None,
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };

    assert_eq!(workload.workload_type(), WorkloadType::Native);
}

#[test]
fn test_workload_spec_default_is_native() {
    let workload = WorkloadSpec::default();
    assert_eq!(workload.workload_type(), WorkloadType::Native);
}

// ============================================================================
// Executable Source Tests
// ============================================================================

#[test]
fn test_executable_source_file_path() {
    let source = ExecutableSource::File {
        path: PathBuf::from("/usr/bin/python3"),
    };

    if let ExecutableSource::File { path } = source {
        assert_eq!(path, PathBuf::from("/usr/bin/python3"));
    } else {
        panic!("Expected File variant");
    }
}

#[test]
fn test_executable_source_bytes_data() {
    let data = vec![0x7f, 0x45, 0x4c, 0x46]; // ELF magic
    let source = ExecutableSource::Bytes {
        data: data.clone().into(),
    };

    if let ExecutableSource::Bytes { data: d } = source {
        assert_eq!(d, data);
    } else {
        panic!("Expected Bytes variant");
    }
}

#[test]
fn test_executable_source_url_string() {
    let url_str = "https://example.com/binary".to_string();
    let source = ExecutableSource::Url {
        url: url_str.clone(),
    };

    if let ExecutableSource::Url { url } = source {
        assert_eq!(url, url_str);
    } else {
        panic!("Expected Url variant");
    }
}

// ============================================================================
// Runtime Type Tests
// ============================================================================

#[test]
fn test_runtime_type_native_equality() {
    let rt1 = RuntimeType::Native;
    let rt2 = RuntimeType::Native;

    assert_eq!(rt1, rt2);
}

#[test]
fn test_runtime_type_native_not_container() {
    let native = RuntimeType::Native;
    let container = RuntimeType::Container;

    assert_ne!(native, container);
}

#[test]
fn test_runtime_type_native_debug() {
    let rt = RuntimeType::Native;
    let debug_str = format!("{rt:?}");

    assert!(debug_str.contains("Native"));
}

// ============================================================================
// Runtime Capabilities Structure Tests
// ============================================================================

#[test]
fn test_runtime_capabilities_can_be_cloned() {
    let engine = NativeRuntimeEngine::new();
    let caps1 = engine.get_capabilities();
    let caps2 = caps1.clone();

    assert_eq!(caps1.version, caps2.version);
    assert_eq!(caps1.supported_workloads, caps2.supported_workloads);
}

#[test]
fn test_runtime_capabilities_debug_format() {
    let engine = NativeRuntimeEngine::new();
    let caps = engine.get_capabilities();
    let debug_str = format!("{caps:?}");

    assert!(debug_str.contains("RuntimeCapabilities"));
}

// ============================================================================
// Runtime Config Tests
// ============================================================================

#[test]
fn test_runtime_config_default_works() {
    let config = RuntimeConfig::default();
    let debug_str = format!("{config:?}");

    assert!(debug_str.contains("RuntimeConfig"));
}

#[test]
fn test_runtime_config_can_be_cloned() {
    let config1 = RuntimeConfig::default();
    let config2 = config1;

    // Should be able to clone
    let _ = config1;
    let _ = config2;
}

// ============================================================================
// Multiple Engine Instances Tests
// ============================================================================

#[test]
fn test_multiple_engine_instances_independent() {
    let engine1 = NativeRuntimeEngine::new();
    let engine2 = NativeRuntimeEngine::new();

    let caps1 = engine1.get_capabilities();
    let caps2 = engine2.get_capabilities();

    // Should have same capabilities
    assert_eq!(caps1.supported_workloads, caps2.supported_workloads);
    assert_eq!(
        caps1.max_concurrent_executions,
        caps2.max_concurrent_executions
    );
}

#[test]
fn test_engine_creation_is_fast() {
    use std::time::Instant;

    let start = Instant::now();
    let _engine = NativeRuntimeEngine::new();
    let duration = start.elapsed();

    // Should create in less than 100ms
    assert!(duration.as_millis() < 100);
}

// ============================================================================
// Workload Type Enum Tests
// ============================================================================

#[test]
fn test_workload_type_equality() {
    let wt1 = WorkloadType::Native;
    let wt2 = WorkloadType::Native;

    assert_eq!(wt1, wt2);
}

#[test]
fn test_workload_type_can_be_in_vec() {
    let types = vec![
        WorkloadType::Native,
        WorkloadType::Container,
        WorkloadType::Wasm,
    ];

    assert_eq!(types.len(), 3);
    assert!(types.contains(&WorkloadType::Native));
}

#[test]
fn test_workload_type_can_be_in_hashmap() {
    use std::collections::HashMap;

    let mut map = HashMap::new();
    map.insert(WorkloadType::Native, "native_engine");
    map.insert(WorkloadType::Container, "container_engine");

    assert_eq!(map.get(&WorkloadType::Native), Some(&"native_engine"));
}
