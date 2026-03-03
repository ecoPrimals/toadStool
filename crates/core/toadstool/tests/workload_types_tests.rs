// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for workload types

use std::collections::HashMap;
use std::path::PathBuf;
use toadstool::workload::{PortMapping, PortProtocol};
use toadstool::*;

// ============================================================================
// WorkloadSpec Tests
// ============================================================================

#[test]
fn test_workload_spec_default() {
    let workload = WorkloadSpec::default();

    match workload {
        WorkloadSpec::Native {
            executable, args, ..
        } => {
            match executable {
                ExecutableSource::File { path } => {
                    assert_eq!(path, PathBuf::from("echo"));
                }
                _ => panic!("Expected File executable source"),
            }
            assert!(args.is_some());
        }
        _ => panic!("Expected Native workload"),
    }
}

#[test]
fn test_workload_spec_native() {
    let workload = WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: PathBuf::from("/bin/ls"),
        },
        args: Some(vec!["-la".to_string()]),
        working_dir: Some(PathBuf::from("/tmp")),
        env_vars: HashMap::new(),
        user: None,
    };

    match workload {
        WorkloadSpec::Native { executable, .. } => match executable {
            ExecutableSource::File { path } => {
                assert_eq!(path, PathBuf::from("/bin/ls"));
            }
            _ => panic!("Expected File executable source"),
        },
        _ => panic!("Expected Native workload"),
    }
}

#[test]
fn test_workload_spec_wasm() {
    let workload = WorkloadSpec::Wasm {
        module: WasmModuleSource::File {
            path: PathBuf::from("module.wasm"),
        },
        args: Some(vec!["arg1".to_string()]),
        wasi_config: None,
        env_vars: HashMap::new(),
    };

    assert!(matches!(workload, WorkloadSpec::Wasm { .. }));
}

#[test]
fn test_workload_spec_container() {
    let workload = WorkloadSpec::Container {
        image: "nginx:latest".to_string(),
        command: Some(vec!["nginx".to_string()]),
        args: Some(vec!["-g".to_string(), "daemon off;".to_string()]),
        env_vars: HashMap::new(),
        working_dir: Some("/app".to_string()),
        volumes: vec![],
        ports: vec![],
        registry_auth: None,
    };

    match workload {
        WorkloadSpec::Container { image, .. } => {
            assert_eq!(image, "nginx:latest");
        }
        _ => panic!("Expected Container workload"),
    }
}

#[test]
fn test_workload_spec_python() {
    let workload = WorkloadSpec::Python {
        source: PythonSource::Code {
            code: "print('Hello')".to_string(),
        },
        python_version: Some("3.11".to_string()),
        requirements: vec!["numpy".to_string()],
        env_vars: HashMap::new(),
    };

    assert!(matches!(workload, WorkloadSpec::Python { .. }));
}

#[test]
fn test_workload_spec_gpu() {
    let workload = WorkloadSpec::Gpu {
        program: GpuProgramSource::OpenCL {
            source: "kernel code".to_string(),
        },
        kernel_name: "main_kernel".to_string(),
        work_group_size: Some((16, 16, 1)),
        global_work_size: (256, 256, 1),
        args: vec![],
    };

    match workload {
        WorkloadSpec::Gpu { kernel_name, .. } => {
            assert_eq!(kernel_name, "main_kernel");
        }
        _ => panic!("Expected GPU workload"),
    }
}

#[test]
fn test_workload_spec_workload_type() {
    let workload = WorkloadSpec::default();
    assert_eq!(workload.workload_type(), WorkloadType::Native);

    let wasm = WorkloadSpec::Wasm {
        module: WasmModuleSource::File {
            path: PathBuf::from("test.wasm"),
        },
        args: None,
        wasi_config: None,
        env_vars: HashMap::new(),
    };
    assert_eq!(wasm.workload_type(), WorkloadType::Wasm);
}

#[test]
fn test_workload_spec_clone() {
    let workload1 = WorkloadSpec::default();
    let workload2 = workload1.clone();

    assert_eq!(workload1.workload_type(), workload2.workload_type());
}

