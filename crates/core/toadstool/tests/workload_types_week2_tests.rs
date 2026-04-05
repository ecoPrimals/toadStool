// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 ecoPrimals
#![allow(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
//! Comprehensive tests for workload.rs types.

use std::collections::HashMap;
use std::path::PathBuf;
use toadstool::workload::*;

// ============================================================================
// WorkloadType Tests
// ============================================================================

#[test]
fn test_workload_type_variants() {
    let native = WorkloadType::Native;
    let wasm = WorkloadType::Wasm;
    let container = WorkloadType::Container;
    let gpu = WorkloadType::Gpu;
    let python = WorkloadType::Python;

    assert!(matches!(native, WorkloadType::Native));
    assert!(matches!(wasm, WorkloadType::Wasm));
    assert!(matches!(container, WorkloadType::Container));
    assert!(matches!(gpu, WorkloadType::Gpu));
    assert!(matches!(python, WorkloadType::Python));
}

#[test]
fn test_workload_type_equality() {
    assert_eq!(WorkloadType::Native, WorkloadType::Native);
    assert_eq!(WorkloadType::Wasm, WorkloadType::Wasm);
    assert_ne!(WorkloadType::Native, WorkloadType::Wasm);
}

#[test]
fn test_workload_type_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(WorkloadType::Native);
    set.insert(WorkloadType::Wasm);
    set.insert(WorkloadType::Native); // Duplicate

    assert_eq!(set.len(), 2);
    assert!(set.contains(&WorkloadType::Native));
}

#[test]
fn test_workload_type_clone() {
    let wt = WorkloadType::Container;
    let cloned = wt.clone();
    assert_eq!(wt, cloned);
}

// ============================================================================
// ExecutableSource Tests
// ============================================================================

#[test]
fn test_executable_source_file() {
    let source = ExecutableSource::File {
        path: PathBuf::from("/usr/bin/echo"),
    };

    match source {
        ExecutableSource::File { path } => {
            assert_eq!(path, PathBuf::from("/usr/bin/echo"));
        }
        _ => panic!("Expected File variant"),
    }
}

#[test]
fn test_executable_source_url() {
    let source = ExecutableSource::Url {
        url: "https://example.com/binary".to_string(),
    };

    match source {
        ExecutableSource::Url { url } => {
            assert_eq!(url, "https://example.com/binary");
        }
        _ => panic!("Expected Url variant"),
    }
}

#[test]
fn test_executable_source_bytes() {
    let data = vec![0x7f, 0x45, 0x4c, 0x46]; // ELF magic
    let source = ExecutableSource::Bytes {
        data: data.clone().into(),
    };

    match source {
        ExecutableSource::Bytes { data: d } => {
            assert_eq!(d, data);
        }
        _ => panic!("Expected Bytes variant"),
    }
}

#[test]
fn test_executable_source_clone() {
    let source = ExecutableSource::File {
        path: PathBuf::from("/bin/ls"),
    };
    let cloned = source.clone();

    match (source, cloned) {
        (ExecutableSource::File { path: p1 }, ExecutableSource::File { path: p2 }) => {
            assert_eq!(p1, p2);
        }
        _ => panic!("Clone failed"),
    }
}

// ============================================================================
// WasmModuleSource Tests
// ============================================================================

#[test]
fn test_wasm_module_source_file() {
    let source = WasmModuleSource::File {
        path: PathBuf::from("module.wasm"),
    };

    match source {
        WasmModuleSource::File { path } => {
            assert_eq!(path, PathBuf::from("module.wasm"));
        }
        _ => panic!("Expected File variant"),
    }
}

#[test]
fn test_wasm_module_source_bytes() {
    let wasm_bytes = vec![0x00, 0x61, 0x73, 0x6d]; // WASM magic
    let source = WasmModuleSource::Bytes {
        data: wasm_bytes.clone().into(),
    };

    match source {
        WasmModuleSource::Bytes { data } => {
            assert_eq!(data, wasm_bytes);
        }
        _ => panic!("Expected Bytes variant"),
    }
}

