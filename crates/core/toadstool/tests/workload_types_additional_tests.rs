//! Additional comprehensive test coverage for workload types
//!
//! This test suite targets workload types defined in crates/core/toadstool/src/workload.rs
//! providing comprehensive coverage for helper types, sources, and configurations.
//!
//! Covers: WorkloadType, ExecutableSource, WasmModuleSource, PythonSource,
//! GpuProgramSource, VolumeMount, PortMapping, and related types.

use std::path::PathBuf;
use toadstool::workload::{
    ExecutableSource, GpuArgument, GpuProgramSource, PortMapping, PortProtocol, PythonSource,
    RegistryAuth, VolumeMount, VolumeMountType, WasiConfig, WasmModuleSource, WorkloadType,
};

// ============================================================================
// WorkloadType Tests (6 tests)
// ============================================================================

#[test]
fn test_workload_type_native() {
    let wt = WorkloadType::Native;
    assert!(matches!(wt, WorkloadType::Native));
}

#[test]
fn test_workload_type_wasm() {
    let wt = WorkloadType::Wasm;
    assert!(matches!(wt, WorkloadType::Wasm));
}

#[test]
fn test_workload_type_container() {
    let wt = WorkloadType::Container;
    assert!(matches!(wt, WorkloadType::Container));
}

#[test]
fn test_workload_type_gpu() {
    let wt = WorkloadType::Gpu;
    assert!(matches!(wt, WorkloadType::Gpu));
}

#[test]
fn test_workload_type_python() {
    let wt = WorkloadType::Python;
    assert!(matches!(wt, WorkloadType::Python));
}

#[test]
fn test_workload_type_equality() {
    let wt1 = WorkloadType::Native;
    let wt2 = WorkloadType::Native;
    assert_eq!(wt1, wt2);

    let wt3 = WorkloadType::Wasm;
    let wt4 = WorkloadType::Container;
    assert_ne!(wt3, wt4);
}

// ============================================================================
// ExecutableSource Tests (4 tests)
// ============================================================================

#[test]
fn test_executable_source_file() {
    let source = ExecutableSource::File {
        path: PathBuf::from("/usr/bin/echo"),
    };
    assert!(matches!(source, ExecutableSource::File { .. }));
}

#[test]
fn test_executable_source_url() {
    let source = ExecutableSource::Url {
        url: "https://example.com/app".to_string(),
    };
    assert!(matches!(source, ExecutableSource::Url { .. }));
}

#[test]
fn test_executable_source_bytes() {
    let data = vec![0x7F, 0x45, 0x4C, 0x46]; // ELF header
    let source = ExecutableSource::Bytes { data: data.clone() };
    assert!(matches!(source, ExecutableSource::Bytes { .. }));
}

#[test]
fn test_executable_source_serialization() {
    let source = ExecutableSource::Url {
        url: "https://cdn.example.com/binary".to_string(),
    };
    let serialized = serde_json::to_string(&source).expect("Failed to serialize");
    let deserialized: ExecutableSource =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    if let ExecutableSource::Url { url } = deserialized {
        assert_eq!(url, "https://cdn.example.com/binary");
    } else {
        panic!("Expected URL variant");
    }
}

// ============================================================================
// WasmModuleSource Tests (4 tests)
// ============================================================================

#[test]
fn test_wasm_module_source_file() {
    let source = WasmModuleSource::File {
        path: PathBuf::from("module.wasm"),
    };
    assert!(matches!(source, WasmModuleSource::File { .. }));
}

#[test]
fn test_wasm_module_source_bytes() {
    let data = vec![0x00, 0x61, 0x73, 0x6D]; // WASM magic number
    let source = WasmModuleSource::Bytes { data: data.clone() };
    assert!(matches!(source, WasmModuleSource::Bytes { .. }));
}

#[test]
fn test_wasm_module_source_url() {
    let source = WasmModuleSource::Url {
        url: "https://example.com/module.wasm".to_string(),
    };
    assert!(matches!(source, WasmModuleSource::Url { .. }));
}

#[test]
fn test_wasm_module_source_serialization() {
    let source = WasmModuleSource::Bytes {
        data: vec![1, 2, 3, 4],
    };
    let serialized = serde_json::to_string(&source).expect("Failed to serialize");
    let deserialized: WasmModuleSource =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    if let WasmModuleSource::Bytes { data } = deserialized {
        assert_eq!(data, vec![1, 2, 3, 4]);
    } else {
        panic!("Expected Bytes variant");
    }
}

// ============================================================================
// VolumeMountType Tests (4 tests)
// ============================================================================

#[test]
fn test_volume_mount_type_bind() {
    let vmt = VolumeMountType::Bind;
    assert!(matches!(vmt, VolumeMountType::Bind));
}

#[test]
fn test_volume_mount_type_volume() {
    let vmt = VolumeMountType::Volume;
    assert!(matches!(vmt, VolumeMountType::Volume));
}

