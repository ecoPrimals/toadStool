//! Comprehensive tests for workload module
//!
//! Sprint 20: workload.rs coverage → 60%+
//! TARGET: PHASE 1 COMPLETION (65%)!
//! Estimated: ~50-60 tests

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

    assert_eq!(native, WorkloadType::Native);
    assert_eq!(wasm, WorkloadType::Wasm);
    assert_eq!(container, WorkloadType::Container);
    assert_eq!(gpu, WorkloadType::Gpu);
    assert_eq!(python, WorkloadType::Python);
}

#[test]
fn test_workload_type_equality() {
    assert_eq!(WorkloadType::Native, WorkloadType::Native);
    assert_ne!(WorkloadType::Native, WorkloadType::Wasm);
}

#[test]
fn test_workload_type_clone() {
    let wtype = WorkloadType::Container;
    let cloned = wtype.clone();

    assert_eq!(cloned, wtype);
}

#[test]
fn test_workload_type_debug() {
    let wtype = WorkloadType::Gpu;
    let debug = format!("{:?}", wtype);

    assert!(!debug.is_empty());
}

#[test]
fn test_workload_type_serialization() {
    let wtype = WorkloadType::Python;
    let json = serde_json::to_string(&wtype);

    assert!(json.is_ok());
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
        ExecutableSource::File { path } => assert_eq!(path, PathBuf::from("/usr/bin/echo")),
        _ => panic!("Expected File variant"),
    }
}

#[test]
fn test_executable_source_url() {
    let source = ExecutableSource::Url {
        url: "https://example.com/binary".to_string(),
    };

    match source {
        ExecutableSource::Url { url } => assert_eq!(url, "https://example.com/binary"),
        _ => panic!("Expected Url variant"),
    }
}

#[test]
fn test_executable_source_bytes() {
    let source = ExecutableSource::Bytes {
        data: vec![1, 2, 3, 4],
    };

    match source {
        ExecutableSource::Bytes { data } => assert_eq!(data, vec![1, 2, 3, 4]),
        _ => panic!("Expected Bytes variant"),
    }
}

#[test]
fn test_executable_source_clone() {
    let source = ExecutableSource::File {
        path: PathBuf::from("/bin/sh"),
    };
    let cloned = source.clone();

    match (source, cloned) {
        (ExecutableSource::File { path: p1 }, ExecutableSource::File { path: p2 }) => {
            assert_eq!(p1, p2)
        }
        _ => panic!("Clone mismatch"),
    }
}

#[test]
fn test_executable_source_debug() {
    let source = ExecutableSource::File {
        path: PathBuf::from("/usr/bin/test"),
    };
    let debug = format!("{:?}", source);

    assert!(!debug.is_empty());
}

#[test]
fn test_executable_source_serialization() {
    let source = ExecutableSource::Bytes { data: vec![5, 6] };
    let json = serde_json::to_string(&source);

    assert!(json.is_ok());
}

// ============================================================================
// WasmModuleSource Tests
// ============================================================================

#[test]
fn test_wasm_module_source_file() {
    let source = WasmModuleSource::File {
        path: PathBuf::from("/path/to/module.wasm"),
    };

    match source {
        WasmModuleSource::File { path } => {
            assert_eq!(path, PathBuf::from("/path/to/module.wasm"))
        }
        _ => panic!("Expected File variant"),
    }
}

#[test]
fn test_wasm_module_source_bytes() {
    let source = WasmModuleSource::Bytes {
        data: vec![0x00, 0x61, 0x73, 0x6d], // WASM magic number
    };

    match source {
        WasmModuleSource::Bytes { data } => assert_eq!(data, vec![0x00, 0x61, 0x73, 0x6d]),
        _ => panic!("Expected Bytes variant"),
    }
}