#[test]
fn test_wasm_module_source_url() {
    let source = WasmModuleSource::Url {
        url: "https://cdn.example.com/module.wasm".to_string(),
    };

    match source {
        WasmModuleSource::Url { url } => {
            assert_eq!(url, "https://cdn.example.com/module.wasm");
        }
        _ => panic!("Expected Url variant"),
    }
}

#[test]
fn test_wasm_module_source_clone() {
    let source = WasmModuleSource::Bytes {
        data: bytes::Bytes::from(vec![1, 2, 3, 4]),
    };
    let cloned = source.clone();

    match (source, cloned) {
        (WasmModuleSource::Bytes { data: d1 }, WasmModuleSource::Bytes { data: d2 }) => {
            assert_eq!(d1, d2);
        }
        _ => panic!("Clone failed"),
    }
}

// ============================================================================
// WasiConfig Tests
// ============================================================================

#[test]
fn test_wasi_config_basic() {
    let config = WasiConfig {
        inherit_env: true,
        inherit_stdio: true,
        allowed_dirs: vec![PathBuf::from("/tmp")],
        preopened_dirs: vec![PathBuf::from("/data")],
        args: vec!["arg1".to_string(), "arg2".to_string()],
    };

    assert!(config.inherit_env);
    assert!(config.inherit_stdio);
    assert_eq!(config.allowed_dirs.len(), 1);
    assert_eq!(config.preopened_dirs.len(), 1);
    assert_eq!(config.args.len(), 2);
}

#[test]
fn test_wasi_config_no_permissions() {
    let config = WasiConfig {
        inherit_env: false,
        inherit_stdio: false,
        allowed_dirs: vec![],
        preopened_dirs: vec![],
        args: vec![],
    };

    assert!(!config.inherit_env);
    assert!(!config.inherit_stdio);
    assert!(config.allowed_dirs.is_empty());
}

#[test]
fn test_wasi_config_clone() {
    let config = WasiConfig {
        inherit_env: true,
        inherit_stdio: false,
        allowed_dirs: vec![PathBuf::from("/tmp")],
        preopened_dirs: vec![],
        args: vec!["test".to_string()],
    };

    let cloned = config.clone();
    assert_eq!(config.inherit_env, cloned.inherit_env);
    assert_eq!(config.args.len(), cloned.args.len());
}

// ============================================================================
// VolumeMount Tests
// ============================================================================

#[test]
fn test_volume_mount_bind() {
    let mount = VolumeMount {
        source: PathBuf::from("/host/data"),
        target: PathBuf::from("/container/data"),
        mount_type: VolumeMountType::Bind,
        read_only: false,
    };

    assert_eq!(mount.source, PathBuf::from("/host/data"));
    assert_eq!(mount.target, PathBuf::from("/container/data"));
    assert!(!mount.read_only);
}

#[test]
fn test_volume_mount_readonly() {
    let mount = VolumeMount {
        source: PathBuf::from("/etc/config"),
        target: PathBuf::from("/app/config"),
        mount_type: VolumeMountType::Bind,
        read_only: true,
    };

    assert!(mount.read_only);
}

#[test]
fn test_volume_mount_types() {
    let bind = VolumeMountType::Bind;
    let volume = VolumeMountType::Volume;
    let tmpfs = VolumeMountType::Tmpfs;

    assert!(matches!(bind, VolumeMountType::Bind));
    assert!(matches!(volume, VolumeMountType::Volume));
    assert!(matches!(tmpfs, VolumeMountType::Tmpfs));
}

#[test]
fn test_volume_mount_clone() {
    let mount = VolumeMount {
        source: PathBuf::from("/src"),
        target: PathBuf::from("/dst"),
        mount_type: VolumeMountType::Volume,
        read_only: false,
    };

    let cloned = mount.clone();
    assert_eq!(mount.source, cloned.source);
    assert_eq!(mount.target, cloned.target);
    assert_eq!(mount.read_only, cloned.read_only);
}

// ============================================================================
// PortMapping Tests
// ============================================================================

