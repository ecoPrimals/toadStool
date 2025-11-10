//! Comprehensive tests for CLI Executor Workload types
//!
//! Week 17 Sprint 4: CLI Executor workload specification tests
//! Target: ~30 tests

use std::collections::HashMap;
use toadstool_cli::executor::workload::*;

// ============================================================================
// WorkloadMetadata Tests (8 tests)
// ============================================================================

#[test]
fn test_workload_metadata_full() {
    let metadata = WorkloadMetadata {
        name: "test-workload".to_string(),
        description: Some("Test workload description".to_string()),
        version: Some("1.0.0".to_string()),
    };

    assert_eq!(metadata.name, "test-workload");
    assert_eq!(
        metadata.description,
        Some("Test workload description".to_string())
    );
    assert_eq!(metadata.version, Some("1.0.0".to_string()));
}

#[test]
fn test_workload_metadata_minimal() {
    let metadata = WorkloadMetadata {
        name: "minimal-workload".to_string(),
        description: None,
        version: None,
    };

    assert_eq!(metadata.name, "minimal-workload");
    assert!(metadata.description.is_none());
    assert!(metadata.version.is_none());
}

#[test]
fn test_workload_metadata_clone() {
    let metadata1 = WorkloadMetadata {
        name: "test".to_string(),
        description: Some("desc".to_string()),
        version: Some("1.0".to_string()),
    };

    let metadata2 = metadata1.clone();
    assert_eq!(metadata1.name, metadata2.name);
    assert_eq!(metadata1.description, metadata2.description);
}

#[test]
fn test_workload_metadata_debug() {
    let metadata = WorkloadMetadata {
        name: "test".to_string(),
        description: Some("desc".to_string()),
        version: Some("1.0".to_string()),
    };

    let debug_str = format!("{:?}", metadata);
    assert!(debug_str.contains("WorkloadMetadata"));
    assert!(debug_str.contains("test"));
}

#[test]
fn test_workload_metadata_different_names() {
    let names = vec![
        "simple",
        "with-dashes",
        "with_underscores",
        "CamelCase",
        "mixedCase123",
    ];

    for name in names {
        let metadata = WorkloadMetadata {
            name: name.to_string(),
            description: None,
            version: None,
        };
        assert_eq!(metadata.name, name);
    }
}

#[test]
fn test_workload_metadata_version_formats() {
    let versions = vec!["1.0.0", "2.1.3", "0.1.0-alpha", "1.0.0-beta.1"];

    for version in versions {
        let metadata = WorkloadMetadata {
            name: "test".to_string(),
            description: None,
            version: Some(version.to_string()),
        };
        assert_eq!(metadata.version, Some(version.to_string()));
    }
}

#[test]
fn test_workload_metadata_long_description() {
    let long_desc = "This is a very long description that describes the workload in great detail with many words and explanations.".to_string();

    let metadata = WorkloadMetadata {
        name: "test".to_string(),
        description: Some(long_desc.clone()),
        version: Some("1.0.0".to_string()),
    };

    assert_eq!(metadata.description, Some(long_desc));
}

#[test]
fn test_workload_metadata_empty_strings() {
    let metadata = WorkloadMetadata {
        name: "".to_string(),
        description: Some("".to_string()),
        version: Some("".to_string()),
    };

    assert_eq!(metadata.name, "");
    assert_eq!(metadata.description, Some("".to_string()));
}

// ============================================================================
// ExecutionSpec Tests (10 tests)
// ============================================================================

#[test]
fn test_execution_spec_native_basic() {
    let spec = ExecutionSpec::Native {
        command: "/bin/echo".to_string(),
        args: Some(vec!["hello".to_string(), "world".to_string()]),
        working_dir: None,
        env: None,
    };

    match spec {
        ExecutionSpec::Native { command, args, .. } => {
            assert_eq!(command, "/bin/echo");
            assert_eq!(args.unwrap().len(), 2);
        }
        _ => panic!("Should be Native variant"),
    }
}

#[test]
fn test_execution_spec_native_with_env() {
    let mut env = HashMap::new();
    env.insert("KEY1".to_string(), "value1".to_string());
    env.insert("KEY2".to_string(), "value2".to_string());

    let spec = ExecutionSpec::Native {
        command: "/bin/sh".to_string(),
        args: None,
        working_dir: Some("/tmp".to_string()),
        env: Some(env.clone()),
    };

    match spec {
        ExecutionSpec::Native { env: e, .. } => {
            assert_eq!(e.unwrap().len(), 2);
        }
        _ => panic!("Should be Native variant"),
    }
}

