// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU arguments, integration scenarios, and edge cases (workload expansion tests).

use std::collections::HashMap;
use std::path::PathBuf;
use toadstool::workload::{PortMapping, PortProtocol};
use toadstool::*;

// ============================================================================
// GpuArgument Tests
// ============================================================================

#[test]
fn test_gpu_argument_buffer() {
    let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
    let arg = GpuArgument::Buffer { data };

    match arg {
        GpuArgument::Buffer { data: d } => {
            assert_eq!(d.len(), 8);
            assert_eq!(d[0], 1);
        }
        _ => panic!("Expected Buffer argument"),
    }
}

#[test]
fn test_gpu_argument_buffer_large() {
    let large_data = vec![0u8; 1024 * 1024]; // 1MB buffer
    let arg = GpuArgument::Buffer { data: large_data };

    match arg {
        GpuArgument::Buffer { data: d } => {
            assert_eq!(d.len(), 1024 * 1024);
        }
        _ => panic!("Expected Buffer argument"),
    }
}

#[test]
fn test_gpu_argument_scalar_f64() {
    let arg = GpuArgument::Scalar {
        value: std::f64::consts::PI,
    };

    match arg {
        GpuArgument::Scalar { value } => {
            assert!((value - std::f64::consts::PI).abs() < 0.0001);
        }
        _ => panic!("Expected Scalar argument"),
    }
}

#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
#[test]
fn test_gpu_argument_scalar_zero() {
    let arg = GpuArgument::Scalar { value: 0.0 };

    match arg {
        GpuArgument::Scalar { value } => {
            assert_eq!(value, 0.0);
        }
        _ => panic!("Expected Scalar argument"),
    }
}

#[test]
fn test_gpu_argument_integer_positive() {
    let arg = GpuArgument::Integer { value: 42 };

    match arg {
        GpuArgument::Integer { value } => {
            assert_eq!(value, 42);
        }
        _ => panic!("Expected Integer argument"),
    }
}

#[test]
fn test_gpu_argument_integer_negative() {
    let arg = GpuArgument::Integer { value: -100 };

    match arg {
        GpuArgument::Integer { value } => {
            assert_eq!(value, -100);
        }
        _ => panic!("Expected Integer argument"),
    }
}

#[test]
fn test_gpu_argument_integer_large() {
    let arg = GpuArgument::Integer { value: i64::MAX };

    match arg {
        GpuArgument::Integer { value } => {
            assert_eq!(value, i64::MAX);
        }
        _ => panic!("Expected Integer argument"),
    }
}