#[test]
fn test_port_mapping_tcp() {
    let mapping = PortMapping {
        container_port: 8080,
        host_port: 80,
        protocol: PortProtocol::Tcp,
    };

    assert_eq!(mapping.container_port, 8080);
    assert_eq!(mapping.host_port, 80);
    assert!(matches!(mapping.protocol, PortProtocol::Tcp));
}

#[test]
fn test_port_mapping_udp() {
    let mapping = PortMapping {
        container_port: 53,
        host_port: 5353,
        protocol: PortProtocol::Udp,
    };

    assert_eq!(mapping.container_port, 53);
    assert_eq!(mapping.host_port, 5353);
    assert!(matches!(mapping.protocol, PortProtocol::Udp));
}

#[test]
fn test_port_protocol_variants() {
    let tcp = PortProtocol::Tcp;
    let udp = PortProtocol::Udp;

    assert!(matches!(tcp, PortProtocol::Tcp));
    assert!(matches!(udp, PortProtocol::Udp));
}

#[test]
fn test_port_mapping_clone() {
    let mapping = PortMapping {
        container_port: 443,
        host_port: 443,
        protocol: PortProtocol::Tcp,
    };

    let cloned = mapping.clone();
    assert_eq!(mapping.container_port, cloned.container_port);
    assert_eq!(mapping.host_port, cloned.host_port);
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
        password: "secret".to_string(),
        server_url: "https://index.docker.io/v1/".to_string(),
    };

    assert!(auth.server_url.contains("docker.io"));
}

#[test]
fn test_registry_auth_clone() {
    let auth = RegistryAuth {
        username: "test".to_string(),
        password: "pwd".to_string(),
        server_url: "https://reg.io".to_string(),
    };

    let cloned = auth.clone();
    assert_eq!(auth.username, cloned.username);
    assert_eq!(auth.server_url, cloned.server_url);
}

// ============================================================================
// GpuProgramSource Tests
// ============================================================================

#[test]
fn test_gpu_program_opencl() {
    let source = GpuProgramSource::OpenCL {
        source: "kernel void hello() { }".to_string(),
    };

    match source {
        GpuProgramSource::OpenCL { source: s } => {
            assert!(s.contains("kernel"));
        }
        _ => panic!("Expected OpenCL variant"),
    }
}

#[test]
fn test_gpu_program_cuda() {
    let source = GpuProgramSource::Cuda {
        source: "__global__ void kernel() { }".to_string(),
    };

    match source {
        GpuProgramSource::Cuda { source: s } => {
            assert!(s.contains("__global__"));
        }
        _ => panic!("Expected CUDA variant"),
    }
}

#[test]
fn test_gpu_program_vulkan() {
    let spirv = vec![0x03, 0x02, 0x23, 0x07]; // SPIR-V magic
    let source = GpuProgramSource::Vulkan {
        spirv: spirv.clone(),
    };

    match source {
        GpuProgramSource::Vulkan { spirv: s } => {
            assert_eq!(s, spirv);
        }
        _ => panic!("Expected Vulkan variant"),
    }
}

#[test]
fn test_gpu_program_source_clone() {
    let source = GpuProgramSource::OpenCL {
        source: "test".to_string(),
    };
    let cloned = source.clone();

    match (source, cloned) {
        (GpuProgramSource::OpenCL { source: s1 }, GpuProgramSource::OpenCL { source: s2 }) => {
            assert_eq!(s1, s2);
        }
        _ => panic!("Clone failed"),
    }
}

// ============================================================================
// GpuArgument Tests
// ============================================================================

#[test]
fn test_gpu_argument_buffer() {
    let arg = GpuArgument::Buffer {
        data: vec![1, 2, 3, 4],
    };

    match arg {
        GpuArgument::Buffer { data } => {
            assert_eq!(data.len(), 4);
        }
        _ => panic!("Expected Buffer variant"),
    }
}

#[test]
#[expect(clippy::approx_constant, reason = "PI approximation for test")]
fn test_gpu_argument_scalar() {
    let arg = GpuArgument::Scalar { value: 3.14 };

    match arg {
        GpuArgument::Scalar { value } => {
            assert!((value - 3.14).abs() < 0.001);
        }
        _ => panic!("Expected Scalar variant"),
    }
}

