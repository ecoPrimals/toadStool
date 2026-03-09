// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive workload types tests - Expansion Pack
//!
//! Additional tests for workload validation, edge cases, WASI config,
//! registry authentication, GPU arguments, and error scenarios.

use std::collections::HashMap;
use std::path::PathBuf;
use toadstool::workload::{PortMapping, PortProtocol};
use toadstool::*;

// ============================================================================
// WorkloadSpec Validation Tests
// ============================================================================

#[test]
fn test_workload_spec_validate_native_success() {
    let workload = WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: PathBuf::from("/bin/ls"),
        },
        args: Some(vec!["-l".to_string()]),
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };

    assert!(workload.validate().is_ok());
}

#[test]
fn test_workload_spec_validate_wasm_success() {
    let workload = WorkloadSpec::Wasm {
        module: WasmModuleSource::Bytes {
            data: bytes::Bytes::from(vec![0x00, 0x61, 0x73, 0x6d, 1, 0, 0, 0]),
        },
        args: Some(vec![]),
        wasi_config: None,
        env_vars: HashMap::new(),
    };

    assert!(workload.validate().is_ok());
}

#[test]
fn test_workload_spec_validate_container_success() {
    let workload = WorkloadSpec::Container {
        image: "alpine:3.18".to_string(),
        command: None,
        args: None,
        env_vars: HashMap::new(),
        working_dir: None,
        volumes: vec![],
        ports: vec![],
        registry_auth: None,
    };

    assert!(workload.validate().is_ok());
}

#[test]
fn test_workload_spec_validate_python_code_success() {
    let workload = WorkloadSpec::Python {
        source: PythonSource::Code {
            code: "print('test')".to_string(),
        },
        python_version: Some("3.11".to_string()),
        requirements: vec![],
        env_vars: HashMap::new(),
    };

    assert!(workload.validate().is_ok());
}

#[test]
fn test_workload_spec_validate_python_empty_code_error() {
    let workload = WorkloadSpec::Python {
        source: PythonSource::Code {
            code: String::new(),
        },
        python_version: None,
        requirements: vec![],
        env_vars: HashMap::new(),
    };

    assert!(workload.validate().is_err());
}

#[test]
fn test_workload_spec_validate_python_empty_module_error() {
    let workload = WorkloadSpec::Python {
        source: PythonSource::Module {
            name: String::new(),
        },
        python_version: None,
        requirements: vec![],
        env_vars: HashMap::new(),
    };

    assert!(workload.validate().is_err());
}

#[test]
fn test_workload_spec_validate_gpu_success() {
    let workload = WorkloadSpec::Gpu {
        program: GpuProgramSource::OpenCL {
            source: "kernel void test() {}".to_string(),
        },
        kernel_name: "test".to_string(),
        work_group_size: Some((16, 16, 1)),
        global_work_size: (256, 256, 1),
        args: vec![],
    };

    assert!(workload.validate().is_ok());
}

// ============================================================================
// WorkloadSpec Edge Cases
// ============================================================================

#[test]
fn test_workload_spec_native_with_many_args() {
    let many_args: Vec<String> = (0..100).map(|i| format!("arg{i}")).collect();

    let workload = WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: PathBuf::from("/bin/echo"),
        },
        args: Some(many_args.clone()),
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };

    match workload {
        WorkloadSpec::Native { args, .. } => {
            assert_eq!(args.unwrap().len(), 100);
        }
        _ => panic!("Expected Native workload"),
    }
}

#[test]
fn test_workload_spec_native_with_many_env_vars() {
    let mut env_vars = HashMap::new();
    for i in 0..50 {
        env_vars.insert(format!("VAR{i}"), format!("value{i}"));
    }

    let workload = WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: PathBuf::from("/bin/env"),
        },
        args: None,
        working_dir: None,
        env_vars: env_vars.clone(),
        user: None,
    };

    match workload {
        WorkloadSpec::Native { env_vars: env, .. } => {
            assert_eq!(env.len(), 50);
        }
        _ => panic!("Expected Native workload"),
    }
}

#[test]
fn test_workload_spec_native_with_user() {
    let workload = WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: PathBuf::from("/bin/whoami"),
        },
        args: None,
        working_dir: None,
        env_vars: HashMap::new(),
        user: Some("nobody".to_string()),
    };

    match workload {
        WorkloadSpec::Native { user, .. } => {
            assert_eq!(user, Some("nobody".to_string()));
        }
        _ => panic!("Expected Native workload"),
    }
}