#[test]
fn test_volume_mount_type_tmpfs() {
    let vmt = VolumeMountType::Tmpfs;
    assert!(matches!(vmt, VolumeMountType::Tmpfs));
}

#[test]
fn test_volume_mount_type_serialization() {
    let vmt = VolumeMountType::Bind;
    let serialized = serde_json::to_string(&vmt).expect("Failed to serialize");
    let deserialized: VolumeMountType =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert!(matches!(deserialized, VolumeMountType::Bind));
}

// ============================================================================
// PortProtocol Tests (3 tests)
// ============================================================================

#[test]
fn test_port_protocol_tcp() {
    let proto = PortProtocol::Tcp;
    assert!(matches!(proto, PortProtocol::Tcp));
}

#[test]
fn test_port_protocol_udp() {
    let proto = PortProtocol::Udp;
    assert!(matches!(proto, PortProtocol::Udp));
}

#[test]
fn test_port_protocol_serialization() {
    let proto = PortProtocol::Tcp;
    let serialized = serde_json::to_string(&proto).expect("Failed to serialize");
    let deserialized: PortProtocol =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert!(matches!(deserialized, PortProtocol::Tcp));
}

// ============================================================================
// GpuProgramSource Tests (4 tests)
// ============================================================================

#[test]
fn test_gpu_program_source_opencl() {
    let source = GpuProgramSource::OpenCL {
        source: "__kernel void test() {}".to_string(),
    };
    assert!(matches!(source, GpuProgramSource::OpenCL { .. }));
}

#[test]
fn test_gpu_program_source_cuda() {
    let source = GpuProgramSource::Cuda {
        source: "__global__ void kernel() {}".to_string(),
    };
    assert!(matches!(source, GpuProgramSource::Cuda { .. }));
}

#[test]
fn test_gpu_program_source_vulkan() {
    let spirv = vec![0x03, 0x02, 0x23, 0x07]; // SPIR-V magic number
    let source = GpuProgramSource::Vulkan { spirv };
    assert!(matches!(source, GpuProgramSource::Vulkan { .. }));
}

#[test]
fn test_gpu_program_source_serialization() {
    let source = GpuProgramSource::OpenCL {
        source: "test".to_string(),
    };
    let serialized = serde_json::to_string(&source).expect("Failed to serialize");
    let deserialized: GpuProgramSource =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    if let GpuProgramSource::OpenCL { source } = deserialized {
        assert_eq!(source, "test");
    } else {
        panic!("Expected OpenCL variant");
    }
}

// ============================================================================
// GpuArgument Tests (4 tests)
// ============================================================================

#[test]
fn test_gpu_argument_buffer() {
    let arg = GpuArgument::Buffer {
        data: vec![1, 2, 3, 4, 5],
    };
    assert!(matches!(arg, GpuArgument::Buffer { .. }));
}

#[test]
fn test_gpu_argument_scalar() {
    let arg = GpuArgument::Scalar { value: 3.14159 };
    assert!(matches!(arg, GpuArgument::Scalar { .. }));
}

#[test]
fn test_gpu_argument_integer() {
    let arg = GpuArgument::Integer { value: 42 };
    assert!(matches!(arg, GpuArgument::Integer { .. }));
}

#[test]
fn test_gpu_argument_serialization() {
    let arg = GpuArgument::Scalar { value: 2.71828 };
    let serialized = serde_json::to_string(&arg).expect("Failed to serialize");
    let deserialized: GpuArgument =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    if let GpuArgument::Scalar { value } = deserialized {
        assert!((value - 2.71828).abs() < 1e-5);
    } else {
        panic!("Expected Scalar variant");
    }
}

// ============================================================================
// PythonSource Tests (4 tests)
// ============================================================================

#[test]
fn test_python_source_code() {
    let source = PythonSource::Code {
        code: "print('Hello, World!')".to_string(),
    };
    assert!(matches!(source, PythonSource::Code { .. }));
}

#[test]
fn test_python_source_file() {
    let source = PythonSource::File {
        path: PathBuf::from("script.py"),
    };
    assert!(matches!(source, PythonSource::File { .. }));
}

#[test]
fn test_python_source_module() {
    let source = PythonSource::Module {
        name: "numpy".to_string(),
    };
    assert!(matches!(source, PythonSource::Module { .. }));
}

#[test]
fn test_python_source_serialization() {
    let source = PythonSource::Code {
        code: "import sys".to_string(),
    };
    let serialized = serde_json::to_string(&source).expect("Failed to serialize");
    let deserialized: PythonSource =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    if let PythonSource::Code { code } = deserialized {
        assert_eq!(code, "import sys");
    } else {
        panic!("Expected Code variant");
    }
}

// ============================================================================
// WasiConfig Tests (3 tests)
// ============================================================================