#[test]
fn test_gpu_argument_integer() {
    let arg = GpuArgument::Integer { value: 42 };

    match arg {
        GpuArgument::Integer { value } => {
            assert_eq!(value, 42);
        }
        _ => panic!("Expected Integer variant"),
    }
}

#[test]
fn test_gpu_argument_clone() {
    let arg = GpuArgument::Scalar { value: 2.71 };
    let cloned = arg.clone();

    match (arg, cloned) {
        (GpuArgument::Scalar { value: v1 }, GpuArgument::Scalar { value: v2 }) => {
            assert_eq!(v1, v2);
        }
        _ => panic!("Clone failed"),
    }
}

// ============================================================================
// PythonSource Tests
// ============================================================================

#[test]
fn test_python_source_code() {
    let source = PythonSource::Code {
        code: "print('Hello, World!')".to_string(),
    };

    match source {
        PythonSource::Code { code } => {
            assert!(code.contains("print"));
        }
        _ => panic!("Expected Code variant"),
    }
}

#[test]
fn test_python_source_file() {
    let source = PythonSource::File {
        path: PathBuf::from("script.py"),
    };

    match source {
        PythonSource::File { path } => {
            assert_eq!(path, PathBuf::from("script.py"));
        }
        _ => panic!("Expected File variant"),
    }
}

#[test]
fn test_python_source_module() {
    let source = PythonSource::Module {
        name: "numpy".to_string(),
    };

    match source {
        PythonSource::Module { name } => {
            assert_eq!(name, "numpy");
        }
        _ => panic!("Expected Module variant"),
    }
}

#[test]
fn test_python_source_clone() {
    let source = PythonSource::Code {
        code: "x = 1".to_string(),
    };
    let cloned = source.clone();

    match (source, cloned) {
        (PythonSource::Code { code: c1 }, PythonSource::Code { code: c2 }) => {
            assert_eq!(c1, c2);
        }
        _ => panic!("Clone failed"),
    }
}

// ============================================================================
// WorkloadSpec Tests
// ============================================================================

#[test]
fn test_workload_spec_default() {
    let spec = WorkloadSpec::default();
    assert_eq!(spec.workload_type(), WorkloadType::Native);
}

#[test]
fn test_workload_spec_native() {
    let spec = WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: PathBuf::from("/bin/echo"),
        },
        args: Some(vec!["hello".to_string()]),
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };

    assert_eq!(spec.workload_type(), WorkloadType::Native);
}

#[test]
fn test_workload_spec_wasm() {
    let spec = WorkloadSpec::Wasm {
        module: WasmModuleSource::Bytes {
            data: bytes::Bytes::from(vec![0x00, 0x61, 0x73, 0x6d]),
        },
        args: None,
        wasi_config: None,
        env_vars: HashMap::new(),
    };

    assert_eq!(spec.workload_type(), WorkloadType::Wasm);
}

#[test]
fn test_workload_spec_container() {
    let spec = WorkloadSpec::Container {
        image: "nginx:latest".to_string(),
        command: None,
        args: None,
        env_vars: HashMap::new(),
        working_dir: None,
        volumes: vec![],
        ports: vec![],
        registry_auth: None,
    };

    assert_eq!(spec.workload_type(), WorkloadType::Container);
}

#[test]
fn test_workload_spec_gpu() {
    let spec = WorkloadSpec::Gpu {
        program: GpuProgramSource::OpenCL {
            source: "kernel void test() { }".to_string(),
        },
        kernel_name: "test".to_string(),
        work_group_size: Some((16, 16, 1)),
        global_work_size: (256, 256, 1),
        args: vec![],
    };

    assert_eq!(spec.workload_type(), WorkloadType::Gpu);
}

#[test]
fn test_workload_spec_python() {
    let spec = WorkloadSpec::Python {
        source: PythonSource::Code {
            code: "print('test')".to_string(),
        },
        python_version: Some("3.11".to_string()),
        requirements: vec!["numpy".to_string()],
        env_vars: HashMap::new(),
    };

    assert_eq!(spec.workload_type(), WorkloadType::Python);
}