#[test]
fn test_workload_spec_container_with_volumes() {
    let volumes = vec![
        VolumeMount {
            source: PathBuf::from("/host/data"),
            target: PathBuf::from("/app/data"),
            mount_type: VolumeMountType::Bind,
            read_only: true,
        },
        VolumeMount {
            source: PathBuf::from("/host/config"),
            target: PathBuf::from("/app/config"),
            mount_type: VolumeMountType::Bind,
            read_only: true,
        },
    ];

    let workload = WorkloadSpec::Container {
        image: "myapp:latest".to_string(),
        command: None,
        args: None,
        env_vars: HashMap::new(),
        working_dir: None,
        volumes: volumes.clone(),
        ports: vec![],
        registry_auth: None,
    };

    match workload {
        WorkloadSpec::Container { volumes: v, .. } => {
            assert_eq!(v.len(), 2);
        }
        _ => panic!("Expected Container workload"),
    }
}

#[test]
fn test_workload_spec_container_with_ports() {
    let ports = vec![
        WorkloadPortMapping {
            container_port: 80,
            host_port: 8080,
            protocol: PortProtocol::Tcp,
        },
        WorkloadPortMapping {
            container_port: 443,
            host_port: 8443,
            protocol: PortProtocol::Tcp,
        },
    ];

    let workload = WorkloadSpec::Container {
        image: "nginx:latest".to_string(),
        command: None,
        args: None,
        env_vars: HashMap::new(),
        working_dir: None,
        volumes: vec![],
        ports: ports.clone(),
        registry_auth: None,
    };

    match workload {
        WorkloadSpec::Container { ports: p, .. } => {
            assert_eq!(p.len(), 2);
        }
        _ => panic!("Expected Container workload"),
    }
}

#[test]
fn test_workload_spec_wasm_with_large_module() {
    let large_module: Vec<u8> = vec![0x00, 0x61, 0x73, 0x6d]; // start of valid WASM
    let mut extended = large_module.clone();
    extended.extend(vec![0; 10000]); // 10KB module

    let workload = WorkloadSpec::Wasm {
        module: WasmModuleSource::Bytes {
            data: extended.into(),
        },
        args: None,
        wasi_config: None,
        env_vars: HashMap::new(),
    };

    match workload {
        WorkloadSpec::Wasm { module, .. } => match module {
            WasmModuleSource::Bytes { data } => {
                assert!(data.len() > 10000);
            }
            _ => panic!("Expected Bytes module source"),
        },
        _ => panic!("Expected Wasm workload"),
    }
}

#[test]
fn test_workload_spec_python_with_many_requirements() {
    let requirements: Vec<String> = vec![
        "numpy==1.24.0".to_string(),
        "pandas==2.0.0".to_string(),
        "scikit-learn==1.2.2".to_string(),
        "tensorflow==2.12.0".to_string(),
        "torch==2.0.0".to_string(),
    ];

    let workload = WorkloadSpec::Python {
        source: PythonSource::Code {
            code: "import numpy as np\nprint(np.__version__)".to_string(),
        },
        python_version: Some("3.11".to_string()),
        requirements: requirements.clone(),
        env_vars: HashMap::new(),
    };

    match workload {
        WorkloadSpec::Python {
            requirements: req, ..
        } => {
            assert_eq!(req.len(), 5);
        }
        _ => panic!("Expected Python workload"),
    }
}

#[test]
fn test_workload_spec_gpu_with_3d_work_group() {
    let workload = WorkloadSpec::Gpu {
        program: GpuProgramSource::OpenCL {
            source: "kernel void compute() {}".to_string(),
        },
        kernel_name: "compute".to_string(),
        work_group_size: Some((8, 8, 8)),
        global_work_size: (128, 128, 128),
        args: vec![],
    };

    match workload {
        WorkloadSpec::Gpu {
            work_group_size,
            global_work_size,
            ..
        } => {
            assert_eq!(work_group_size, Some((8, 8, 8)));
            assert_eq!(global_work_size, (128, 128, 128));
        }
        _ => panic!("Expected GPU workload"),
    }
}

// ============================================================================
// WasiConfig Tests
// ============================================================================