#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
#[test]
fn test_gpu_argument_clone() {
    let arg1 = GpuArgument::Scalar {
        value: std::f64::consts::E,
    };
    let arg2 = arg1.clone();

    match (arg1, arg2) {
        (GpuArgument::Scalar { value: v1 }, GpuArgument::Scalar { value: v2 }) => {
            assert_eq!(v1, v2);
        }
        _ => panic!("Clone failed"),
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_workload_spec_container_full_configuration() {
    let mut env_vars = HashMap::new();
    env_vars.insert("NODE_ENV".to_string(), "production".to_string());
    env_vars.insert("PORT".to_string(), "3000".to_string());

    let volumes = vec![VolumeMount {
        source: PathBuf::from("/host/app"),
        target: PathBuf::from("/app"),
        mount_type: VolumeMountType::Bind,
        read_only: true,
    }];

    let ports = vec![WorkloadPortMapping {
        container_port: 3000,
        host_port: 3000,
        protocol: PortProtocol::Tcp,
    }];

    let auth = RegistryAuth {
        username: "user".to_string(),
        password: "password".to_string(),
        server_url: "https://registry.example.com".to_string(),
    };

    let workload = WorkloadSpec::Container {
        image: "node:18-alpine".to_string(),
        command: Some(vec!["node".to_string()]),
        args: Some(vec!["server.js".to_string()]),
        env_vars,
        working_dir: Some("/app".to_string()),
        volumes,
        ports,
        registry_auth: Some(auth),
    };

    assert!(workload.validate().is_ok());
}

#[test]
fn test_workload_spec_wasm_with_wasi_full() {
    let wasi = WasiConfig {
        inherit_env: false,
        inherit_stdio: false,
        allowed_dirs: vec![PathBuf::from("/data")],
        preopened_dirs: vec![PathBuf::from("/data")],
        args: vec!["--config".to_string(), "config.toml".to_string()],
    };

    let mut env_vars = HashMap::new();
    env_vars.insert("LOG_LEVEL".to_string(), "debug".to_string());

    let workload = WorkloadSpec::Wasm {
        module: WasmModuleSource::Bytes {
            data: bytes::Bytes::from(vec![0x00, 0x61, 0x73, 0x6d, 1, 0, 0, 0]),
        },
        args: Some(vec!["main".to_string()]),
        wasi_config: Some(wasi),
        env_vars,
    };

    assert!(workload.validate().is_ok());
}

#[test]
fn test_workload_spec_gpu_with_multiple_arguments() {
    let args = vec![
        GpuArgument::Buffer {
            data: vec![1u8, 2, 3, 4],
        },
        GpuArgument::Integer { value: 256 },
        GpuArgument::Buffer {
            data: vec![0u8; 1024],
        },
    ];

    let workload = WorkloadSpec::Gpu {
        program: GpuProgramSource::Cuda {
            source: "__global__ void kernel() {}".to_string(),
        },
        kernel_name: "kernel".to_string(),
        work_group_size: Some((256, 1, 1)),
        global_work_size: (1024, 1, 1),
        args,
    };

    match workload {
        WorkloadSpec::Gpu { args, .. } => {
            assert_eq!(args.len(), 3);
        }
        _ => panic!("Expected GPU workload"),
    }
}

#[test]
fn test_workload_spec_serialization_roundtrip() {
    let original = WorkloadSpec::Container {
        image: "alpine:latest".to_string(),
        command: None,
        args: None,
        env_vars: HashMap::new(),
        working_dir: None,
        volumes: vec![],
        ports: vec![],
        registry_auth: None,
    };

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: WorkloadSpec = serde_json::from_str(&serialized).unwrap();

    match deserialized {
        WorkloadSpec::Container { image, .. } => {
            assert_eq!(image, "alpine:latest");
        }
        _ => panic!("Deserialization failed"),
    }
}

#[test]
fn test_workload_type_hash() {
    use std::collections::HashSet;

    let mut types = HashSet::new();
    types.insert(WorkloadType::Native);
    types.insert(WorkloadType::Wasm);
    types.insert(WorkloadType::Container);

    assert!(types.contains(&WorkloadType::Native));
    assert!(types.contains(&WorkloadType::Wasm));
    assert_eq!(types.len(), 3);
}

#[test]
fn test_multiple_workload_specs_collection() {
    let workloads = vec![
        WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: PathBuf::from("/bin/ls"),
            },
            args: None,
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        },
        WorkloadSpec::Container {
            image: "nginx".to_string(),
            command: None,
            args: None,
            env_vars: HashMap::new(),
            working_dir: None,
            volumes: vec![],
            ports: vec![],
            registry_auth: None,
        },
        WorkloadSpec::Python {
            source: PythonSource::Code {
                code: "print('test')".to_string(),
            },
            python_version: None,
            requirements: vec![],
            env_vars: HashMap::new(),
        },
    ];

    assert_eq!(workloads.len(), 3);
    assert_eq!(workloads[0].workload_type(), WorkloadType::Native);
    assert_eq!(workloads[1].workload_type(), WorkloadType::Container);
    assert_eq!(workloads[2].workload_type(), WorkloadType::Python);
}

// ============================================================================
// Edge Case and Error Scenarios
// ============================================================================

#[test]
fn test_executable_source_empty_url() {
    let source = ExecutableSource::Url { url: String::new() };

    match source {
        ExecutableSource::Url { url } => {
            assert!(url.is_empty());
        }
        _ => panic!("Expected Url source"),
    }
}

#[test]
fn test_executable_source_large_bytes() {
    let large_data = vec![0u8; 1024 * 1024]; // 1MB
    let source = ExecutableSource::Bytes {
        data: large_data.into(),
    };

    match source {
        ExecutableSource::Bytes { data } => {
            assert_eq!(data.len(), 1024 * 1024);
        }
        _ => panic!("Expected Bytes source"),
    }
}

#[test]
fn test_port_mapping_well_known_ports() {
    let http = PortMapping {
        host_port: 80,
        container_port: 80,
        protocol: PortProtocol::Tcp,
    };

    let https = PortMapping {
        host_port: 443,
        container_port: 443,
        protocol: PortProtocol::Tcp,
    };

    assert_eq!(http.host_port, 80);
    assert_eq!(https.host_port, 443);
}

#[test]
fn test_port_mapping_high_port_numbers() {
    let port = PortMapping {
        host_port: 65535,
        container_port: 8080,
        protocol: PortProtocol::Tcp,
    };

    assert_eq!(port.host_port, 65535);
}

#[test]
fn test_volume_mount_nested_paths() {
    let mount = VolumeMount {
        source: PathBuf::from("/host/app/data/storage"),
        target: PathBuf::from("/container/app/data/storage"),
        mount_type: VolumeMountType::Bind,
        read_only: false,
    };

    assert!(mount.source.to_string_lossy().contains("storage"));
    assert!(mount.target.to_string_lossy().contains("storage"));
}

#[test]
fn test_workload_spec_python_multiline_code() {
    let code = r#"
def main():
    for i in range(10):
        print(f"Iteration {i}")

if __name__ == "__main__":
    main()
"#;

    let workload = WorkloadSpec::Python {
        source: PythonSource::Code {
            code: code.to_string(),
        },
        python_version: Some("3.11".to_string()),
        requirements: vec![],
        env_vars: HashMap::new(),
    };

    assert!(workload.validate().is_ok());
}