#[test]
fn test_workload_spec_with_environment() {
    let mut env = HashMap::new();
    env.insert("KEY1".to_string(), "VALUE1".to_string());
    env.insert("KEY2".to_string(), "VALUE2".to_string());

    let spec = WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: PathBuf::from("/bin/env"),
        },
        args: None,
        working_dir: None,
        env_vars: env.clone(),
        user: None,
    };

    match spec {
        WorkloadSpec::Native { env_vars, .. } => {
            assert_eq!(env_vars.len(), 2);
            assert_eq!(env_vars.get("KEY1"), Some(&"VALUE1".to_string()));
        }
        _ => panic!("Expected Native variant"),
    }
}

#[test]
fn test_workload_spec_clone() {
    let spec = WorkloadSpec::default();
    let cloned = spec.clone();

    assert_eq!(spec.workload_type(), cloned.workload_type());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_container_with_volumes_and_ports() {
    let volumes = vec![
        VolumeMount {
            source: PathBuf::from("/host/data"),
            target: PathBuf::from("/app/data"),
            mount_type: VolumeMountType::Bind,
            read_only: false,
        },
        VolumeMount {
            source: PathBuf::from("/host/config"),
            target: PathBuf::from("/app/config"),
            mount_type: VolumeMountType::Bind,
            read_only: true,
        },
    ];

    let ports = vec![
        PortMapping {
            container_port: 8080,
            host_port: 80,
            protocol: PortProtocol::Tcp,
        },
        PortMapping {
            container_port: 8443,
            host_port: 443,
            protocol: PortProtocol::Tcp,
        },
    ];

    let spec = WorkloadSpec::Container {
        image: "myapp:v1".to_string(),
        command: Some(vec!["./app".to_string()]),
        args: Some(vec!["--config=/app/config".to_string()]),
        env_vars: HashMap::new(),
        working_dir: Some("/app".to_string()),
        volumes,
        ports,
        registry_auth: None,
    };

    match spec {
        WorkloadSpec::Container { volumes, ports, .. } => {
            assert_eq!(volumes.len(), 2);
            assert_eq!(ports.len(), 2);
            assert!(volumes[1].read_only);
        }
        _ => panic!("Expected Container variant"),
    }
}

#[test]
fn test_wasm_with_wasi() {
    let wasi_config = WasiConfig {
        inherit_env: false,
        inherit_stdio: true,
        allowed_dirs: vec![PathBuf::from("/tmp")],
        preopened_dirs: vec![PathBuf::from("/data")],
        args: vec!["input.txt".to_string()],
    };

    let spec = WorkloadSpec::Wasm {
        module: WasmModuleSource::File {
            path: PathBuf::from("module.wasm"),
        },
        args: Some(vec!["arg1".to_string()]),
        wasi_config: Some(wasi_config),
        env_vars: HashMap::new(),
    };

    match spec {
        WorkloadSpec::Wasm { wasi_config, .. } => {
            assert!(wasi_config.is_some());
            let config = wasi_config.unwrap();
            assert!(config.inherit_stdio);
            assert_eq!(config.args.len(), 1);
        }
        _ => panic!("Expected Wasm variant"),
    }
}

#[test]
fn test_gpu_workload_with_arguments() {
    let args = vec![
        GpuArgument::Buffer {
            data: vec![0u8; 1024],
        },
        GpuArgument::Scalar { value: 1.5 },
        GpuArgument::Integer { value: 256 },
    ];

    let spec = WorkloadSpec::Gpu {
        program: GpuProgramSource::Cuda {
            source: "__global__ void process() { }".to_string(),
        },
        kernel_name: "process".to_string(),
        work_group_size: Some((32, 32, 1)),
        global_work_size: (1024, 1024, 1),
        args,
    };

    match spec {
        WorkloadSpec::Gpu { args, .. } => {
            assert_eq!(args.len(), 3);
            assert!(matches!(args[0], GpuArgument::Buffer { .. }));
            assert!(matches!(args[1], GpuArgument::Scalar { .. }));
            assert!(matches!(args[2], GpuArgument::Integer { .. }));
        }
        _ => panic!("Expected Gpu variant"),
    }
}