#[test]
fn test_wasi_config_default() {
    let wasi = WasiConfig {
        inherit_env: false,
        inherit_stdio: false,
        allowed_dirs: vec![],
        preopened_dirs: vec![],
        args: vec![],
    };

    assert!(!wasi.inherit_env);
    assert!(!wasi.inherit_stdio);
    assert!(wasi.args.is_empty());
}

#[test]
fn test_wasi_config_with_allowed_dirs() {
    let dirs = vec![PathBuf::from("/data"), PathBuf::from("/tmp")];

    let wasi = WasiConfig {
        inherit_env: false,
        inherit_stdio: false,
        allowed_dirs: dirs.clone(),
        preopened_dirs: vec![],
        args: vec![],
    };

    assert_eq!(wasi.allowed_dirs.len(), 2);
}

#[test]
fn test_wasi_config_with_args() {
    let args = vec![
        "--verbose".to_string(),
        "--output".to_string(),
        "file.txt".to_string(),
    ];

    let wasi = WasiConfig {
        inherit_env: false,
        inherit_stdio: false,
        allowed_dirs: vec![],
        preopened_dirs: vec![],
        args: args.clone(),
    };

    assert_eq!(wasi.args.len(), 3);
}

#[test]
fn test_wasi_config_with_preopened_dirs() {
    let dirs = vec![PathBuf::from("/data"), PathBuf::from("/tmp")];

    let wasi = WasiConfig {
        inherit_env: false,
        inherit_stdio: false,
        allowed_dirs: vec![],
        preopened_dirs: dirs.clone(),
        args: vec![],
    };

    assert_eq!(wasi.preopened_dirs.len(), 2);
}

#[test]
fn test_wasi_config_inherit_env() {
    let wasi = WasiConfig {
        inherit_env: true,
        inherit_stdio: true,
        allowed_dirs: vec![],
        preopened_dirs: vec![],
        args: vec![],
    };

    assert!(wasi.inherit_env);
    assert!(wasi.inherit_stdio);
}

#[test]
fn test_wasi_config_clone() {
    let wasi1 = WasiConfig {
        inherit_env: true,
        inherit_stdio: false,
        allowed_dirs: vec![],
        preopened_dirs: vec![],
        args: vec!["test".to_string()],
    };

    let wasi2 = wasi1.clone();
    assert_eq!(wasi1.inherit_env, wasi2.inherit_env);
    assert_eq!(wasi1.args.len(), wasi2.args.len());
}

// ============================================================================
// RegistryAuth Tests
// ============================================================================

#[test]
fn test_registry_auth_basic() {
    let auth = RegistryAuth {
        username: "user".to_string(),
        password: "pass".to_string(),
        server_url: "https://registry.example.com".to_string(),
    };

    assert_eq!(auth.username, "user");
    assert_eq!(auth.password, "pass");
    assert_eq!(auth.server_url, "https://registry.example.com");
}

#[test]
fn test_registry_auth_dockerhub() {
    let auth = RegistryAuth {
        username: "dockeruser".to_string(),
        password: "dockerpass".to_string(),
        server_url: "https://registry-1.docker.io".to_string(),
    };

    assert_eq!(auth.username, "dockeruser");
    assert!(auth.server_url.contains("docker.io"));
}

#[test]
fn test_registry_auth_github_packages() {
    let auth = RegistryAuth {
        username: "github-user".to_string(),
        password: "ghp_token123".to_string(),
        server_url: "https://ghcr.io".to_string(),
    };

    assert_eq!(auth.server_url, "https://ghcr.io");
}

#[test]
fn test_registry_auth_empty_password() {
    let auth = RegistryAuth {
        username: "user".to_string(),
        password: String::new(),
        server_url: "https://registry.local".to_string(),
    };

    assert!(auth.password.is_empty());
}

#[test]
fn test_registry_auth_clone() {
    let auth1 = RegistryAuth {
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        server_url: "https://test.registry".to_string(),
    };
    let auth2 = auth1.clone();

    assert_eq!(auth1.username, auth2.username);
    assert_eq!(auth1.password, auth2.password);
    assert_eq!(auth1.server_url, auth2.server_url);
}

// ============================================================================
// GpuArgument Tests
// ============================================================================

#[test]
fn test_gpu_argument_buffer() {
    let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
    let arg = GpuArgument::Buffer { data: data.clone() };

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
    let arg = GpuArgument::Buffer {
        data: large_data.clone(),
    };

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
        data: large_data.clone().into(),
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