#[test]
fn test_execution_spec_python_script() {
    let spec = ExecutionSpec::Python {
        script: Some("print('hello')".to_string()),
        file: None,
        args: None,
        env: None,
    };

    match spec {
        ExecutionSpec::Python { script, .. } => {
            assert!(script.is_some());
        }
        _ => panic!("Should be Python variant"),
    }
}

#[test]
fn test_execution_spec_python_file() {
    let spec = ExecutionSpec::Python {
        script: None,
        file: Some("/path/to/script.py".to_string()),
        args: Some(vec!["arg1".to_string()]),
        env: None,
    };

    match spec {
        ExecutionSpec::Python { file, args, .. } => {
            assert_eq!(file.unwrap(), "/path/to/script.py");
            assert_eq!(args.unwrap().len(), 1);
        }
        _ => panic!("Should be Python variant"),
    }
}

#[test]
fn test_execution_spec_wasm() {
    let spec = ExecutionSpec::Wasm {
        module: "module.wasm".to_string(),
        args: Some(vec!["--flag".to_string()]),
        env: None,
    };

    match spec {
        ExecutionSpec::Wasm { module, args, .. } => {
            assert_eq!(module, "module.wasm");
            assert_eq!(args.unwrap().len(), 1);
        }
        _ => panic!("Should be Wasm variant"),
    }
}

#[test]
fn test_execution_spec_container() {
    let spec = ExecutionSpec::Container {
        image: "ubuntu:latest".to_string(),
        command: Some(vec!["/bin/bash".to_string()]),
        args: Some(vec!["-c".to_string(), "echo hello".to_string()]),
        env: None,
    };

    match spec {
        ExecutionSpec::Container {
            image,
            command,
            args,
            ..
        } => {
            assert_eq!(image, "ubuntu:latest");
            assert_eq!(command.unwrap().len(), 1);
            assert_eq!(args.unwrap().len(), 2);
        }
        _ => panic!("Should be Container variant"),
    }
}

#[test]
fn test_execution_spec_clone() {
    let spec1 = ExecutionSpec::Native {
        command: "test".to_string(),
        args: None,
        working_dir: None,
        env: None,
    };

    let spec2 = spec1.clone();
    match (&spec1, &spec2) {
        (ExecutionSpec::Native { command: c1, .. }, ExecutionSpec::Native { command: c2, .. }) => {
            assert_eq!(c1, c2);
        }
        _ => panic!("Both should be Native"),
    }
}

#[test]
fn test_execution_spec_debug() {
    let specs = vec![
        ExecutionSpec::Native {
            command: "test".to_string(),
            args: None,
            working_dir: None,
            env: None,
        },
        ExecutionSpec::Python {
            script: Some("test".to_string()),
            file: None,
            args: None,
            env: None,
        },
        ExecutionSpec::Wasm {
            module: "test.wasm".to_string(),
            args: None,
            env: None,
        },
    ];

    for spec in specs {
        let debug_str = format!("{:?}", spec);
        assert!(!debug_str.is_empty());
    }
}

#[test]
fn test_execution_spec_all_variants() {
    let specs = vec![
        ExecutionSpec::Native {
            command: "cmd".to_string(),
            args: None,
            working_dir: None,
            env: None,
        },
        ExecutionSpec::Python {
            script: Some("code".to_string()),
            file: None,
            args: None,
            env: None,
        },
        ExecutionSpec::Wasm {
            module: "mod.wasm".to_string(),
            args: None,
            env: None,
        },
        ExecutionSpec::Container {
            image: "img".to_string(),
            command: None,
            args: None,
            env: None,
        },
    ];

    assert_eq!(specs.len(), 4);
}

#[test]
fn test_execution_spec_with_complex_env() {
    let mut env = HashMap::new();
    for i in 0..10 {
        env.insert(format!("KEY{}", i), format!("value{}", i));
    }

    let spec = ExecutionSpec::Native {
        command: "test".to_string(),
        args: None,
        working_dir: None,
        env: Some(env.clone()),
    };

    match spec {
        ExecutionSpec::Native { env: e, .. } => {
            assert_eq!(e.unwrap().len(), 10);
        }
        _ => panic!("Should be Native"),
    }
}

// ============================================================================
// ResourceSpec Tests (6 tests)
// ============================================================================

