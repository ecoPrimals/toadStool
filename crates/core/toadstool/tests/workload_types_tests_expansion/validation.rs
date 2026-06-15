// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workload validation, edge cases, WASI config, and registry authentication.

use std::collections::HashMap;
use std::path::PathBuf;
use toadstool::workload::PortProtocol;
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
        program: GpuProgramSource::Cuda {
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
        args: Some(many_args),
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
        volumes,
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
        ports,
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
    let mut extended = large_module;
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
        requirements,
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
        program: GpuProgramSource::Cuda {
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
        allowed_dirs: dirs,
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
        args,
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
        preopened_dirs: dirs,
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