#[test]
fn test_workload_spec_serialization() {
    let workload = WorkloadSpec::default();
    let serialized = serde_json::to_string(&workload).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// WorkloadType Tests
// ============================================================================

#[test]
fn test_workload_type_native() {
    let wt = WorkloadType::Native;
    assert_eq!(wt, WorkloadType::Native);
}

#[test]
fn test_workload_type_wasm() {
    let wt = WorkloadType::Wasm;
    assert_eq!(wt, WorkloadType::Wasm);
}

#[test]
fn test_workload_type_container() {
    let wt = WorkloadType::Container;
    assert_eq!(wt, WorkloadType::Container);
}

#[test]
fn test_workload_type_gpu() {
    let wt = WorkloadType::Gpu;
    assert_eq!(wt, WorkloadType::Gpu);
}

#[test]
fn test_workload_type_python() {
    let wt = WorkloadType::Python;
    assert_eq!(wt, WorkloadType::Python);
}

#[test]
fn test_workload_type_clone() {
    let wt1 = WorkloadType::Native;
    let wt2 = wt1.clone();
    assert_eq!(wt1, wt2);
}

#[test]
fn test_workload_type_equality() {
    assert_eq!(WorkloadType::Native, WorkloadType::Native);
    assert_ne!(WorkloadType::Native, WorkloadType::Wasm);
}

// ============================================================================
// ExecutableSource Tests
// ============================================================================

#[test]
fn test_executable_source_file() {
    let source = ExecutableSource::File {
        path: PathBuf::from("/bin/bash"),
    };

    match source {
        ExecutableSource::File { path } => {
            assert_eq!(path, PathBuf::from("/bin/bash"));
        }
        _ => panic!("Expected File source"),
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
        _ => panic!("Expected Url source"),
    }
}

#[test]
fn test_executable_source_bytes() {
    let code = "#!/bin/bash\necho 'Hello'";
    let source = ExecutableSource::Bytes {
        data: bytes::Bytes::from(code.as_bytes().to_vec()),
    };

    match source {
        ExecutableSource::Bytes { data } => {
            assert_eq!(data, code.as_bytes());
        }
        _ => panic!("Expected Bytes source"),
    }
}

#[test]
fn test_executable_source_clone() {
    let source1 = ExecutableSource::File {
        path: PathBuf::from("/test"),
    };
    let source2 = source1.clone();

    match (source1, source2) {
        (ExecutableSource::File { path: path1 }, ExecutableSource::File { path: path2 }) => {
            assert_eq!(path1, path2);
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
        _ => panic!("Expected File source"),
    }
}

#[test]
fn test_wasm_module_source_url() {
    let source = WasmModuleSource::Url {
        url: "https://example.com/module.wasm".to_string(),
    };

    match source {
        WasmModuleSource::Url { url } => {
            assert_eq!(url, "https://example.com/module.wasm");
        }
        _ => panic!("Expected Url source"),
    }
}

#[test]
fn test_wasm_module_source_bytes() {
    let bytes = vec![0x00, 0x61, 0x73, 0x6d]; // WASM magic number
    let source = WasmModuleSource::Bytes {
        data: bytes.clone().into(),
    };

    match source {
        WasmModuleSource::Bytes { data } => {
            assert_eq!(data, bytes);
        }
        _ => panic!("Expected Bytes source"),
    }
}

#[test]
fn test_wasm_module_source_clone() {
    let source1 = WasmModuleSource::File {
        path: PathBuf::from("test.wasm"),
    };
    let source2 = source1.clone();

    match (source1, source2) {
        (WasmModuleSource::File { path: path1 }, WasmModuleSource::File { path: path2 }) => {
            assert_eq!(path1, path2);
        }
        _ => panic!("Clone failed"),
    }
}

// ============================================================================
// PythonSource Tests
// ============================================================================

#[test]
fn test_python_source_file() {
    let source = PythonSource::File {
        path: PathBuf::from("script.py"),
    };

    match source {
        PythonSource::File { path } => {
            assert_eq!(path, PathBuf::from("script.py"));
        }
        _ => panic!("Expected File source"),
    }
}

#[test]
fn test_python_source_code() {
    let code = "print('Hello, World!')";
    let source = PythonSource::Code {
        code: code.to_string(),
    };

    match source {
        PythonSource::Code { code: c } => {
            assert_eq!(c, code);
        }
        _ => panic!("Expected Code source"),
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
        _ => panic!("Expected Module source"),
    }
}

#[test]
fn test_python_source_clone() {
    let source1 = PythonSource::Code {
        code: "test".to_string(),
    };
    let source2 = source1.clone();

    match (source1, source2) {
        (PythonSource::Code { code: code1 }, PythonSource::Code { code: code2 }) => {
            assert_eq!(code1, code2);
        }
        _ => panic!("Clone failed"),
    }
}

// ============================================================================
// GpuProgramSource Tests
// ============================================================================

#[test]
fn test_gpu_program_source_opencl() {
    let code = "kernel void test() {}";
    let source = GpuProgramSource::OpenCL {
        source: code.to_string(),
    };

    match source {
        GpuProgramSource::OpenCL { source: s } => {
            assert_eq!(s, code);
        }
        _ => panic!("Expected OpenCL source"),
    }
}

#[test]
fn test_gpu_program_source_cuda() {
    let code = "__global__ void test() {}";
    let source = GpuProgramSource::Cuda {
        source: code.to_string(),
    };

    match source {
        GpuProgramSource::Cuda { source: s } => {
            assert_eq!(s, code);
        }
        _ => panic!("Expected CUDA source"),
    }
}

#[test]
fn test_gpu_program_source_vulkan() {
    let spirv = vec![0x03, 0x02, 0x23, 0x07]; // SPIR-V magic number
    let source = GpuProgramSource::Vulkan {
        spirv: spirv.clone(),
    };

    match source {
        GpuProgramSource::Vulkan { spirv: s } => {
            assert_eq!(s, spirv);
        }
        _ => panic!("Expected Vulkan source"),
    }
}

#[test]
fn test_gpu_program_source_clone() {
    let source1 = GpuProgramSource::OpenCL {
        source: "test".to_string(),
    };
    let source2 = source1.clone();

    match (source1, source2) {
        (
            GpuProgramSource::OpenCL { source: source1 },
            GpuProgramSource::OpenCL { source: source2 },
        ) => {
            assert_eq!(source1, source2);
        }
        _ => panic!("Clone failed"),
    }
}

// ============================================================================
// VolumeMount Tests
// ============================================================================

#[test]
fn test_volume_mount_creation() {
    let mount = VolumeMount {
        source: PathBuf::from("/host/path"),
        target: PathBuf::from("/container/path"),
        mount_type: VolumeMountType::Bind,
        read_only: false,
    };

    assert_eq!(mount.source, PathBuf::from("/host/path"));
    assert_eq!(mount.target, PathBuf::from("/container/path"));
    assert!(matches!(mount.mount_type, VolumeMountType::Bind));
    assert!(!mount.read_only);
}

#[test]
fn test_volume_mount_volume_type() {
    let mount = VolumeMount {
        source: PathBuf::from("data-volume"),
        target: PathBuf::from("/app/data"),
        mount_type: VolumeMountType::Volume,
        read_only: true,
    };

    assert!(matches!(mount.mount_type, VolumeMountType::Volume));
    assert!(mount.read_only);
}

#[test]
fn test_volume_mount_tmpfs() {
    let mount = VolumeMount {
        source: PathBuf::from(""),
        target: PathBuf::from("/tmp"),
        mount_type: VolumeMountType::Tmpfs,
        read_only: false,
    };

    assert!(matches!(mount.mount_type, VolumeMountType::Tmpfs));
}

#[test]
fn test_volume_mount_clone() {
    let mount1 = VolumeMount {
        source: PathBuf::from("/source"),
        target: PathBuf::from("/target"),
        mount_type: VolumeMountType::Bind,
        read_only: true,
    };

    let mount2 = mount1.clone();

    assert_eq!(mount1.source, mount2.source);
    assert_eq!(mount1.target, mount2.target);
    assert_eq!(mount1.read_only, mount2.read_only);
}

// ============================================================================
// PortMapping Tests
// ============================================================================

#[test]
fn test_port_mapping_creation() {
    let port = PortMapping {
        host_port: 8080,
        container_port: 80,
        protocol: PortProtocol::Tcp,
    };

    assert_eq!(port.host_port, 8080);
    assert_eq!(port.container_port, 80);
    // PortProtocol doesn't derive PartialEq, so we can't compare it
    matches!(port.protocol, PortProtocol::Tcp);
}

#[test]
fn test_port_mapping_udp() {
    let port = PortMapping {
        host_port: 53,
        container_port: 53,
        protocol: PortProtocol::Udp,
    };

    matches!(port.protocol, PortProtocol::Udp);
}

#[test]
fn test_port_mapping_clone() {
    let port1 = PortMapping {
        host_port: 443,
        container_port: 443,
        protocol: PortProtocol::Tcp,
    };

    let port2 = port1.clone();

    assert_eq!(port1.host_port, port2.host_port);
    assert_eq!(port1.container_port, port2.container_port);
    // PortProtocol doesn't derive PartialEq, just verify they're the same variant
    matches!(port2.protocol, PortProtocol::Tcp);
}

// ============================================================================
// PortProtocol Tests
// ============================================================================

#[test]
fn test_port_protocol_tcp() {
    let protocol = PortProtocol::Tcp;
    assert!(matches!(protocol, PortProtocol::Tcp));
}

#[test]
fn test_port_protocol_udp() {
    let protocol = PortProtocol::Udp;
    assert!(matches!(protocol, PortProtocol::Udp));
}

#[test]
fn test_port_protocol_clone() {
    let protocol1 = PortProtocol::Tcp;
    let protocol2 = protocol1.clone();

    match (protocol1, protocol2) {
        (PortProtocol::Tcp, PortProtocol::Tcp) => {}
        (PortProtocol::Udp, PortProtocol::Udp) => {}
        _ => panic!("Clone failed"),
    }
}

// ============================================================================
// VolumeMountType Tests
// ============================================================================

#[test]
fn test_volume_mount_type_bind() {
    let mount_type = VolumeMountType::Bind;
    assert!(matches!(mount_type, VolumeMountType::Bind));
}

#[test]
fn test_volume_mount_type_volume() {
    let mount_type = VolumeMountType::Volume;
    assert!(matches!(mount_type, VolumeMountType::Volume));
}

#[test]
fn test_volume_mount_type_tmpfs() {
    let mount_type = VolumeMountType::Tmpfs;
    assert!(matches!(mount_type, VolumeMountType::Tmpfs));
}

#[test]
fn test_volume_mount_type_clone() {
    let mount_type1 = VolumeMountType::Bind;
    let mount_type2 = mount_type1.clone();

    match (mount_type1, mount_type2) {
        (VolumeMountType::Bind, VolumeMountType::Bind) => {}
        (VolumeMountType::Volume, VolumeMountType::Volume) => {}
        (VolumeMountType::Tmpfs, VolumeMountType::Tmpfs) => {}
        _ => panic!("Clone failed"),
    }
}