#[test]
fn test_wasm_module_source_url() {
    let source = WasmModuleSource::Url {
        url: "https://example.com/module.wasm".to_string(),
    };

    match source {
        WasmModuleSource::Url { url } => assert_eq!(url, "https://example.com/module.wasm"),
        _ => panic!("Expected Url variant"),
    }
}

#[test]
fn test_wasm_module_source_clone() {
    let source = WasmModuleSource::Bytes {
        data: vec![1, 2, 3],
    };
    let cloned = source.clone();

    match (source, cloned) {
        (WasmModuleSource::Bytes { data: d1 }, WasmModuleSource::Bytes { data: d2 }) => {
            assert_eq!(d1, d2)
        }
        _ => panic!("Clone mismatch"),
    }
}

#[test]
fn test_wasm_module_source_debug() {
    let source = WasmModuleSource::File {
        path: PathBuf::from("module.wasm"),
    };
    let debug = format!("{:?}", source);

    assert!(!debug.is_empty());
}

#[test]
fn test_wasm_module_source_serialization() {
    let source = WasmModuleSource::Url {
        url: "http://localhost/test.wasm".to_string(),
    };
    let json = serde_json::to_string(&source);

    assert!(json.is_ok());
}

// ============================================================================
// WasiConfig Tests
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
fn test_wasi_config_no_inheritance() {
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
        allowed_dirs: vec![],
        preopened_dirs: vec![],
        args: vec![],
    };
    let cloned = config.clone();

    assert_eq!(cloned.inherit_env, config.inherit_env);
    assert_eq!(cloned.inherit_stdio, config.inherit_stdio);
}

#[test]
fn test_wasi_config_debug() {
    let config = WasiConfig {
        inherit_env: true,
        inherit_stdio: true,
        allowed_dirs: vec![],
        preopened_dirs: vec![],
        args: vec![],
    };
    let debug = format!("{:?}", config);

    assert!(!debug.is_empty());
}

#[test]
fn test_wasi_config_serialization() {
    let config = WasiConfig {
        inherit_env: true,
        inherit_stdio: true,
        allowed_dirs: vec![],
        preopened_dirs: vec![],
        args: vec![],
    };
    let json = serde_json::to_string(&config);

    assert!(json.is_ok());
}

// ============================================================================
// VolumeMountType Tests
// ============================================================================

#[test]
fn test_volume_mount_type_variants() {
    let bind = VolumeMountType::Bind;
    let volume = VolumeMountType::Volume;
    let tmpfs = VolumeMountType::Tmpfs;

    assert!(matches!(bind, VolumeMountType::Bind));
    assert!(matches!(volume, VolumeMountType::Volume));
    assert!(matches!(tmpfs, VolumeMountType::Tmpfs));
}

#[test]
fn test_volume_mount_type_clone() {
    let mount_type = VolumeMountType::Bind;
    let cloned = mount_type.clone();

    assert!(matches!(cloned, VolumeMountType::Bind));
}

#[test]
fn test_volume_mount_type_debug() {
    let mount_type = VolumeMountType::Volume;
    let debug = format!("{:?}", mount_type);

    assert!(!debug.is_empty());
}

#[test]
fn test_volume_mount_type_serialization() {
    let mount_type = VolumeMountType::Tmpfs;
    let json = serde_json::to_string(&mount_type);

    assert!(json.is_ok());
}

// ============================================================================
// VolumeMount Tests
// ============================================================================