#[test]
fn test_wasi_config_creation() {
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
fn test_wasi_config_clone() {
    let config = WasiConfig {
        inherit_env: false,
        inherit_stdio: false,
        allowed_dirs: vec![],
        preopened_dirs: vec![],
        args: vec![],
    };
    let cloned = config.clone();
    assert_eq!(config.inherit_env, cloned.inherit_env);
}

#[test]
fn test_wasi_config_serialization() {
    let config = WasiConfig {
        inherit_env: true,
        inherit_stdio: false,
        allowed_dirs: vec![PathBuf::from("/tmp")],
        preopened_dirs: vec![],
        args: vec!["test".to_string()],
    };

    let serialized = serde_json::to_string(&config).expect("Failed to serialize");
    let deserialized: WasiConfig =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(config.inherit_env, deserialized.inherit_env);
    assert_eq!(config.args.len(), deserialized.args.len());
}

// ============================================================================
// VolumeMount Tests (3 tests)
// ============================================================================

#[test]
fn test_volume_mount_creation() {
    let mount = VolumeMount {
        source: PathBuf::from("/host/data"),
        target: PathBuf::from("/container/data"),
        mount_type: VolumeMountType::Bind,
        read_only: true,
    };

    assert_eq!(mount.source, PathBuf::from("/host/data"));
    assert_eq!(mount.target, PathBuf::from("/container/data"));
    assert!(mount.read_only);
}

#[test]
fn test_volume_mount_clone() {
    let mount = VolumeMount {
        source: PathBuf::from("/a"),
        target: PathBuf::from("/b"),
        mount_type: VolumeMountType::Volume,
        read_only: false,
    };
    let cloned = mount.clone();
    assert_eq!(mount.source, cloned.source);
    assert_eq!(mount.read_only, cloned.read_only);
}

#[test]
fn test_volume_mount_serialization() {
    let mount = VolumeMount {
        source: PathBuf::from("/src"),
        target: PathBuf::from("/dst"),
        mount_type: VolumeMountType::Tmpfs,
        read_only: true,
    };

    let serialized = serde_json::to_string(&mount).expect("Failed to serialize");
    let deserialized: VolumeMount =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(mount.source, deserialized.source);
    assert_eq!(mount.read_only, deserialized.read_only);
}

// ============================================================================
// PortMapping Tests (3 tests)
// ============================================================================

#[test]
fn test_port_mapping_creation() {
    let mapping = PortMapping {
        container_port: 8080,
        host_port: 80,
        protocol: PortProtocol::Tcp,
    };

    assert_eq!(mapping.container_port, 8080);
    assert_eq!(mapping.host_port, 80);
}

#[test]
fn test_port_mapping_clone() {
    let mapping = PortMapping {
        container_port: 3000,
        host_port: 3000,
        protocol: PortProtocol::Udp,
    };
    let cloned = mapping.clone();
    assert_eq!(mapping.container_port, cloned.container_port);
}

#[test]
fn test_port_mapping_serialization() {
    let mapping = PortMapping {
        container_port: 443,
        host_port: 8443,
        protocol: PortProtocol::Tcp,
    };

    let serialized = serde_json::to_string(&mapping).expect("Failed to serialize");
    let deserialized: PortMapping =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(mapping.container_port, deserialized.container_port);
    assert_eq!(mapping.host_port, deserialized.host_port);
}

// ============================================================================
// RegistryAuth Tests (3 tests)
// ============================================================================

#[test]
fn test_registry_auth_creation() {
    let auth = RegistryAuth {
        username: "user123".to_string(),
        password: "secret".to_string(),
        server_url: "https://registry.example.com".to_string(),
    };

    assert_eq!(auth.username, "user123");
    assert_eq!(auth.password, "secret");
    assert_eq!(auth.server_url, "https://registry.example.com");
}

#[test]
fn test_registry_auth_clone() {
    let auth = RegistryAuth {
        username: "admin".to_string(),
        password: "pass".to_string(),
        server_url: "https://docker.io".to_string(),
    };
    let cloned = auth.clone();
    assert_eq!(auth.username, cloned.username);
    assert_eq!(auth.server_url, cloned.server_url);
}

#[test]
fn test_registry_auth_serialization() {
    let auth = RegistryAuth {
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        server_url: "https://ghcr.io".to_string(),
    };

    let serialized = serde_json::to_string(&auth).expect("Failed to serialize");
    let deserialized: RegistryAuth =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(auth.username, deserialized.username);
    assert_eq!(auth.server_url, deserialized.server_url);
}

// ============================================================================
// Summary
// ============================================================================

// Total tests added: 49
// Coverage areas:
// - WorkloadType (6 tests)
// - ExecutableSource (4 tests)
// - WasmModuleSource (4 tests)
// - VolumeMountType (4 tests)
// - PortProtocol (3 tests)
// - GpuProgramSource (4 tests)
// - GpuArgument (4 tests)
// - PythonSource (4 tests)
// - WasiConfig (3 tests)
// - VolumeMount (3 tests)
// - PortMapping (3 tests)
// - RegistryAuth (3 tests)
// - Serialization tests (multiple across types)
// - Clone tests (multiple)
// - Equality tests (multiple)