#[test]
fn test_resource_spec_full() {
    let spec = ResourceSpec {
        cpu_cores: Some(4.0),
        memory_mb: Some(8192),
        disk_mb: Some(10240),
        gpu: Some(true),
    };

    assert_eq!(spec.cpu_cores, Some(4.0));
    assert_eq!(spec.memory_mb, Some(8192));
    assert_eq!(spec.disk_mb, Some(10240));
    assert_eq!(spec.gpu, Some(true));
}

#[test]
fn test_resource_spec_minimal() {
    let spec = ResourceSpec {
        cpu_cores: None,
        memory_mb: None,
        disk_mb: None,
        gpu: None,
    };

    assert!(spec.cpu_cores.is_none());
    assert!(spec.memory_mb.is_none());
    assert!(spec.disk_mb.is_none());
    assert!(spec.gpu.is_none());
}

#[test]
fn test_resource_spec_different_cpu_values() {
    let cpu_values = [0.5, 1.0, 2.0, 4.0, 8.0, 16.0];

    for cpu in cpu_values {
        let spec = ResourceSpec {
            cpu_cores: Some(cpu),
            memory_mb: Some(1024),
            disk_mb: None,
            gpu: None,
        };
        assert_eq!(spec.cpu_cores, Some(cpu));
    }
}

#[test]
fn test_resource_spec_different_memory_values() {
    let memory_values = [512, 1024, 2048, 4096, 8192, 16384];

    for mem in memory_values {
        let spec = ResourceSpec {
            cpu_cores: Some(2.0),
            memory_mb: Some(mem),
            disk_mb: None,
            gpu: None,
        };
        assert_eq!(spec.memory_mb, Some(mem));
    }
}

#[test]
fn test_resource_spec_with_gpu() {
    let spec = ResourceSpec {
        cpu_cores: Some(8.0),
        memory_mb: Some(16384),
        disk_mb: Some(51200),
        gpu: Some(true),
    };

    assert_eq!(spec.gpu, Some(true));
}

#[test]
fn test_resource_spec_clone_debug() {
    let spec1 = ResourceSpec {
        cpu_cores: Some(4.0),
        memory_mb: Some(8192),
        disk_mb: Some(10240),
        gpu: Some(false),
    };

    let spec2 = spec1.clone();
    assert_eq!(spec1.cpu_cores, spec2.cpu_cores);
    assert_eq!(spec1.memory_mb, spec2.memory_mb);

    let debug_str = format!("{:?}", spec1);
    assert!(debug_str.contains("ResourceSpec"));
}

// ============================================================================
// SecuritySpec Tests (6 tests)
// ============================================================================

#[test]
fn test_security_spec_with_isolation() {
    let spec = SecuritySpec {
        isolation: Some("strict".to_string()),
    };

    assert_eq!(spec.isolation, Some("strict".to_string()));
}

#[test]
fn test_security_spec_without_isolation() {
    let spec = SecuritySpec { isolation: None };

    assert!(spec.isolation.is_none());
}

#[test]
fn test_security_spec_different_isolation_levels() {
    let levels = vec!["none", "basic", "strict", "maximum"];

    for level in levels {
        let spec = SecuritySpec {
            isolation: Some(level.to_string()),
        };
        assert_eq!(spec.isolation, Some(level.to_string()));
    }
}

#[test]
fn test_security_spec_clone() {
    let spec1 = SecuritySpec {
        isolation: Some("strict".to_string()),
    };

    let spec2 = spec1.clone();
    assert_eq!(spec1.isolation, spec2.isolation);
}

#[test]
fn test_security_spec_debug() {
    let spec = SecuritySpec {
        isolation: Some("strict".to_string()),
    };

    let debug_str = format!("{:?}", spec);
    assert!(debug_str.contains("SecuritySpec"));
    assert!(debug_str.contains("isolation"));
}

#[test]
fn test_security_spec_empty_string() {
    let spec = SecuritySpec {
        isolation: Some("".to_string()),
    };

    assert_eq!(spec.isolation, Some("".to_string()));
}

// ============================================================================
// Test Coverage Summary
// ============================================================================

#[test]
fn test_executor_workload_coverage_summary() {
    println!("=== CLI Executor Workload Test Coverage ===");
    println!("WorkloadMetadata Tests:           8 tests");
    println!("ExecutionSpec Tests:             10 tests");
    println!("ResourceSpec Tests:               6 tests");
    println!("SecuritySpec Tests:               6 tests");
    println!("────────────────────────────────────────");
    println!("Total:                           30 tests");
    println!("Module Coverage:                 Expanded");
    println!("=========================================");
}