#[test]
fn test_volume_mount_creation() {
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
fn test_volume_mount_read_only() {
    let mount = VolumeMount {
        source: PathBuf::from("/host/config"),
        target: PathBuf::from("/container/config"),
        mount_type: VolumeMountType::Bind,
        read_only: true,
    };

    assert!(mount.read_only);
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

    assert_eq!(cloned.source, mount.source);
    assert_eq!(cloned.target, mount.target);
}

#[test]
fn test_volume_mount_debug() {
    let mount = VolumeMount {
        source: PathBuf::from("/a"),
        target: PathBuf::from("/b"),
        mount_type: VolumeMountType::Tmpfs,
        read_only: false,
    };
    let debug = format!("{:?}", mount);

    assert!(!debug.is_empty());
}

#[test]
fn test_volume_mount_serialization() {
    let mount = VolumeMount {
        source: PathBuf::from("/test"),
        target: PathBuf::from("/test2"),
        mount_type: VolumeMountType::Bind,
        read_only: true,
    };
    let json = serde_json::to_string(&mount);

    assert!(json.is_ok());
}

// ============================================================================
// PortProtocol Tests
// ============================================================================

#[test]
fn test_port_protocol_variants() {
    let tcp = PortProtocol::Tcp;
    let udp = PortProtocol::Udp;

    assert!(matches!(tcp, PortProtocol::Tcp));
    assert!(matches!(udp, PortProtocol::Udp));
}

#[test]
fn test_port_protocol_clone() {
    let protocol = PortProtocol::Tcp;
    let cloned = protocol.clone();

    assert!(matches!(cloned, PortProtocol::Tcp));
}

#[test]
fn test_port_protocol_debug() {
    let protocol = PortProtocol::Udp;
    let debug = format!("{:?}", protocol);

    assert!(!debug.is_empty());
}

#[test]
fn test_port_protocol_serialization() {
    let protocol = PortProtocol::Tcp;
    let json = serde_json::to_string(&protocol);

    assert!(json.is_ok());
}

// ============================================================================
// PortMapping Tests
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
fn test_port_mapping_udp() {
    let mapping = PortMapping {
        container_port: 53,
        host_port: 53,
        protocol: PortProtocol::Udp,
    };

    assert_eq!(mapping.container_port, 53);
    assert!(matches!(mapping.protocol, PortProtocol::Udp));
}

#[test]
fn test_port_mapping_clone() {
    let mapping = PortMapping {
        container_port: 443,
        host_port: 443,
        protocol: PortProtocol::Tcp,
    };
    let cloned = mapping.clone();

    assert_eq!(cloned.container_port, mapping.container_port);
    assert_eq!(cloned.host_port, mapping.host_port);
}

#[test]
fn test_port_mapping_debug() {
    let mapping = PortMapping {
        container_port: 3000,
        host_port: 3000,
        protocol: PortProtocol::Tcp,
    };
    let debug = format!("{:?}", mapping);

    assert!(!debug.is_empty());
}

#[test]
fn test_port_mapping_serialization() {
    let mapping = PortMapping {
        container_port: 5432,
        host_port: 5432,
        protocol: PortProtocol::Tcp,
    };
    let json = serde_json::to_string(&mapping);

    assert!(json.is_ok());
}

// ============================================================================
// RegistryAuth Tests
// ============================================================================

#[test]
fn test_registry_auth_creation() {
    let auth = RegistryAuth {
        username: "user".to_string(),
        password: "pass".to_string(),
        server_url: "docker.io".to_string(),
    };

    assert_eq!(auth.username, "user");
    assert_eq!(auth.password, "pass");
    assert_eq!(auth.server_url, "docker.io".to_string());
}

#[test]
fn test_registry_auth_default_registry() {
    let auth = RegistryAuth {
        username: "user".to_string(),
        password: "pass".to_string(),
        server_url: "https://index.docker.io/v1/".to_string(),
    };

    assert_eq!(auth.server_url, "https://index.docker.io/v1/");
}

#[test]
fn test_registry_auth_clone() {
    let auth = RegistryAuth {
        username: "test".to_string(),
        password: "secret".to_string(),
        server_url: "ghcr.io".to_string(),
    };
    let cloned = auth.clone();

    assert_eq!(cloned.username, auth.username);
    assert_eq!(cloned.password, auth.password);
}

#[test]
fn test_registry_auth_debug() {
    let auth = RegistryAuth {
        username: "u".to_string(),
        password: "p".to_string(),
        server_url: "registry.example.com".to_string(),
    };
    let debug = format!("{:?}", auth);

    assert!(!debug.is_empty());
}

#[test]
fn test_registry_auth_serialization() {
    let auth = RegistryAuth {
        username: "user".to_string(),
        password: "pass".to_string(),
        server_url: "registry.com".to_string(),
    };
    let json = serde_json::to_string(&auth);

    assert!(json.is_ok());
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
            data: vec![0x00, 0x61, 0x73, 0x6d],
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
fn test_workload_spec_python() {
    let spec = WorkloadSpec::Python {
        source: PythonSource::Code {
            code: "print('hello')".to_string(),
        },
        python_version: Some("3.11".to_string()),
        requirements: vec![],
        env_vars: HashMap::new(),
    };

    assert_eq!(spec.workload_type(), WorkloadType::Python);
}

#[test]
fn test_workload_spec_clone() {
    let spec = WorkloadSpec::default();
    let cloned = spec.clone();

    assert_eq!(cloned.workload_type(), spec.workload_type());
}

#[test]
fn test_workload_spec_debug() {
    let spec = WorkloadSpec::default();
    let debug = format!("{:?}", spec);

    assert!(!debug.is_empty());
}

#[test]
fn test_workload_spec_serialization() {
    let spec = WorkloadSpec::default();
    let json = serde_json::to_string(&spec);

    assert!(json.is_ok());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_executable_source_large_bytes() {
    let large_data = vec![0u8; 1024 * 1024]; // 1MB
    let source = ExecutableSource::Bytes { data: large_data };

    match source {
        ExecutableSource::Bytes { data } => assert_eq!(data.len(), 1024 * 1024),
        _ => panic!("Expected Bytes variant"),
    }
}

#[test]
fn test_wasi_config_many_dirs() {
    let dirs: Vec<PathBuf> = (0..100)
        .map(|i| PathBuf::from(format!("/dir{}", i)))
        .collect();

    let config = WasiConfig {
        inherit_env: true,
        inherit_stdio: true,
        allowed_dirs: dirs.clone(),
        preopened_dirs: vec![],
        args: vec![],
    };

    assert_eq!(config.allowed_dirs.len(), 100);
}

#[test]
fn test_port_mapping_high_ports() {
    let mapping = PortMapping {
        container_port: 65535,
        host_port: 65535,
        protocol: PortProtocol::Tcp,
    };

    assert_eq!(mapping.container_port, 65535);
}

#[test]
fn test_workload_spec_native_with_user() {
    let spec = WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: PathBuf::from("/usr/bin/id"),
        },
        args: None,
        working_dir: None,
        env_vars: HashMap::new(),
        user: Some("nobody".to_string()),
    };

    match spec {
        WorkloadSpec::Native { user, .. } => assert_eq!(user, Some("nobody".to_string())),
        _ => panic!("Expected Native variant"),
    }
}

#[test]
fn test_workload_spec_container_with_volumes() {
    let volumes = vec![
        VolumeMount {
            source: PathBuf::from("/host/vol1"),
            target: PathBuf::from("/container/vol1"),
            mount_type: VolumeMountType::Bind,
            read_only: false,
        },
        VolumeMount {
            source: PathBuf::from("/host/vol2"),
            target: PathBuf::from("/container/vol2"),
            mount_type: VolumeMountType::Volume,
            read_only: true,
        },
    ];

    let spec = WorkloadSpec::Container {
        image: "alpine:latest".to_string(),
        command: None,
        args: None,
        env_vars: HashMap::new(),
        working_dir: None,
        volumes: volumes.clone(),
        ports: vec![],
        registry_auth: None,
    };

    match spec {
        WorkloadSpec::Container { volumes: v, .. } => assert_eq!(v.len(), 2),
        _ => panic!("Expected Container variant"),
    }
}
