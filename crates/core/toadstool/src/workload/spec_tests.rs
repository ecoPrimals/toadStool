// SPDX-License-Identifier: AGPL-3.0-or-later

use super::validators;
use super::*;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::NamedTempFile;

#[test]
fn test_workload_spec_default() {
    let spec = WorkloadSpec::default();
    assert_eq!(spec.workload_type(), WorkloadType::Native);
    if let WorkloadSpec::Native {
        executable, args, ..
    } = &spec
    {
        assert!(matches!(executable, ExecutableSource::File { .. }));
        assert_eq!(args.as_ref().unwrap()[0], "Hello, World!");
    }
}

#[test]
fn test_workload_type_for_all_variants() {
    let native = WorkloadSpec::Native {
        executable: ExecutableSource::Url {
            url: "https://example.com/bin".to_string(),
        },
        args: None,
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };
    assert_eq!(native.workload_type(), WorkloadType::Native);

    let wasm = WorkloadSpec::Wasm {
        module: WasmModuleSource::Bytes {
            data: bytes::Bytes::from(vec![0, 97, 115, 109]),
        },
        args: None,
        wasi_config: None,
        env_vars: HashMap::new(),
    };
    assert_eq!(wasm.workload_type(), WorkloadType::Wasm);

    let container = WorkloadSpec::Container {
        image: "ubuntu:22.04".to_string(),
        command: None,
        args: None,
        env_vars: HashMap::new(),
        working_dir: None,
        volumes: vec![],
        ports: vec![],
        registry_auth: None,
    };
    assert_eq!(container.workload_type(), WorkloadType::Container);

    let gpu = WorkloadSpec::Gpu {
        program: GpuProgramSource::OpenCL {
            source: "kernel void k() {}".to_string(),
        },
        kernel_name: "k".to_string(),
        work_group_size: None,
        global_work_size: (1, 1, 1),
        args: vec![],
    };
    assert_eq!(gpu.workload_type(), WorkloadType::Gpu);

    let python = WorkloadSpec::Python {
        source: PythonSource::Code {
            code: "print(1)".to_string(),
        },
        python_version: None,
        requirements: vec![],
        env_vars: HashMap::new(),
    };
    assert_eq!(python.workload_type(), WorkloadType::Python);

    let ai_ml = WorkloadSpec::AiMl {
        workload: ai_ml::AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Inference,
            ModelSize::Small,
            1,
        ),
    };
    assert_eq!(ai_ml.workload_type(), WorkloadType::AiMl);

    let cuda = WorkloadSpec::Cuda {
        workload: cuda::CudaWorkload::new(
            cuda::CudaSource::CudaCpp {
                source: "extern \"C\" __global__ void k() {}".to_string(),
                entry_point: "k".to_string(),
            },
            cuda::CudaLaunchConfig::new((1, 1, 1), (1, 1, 1)),
        ),
    };
    assert_eq!(cuda.workload_type(), WorkloadType::Cuda);
}

#[test]
fn test_workload_type_enum_variants() {
    assert_eq!(WorkloadType::Native, WorkloadType::Native);
    assert_ne!(WorkloadType::Native, WorkloadType::Wasm);
    let mut s = std::collections::HashSet::new();
    s.insert(WorkloadType::Native);
    s.insert(WorkloadType::Cuda);
    assert_eq!(s.len(), 2);
}

#[test]
fn test_validate_container_empty_image() {
    let spec = WorkloadSpec::Container {
        image: String::new(),
        command: None,
        args: None,
        env_vars: HashMap::new(),
        working_dir: None,
        volumes: vec![],
        ports: vec![],
        registry_auth: None,
    };
    assert!(spec.validate().is_err());
    assert!(spec.validate().unwrap_err().to_string().contains("empty"));
}

#[test]
fn test_validate_container_ok() {
    let spec = WorkloadSpec::Container {
        image: "alpine:latest".to_string(),
        command: None,
        args: None,
        env_vars: HashMap::new(),
        working_dir: None,
        volumes: vec![],
        ports: vec![],
        registry_auth: None,
    };
    assert!(spec.validate().is_ok());
}

#[test]
fn test_validate_executable_empty_url() {
    let spec = WorkloadSpec::Native {
        executable: ExecutableSource::Url { url: String::new() },
        args: None,
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };
    assert!(spec.validate().is_err());
}

#[test]
fn test_validate_executable_empty_bytes() {
    let spec = WorkloadSpec::Native {
        executable: ExecutableSource::Bytes {
            data: bytes::Bytes::new(),
        },
        args: None,
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };
    assert!(spec.validate().is_err());
}

#[test]
fn test_validate_executable_url_ok() {
    let spec = WorkloadSpec::Native {
        executable: ExecutableSource::Url {
            url: "https://example.com/app".to_string(),
        },
        args: None,
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };
    assert!(spec.validate().is_ok());
}

#[test]
fn test_validate_wasm_empty_bytes() {
    let spec = WorkloadSpec::Wasm {
        module: WasmModuleSource::Bytes {
            data: bytes::Bytes::new(),
        },
        args: None,
        wasi_config: None,
        env_vars: HashMap::new(),
    };
    assert!(spec.validate().is_err());
}

#[test]
fn test_validate_wasm_empty_url() {
    let spec = WorkloadSpec::Wasm {
        module: WasmModuleSource::Url { url: String::new() },
        args: None,
        wasi_config: None,
        env_vars: HashMap::new(),
    };
    assert!(spec.validate().is_err());
}

#[test]
fn test_validate_wasm_bytes_ok() {
    let spec = WorkloadSpec::Wasm {
        module: WasmModuleSource::Bytes {
            data: bytes::Bytes::from(vec![0, 97, 115, 109]),
        },
        args: None,
        wasi_config: None,
        env_vars: HashMap::new(),
    };
    assert!(spec.validate().is_ok());
}

#[test]
fn test_validate_gpu_empty_opencl() {
    let spec = WorkloadSpec::Gpu {
        program: GpuProgramSource::OpenCL {
            source: String::new(),
        },
        kernel_name: "k".to_string(),
        work_group_size: None,
        global_work_size: (1, 1, 1),
        args: vec![],
    };
    assert!(spec.validate().is_err());
}

#[test]
fn test_validate_gpu_empty_cuda() {
    let spec = WorkloadSpec::Gpu {
        program: GpuProgramSource::Cuda {
            source: String::new(),
        },
        kernel_name: "k".to_string(),
        work_group_size: None,
        global_work_size: (1, 1, 1),
        args: vec![],
    };
    assert!(spec.validate().is_err());
}

#[test]
fn test_validate_gpu_empty_vulkan() {
    let spec = WorkloadSpec::Gpu {
        program: GpuProgramSource::Vulkan { spirv: vec![] },
        kernel_name: "k".to_string(),
        work_group_size: None,
        global_work_size: (1, 1, 1),
        args: vec![],
    };
    assert!(spec.validate().is_err());
}

#[test]
fn test_validate_gpu_ok() {
    let spec = WorkloadSpec::Gpu {
        program: GpuProgramSource::OpenCL {
            source: "kernel void k() {}".to_string(),
        },
        kernel_name: "k".to_string(),
        work_group_size: None,
        global_work_size: (1, 1, 1),
        args: vec![],
    };
    assert!(spec.validate().is_ok());
}

#[test]
fn test_validate_python_empty_code() {
    let spec = WorkloadSpec::Python {
        source: PythonSource::Code {
            code: String::new(),
        },
        python_version: None,
        requirements: vec![],
        env_vars: HashMap::new(),
    };
    assert!(spec.validate().is_err());
}

#[test]
fn test_validate_python_empty_module() {
    let spec = WorkloadSpec::Python {
        source: PythonSource::Module {
            name: String::new(),
        },
        python_version: None,
        requirements: vec![],
        env_vars: HashMap::new(),
    };
    assert!(spec.validate().is_err());
}

#[test]
fn test_validate_python_code_ok() {
    let spec = WorkloadSpec::Python {
        source: PythonSource::Code {
            code: "print(1)".to_string(),
        },
        python_version: None,
        requirements: vec![],
        env_vars: HashMap::new(),
    };
    assert!(spec.validate().is_ok());
}

#[test]
fn test_validate_ai_ml_ok() {
    let spec = WorkloadSpec::AiMl {
        workload: ai_ml::AiMlWorkload::new(
            AiFramework::Candle,
            AiOperation::Inference,
            ModelSize::Small,
            8,
        ),
    };
    assert!(spec.validate().is_ok());
}

#[test]
fn test_validate_cuda_ok() {
    let spec = WorkloadSpec::Cuda {
        workload: cuda::CudaWorkload::new(
            cuda::CudaSource::CudaCpp {
                source: "void k() {}".to_string(),
                entry_point: "k".to_string(),
            },
            cuda::CudaLaunchConfig::new((2, 1, 1), (64, 1, 1)),
        ),
    };
    assert!(spec.validate().is_ok());
}

// --- validators.rs: validate_executable (File / Url / Bytes) ---

#[test]
fn test_validators_executable_file_not_found() {
    let path = PathBuf::from("/nonexistent/toadstool_workload_exec.bin");
    let err = validators::validate_executable(&ExecutableSource::File { path }).unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[test]
fn test_validators_executable_file_ok() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    validators::validate_executable(&ExecutableSource::File { path }).unwrap();
}

#[test]
fn test_validators_executable_url_empty() {
    assert!(
        validators::validate_executable(&ExecutableSource::Url { url: String::new() }).is_err()
    );
}

#[test]
fn test_validators_executable_url_ok() {
    validators::validate_executable(&ExecutableSource::Url {
        url: "https://example.com/bin".to_string(),
    })
    .unwrap();
}

#[test]
fn test_validators_executable_url_very_long() {
    let url = format!("https://example.com/{}", "x".repeat(16 * 1024));
    validators::validate_executable(&ExecutableSource::Url { url }).unwrap();
}

#[test]
fn test_validators_executable_bytes_empty() {
    assert!(
        validators::validate_executable(&ExecutableSource::Bytes {
            data: bytes::Bytes::new()
        })
        .is_err()
    );
}

#[test]
fn test_validators_executable_bytes_ok() {
    validators::validate_executable(&ExecutableSource::Bytes {
        data: bytes::Bytes::from_static(b"\0"),
    })
    .unwrap();
}

// --- validators.rs: validate_wasm_module (File / Bytes / Url) ---

#[test]
fn test_validators_wasm_file_not_found() {
    let path = PathBuf::from("/nonexistent/toadstool_workload.wasm");
    let err = validators::validate_wasm_module(&WasmModuleSource::File { path }).unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[test]
fn test_validators_wasm_file_ok() {
    let tmp = NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), [0x00, 0x61, 0x73, 0x6D]).unwrap();
    let path = tmp.path().to_path_buf();
    validators::validate_wasm_module(&WasmModuleSource::File { path }).unwrap();
}

#[test]
fn test_validators_wasm_bytes_empty() {
    assert!(
        validators::validate_wasm_module(&WasmModuleSource::Bytes {
            data: bytes::Bytes::new()
        })
        .is_err()
    );
}

#[test]
fn test_validators_wasm_bytes_ok() {
    validators::validate_wasm_module(&WasmModuleSource::Bytes {
        data: bytes::Bytes::from(vec![0, 97, 115, 109]),
    })
    .unwrap();
}

#[test]
fn test_validators_wasm_url_empty() {
    assert!(
        validators::validate_wasm_module(&WasmModuleSource::Url { url: String::new() }).is_err()
    );
}

#[test]
fn test_validators_wasm_url_ok() {
    validators::validate_wasm_module(&WasmModuleSource::Url {
        url: "https://cdn.example.com/mod.wasm".to_string(),
    })
    .unwrap();
}

#[test]
fn test_validators_wasm_url_very_long() {
    let url = format!("https://example.com/{}", "w".repeat(32 * 1024));
    validators::validate_wasm_module(&WasmModuleSource::Url { url }).unwrap();
}

// --- validators.rs: validate_gpu_program (OpenCL / Cuda / Vulkan) ---

#[test]
fn test_validators_gpu_opencl_ok() {
    validators::validate_gpu_program(&GpuProgramSource::OpenCL {
        source: "kernel void k() {}".to_string(),
    })
    .unwrap();
}

#[test]
fn test_validators_gpu_cuda_ok() {
    validators::validate_gpu_program(&GpuProgramSource::Cuda {
        source: "__global__ void k() {}".to_string(),
    })
    .unwrap();
}

#[test]
fn test_validators_gpu_vulkan_ok() {
    validators::validate_gpu_program(&GpuProgramSource::Vulkan {
        spirv: vec![0x03, 0x02, 0x23, 0x07],
    })
    .unwrap();
}

// --- validators.rs: validate_python_source (Code / File / Module) ---

#[test]
fn test_validators_python_code_empty() {
    assert!(
        validators::validate_python_source(&PythonSource::Code {
            code: String::new()
        })
        .is_err()
    );
}

#[test]
fn test_validators_python_code_very_long() {
    let code = "x = 1\n".repeat(4096);
    validators::validate_python_source(&PythonSource::Code { code }).unwrap();
}

#[test]
fn test_validators_python_file_not_found() {
    let path = PathBuf::from("/nonexistent/toadstool_workload_script.py");
    let err = validators::validate_python_source(&PythonSource::File { path }).unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[test]
fn test_validators_python_file_ok() {
    let tmp = NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), b"print(1)\n").unwrap();
    let path = tmp.path().to_path_buf();
    validators::validate_python_source(&PythonSource::File { path }).unwrap();
}

#[test]
fn test_validators_python_module_empty() {
    assert!(
        validators::validate_python_source(&PythonSource::Module {
            name: String::new()
        })
        .is_err()
    );
}

#[test]
fn test_validators_python_module_ok() {
    validators::validate_python_source(&PythonSource::Module {
        name: "numpy.linalg".to_string(),
    })
    .unwrap();
}

// --- WorkloadSpec::validate per variant (incl. file paths, AiMl, Cuda) ---

#[test]
fn test_validate_native_executable_file_missing() {
    let spec = WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: PathBuf::from("/nonexistent/native_bin"),
        },
        args: None,
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };
    assert!(spec.validate().is_err());
}

#[test]
fn test_validate_native_executable_file_ok() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    let spec = WorkloadSpec::Native {
        executable: ExecutableSource::File { path },
        args: None,
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };
    assert!(spec.validate().is_ok());
}

#[test]
fn test_validate_wasm_module_file_ok() {
    let tmp = NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), [0x00, 0x61, 0x73, 0x6D]).unwrap();
    let path = tmp.path().to_path_buf();
    let spec = WorkloadSpec::Wasm {
        module: WasmModuleSource::File { path },
        args: None,
        wasi_config: None,
        env_vars: HashMap::new(),
    };
    assert!(spec.validate().is_ok());
}

#[test]
fn test_validate_wasm_module_url_ok() {
    let spec = WorkloadSpec::Wasm {
        module: WasmModuleSource::Url {
            url: "https://wasm.example.com/pkg.wasm".to_string(),
        },
        args: None,
        wasi_config: None,
        env_vars: HashMap::new(),
    };
    assert!(spec.validate().is_ok());
}

#[test]
fn test_validate_gpu_cuda_and_vulkan_sources_ok() {
    let cuda_spec = WorkloadSpec::Gpu {
        program: GpuProgramSource::Cuda {
            source: "__global__ void k() {}".to_string(),
        },
        kernel_name: "k".to_string(),
        work_group_size: None,
        global_work_size: (1, 1, 1),
        args: vec![],
    };
    assert!(cuda_spec.validate().is_ok());

    let vk_spec = WorkloadSpec::Gpu {
        program: GpuProgramSource::Vulkan {
            spirv: vec![0x01; 16],
        },
        kernel_name: "main".to_string(),
        work_group_size: None,
        global_work_size: (8, 1, 1),
        args: vec![],
    };
    assert!(vk_spec.validate().is_ok());
}

#[test]
fn test_validate_python_file_and_module_ok() {
    let tmp = NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), b"# ok\n").unwrap();
    let path = tmp.path().to_path_buf();
    let file_spec = WorkloadSpec::Python {
        source: PythonSource::File { path },
        python_version: None,
        requirements: vec![],
        env_vars: HashMap::new(),
    };
    assert!(file_spec.validate().is_ok());

    let mod_spec = WorkloadSpec::Python {
        source: PythonSource::Module {
            name: "mymod".to_string(),
        },
        python_version: Some("3.12".to_string()),
        requirements: vec![],
        env_vars: HashMap::new(),
    };
    assert!(mod_spec.validate().is_ok());
}

#[test]
fn test_validate_aiml_training_variant() {
    let spec = WorkloadSpec::AiMl {
        workload: ai_ml::AiMlWorkload::new(
            AiFramework::TensorFlow,
            AiOperation::Training,
            ModelSize::Medium,
            4,
        ),
    };
    assert!(spec.validate().is_ok());
}

#[test]
fn test_validate_cuda_ptx_source() {
    let spec = WorkloadSpec::Cuda {
        workload: cuda::CudaWorkload::new(
            cuda::CudaSource::Ptx {
                source: ".version 7.0".to_string(),
                entry_point: "k".to_string(),
            },
            cuda::CudaLaunchConfig::new((4, 1, 1), (256, 1, 1)),
        ),
    };
    assert!(spec.validate().is_ok());
}

#[test]
fn test_volume_mount_type_serialization() {
    let bind = VolumeMountType::Bind;
    let json = serde_json::to_string(&bind).unwrap();
    let parsed: VolumeMountType = serde_json::from_str(&json).unwrap();
    assert_eq!(bind, parsed);
}

#[test]
fn test_port_protocol_serialization() {
    for proto in [PortProtocol::Tcp, PortProtocol::Udp] {
        let json = serde_json::to_string(&proto).unwrap();
        let parsed: PortProtocol = serde_json::from_str(&json).unwrap();
        assert_eq!(proto, parsed);
    }
}

#[test]
fn test_workload_type_serialization() {
    for wtype in [
        WorkloadType::Native,
        WorkloadType::Wasm,
        WorkloadType::Container,
        WorkloadType::Gpu,
        WorkloadType::Python,
        WorkloadType::AiMl,
        WorkloadType::Cuda,
    ] {
        let json = serde_json::to_string(&wtype).unwrap();
        let parsed: WorkloadType = serde_json::from_str(&json).unwrap();
        assert_eq!(wtype, parsed);
    }
}

#[test]
fn test_gpu_argument_variants() {
    let buf = GpuArgument::Buffer {
        data: vec![1, 2, 3],
    };
    let json_buf = serde_json::to_string(&buf).unwrap();
    let parsed: GpuArgument = serde_json::from_str(&json_buf).unwrap();
    assert!(matches!(parsed, GpuArgument::Buffer { data } if data == vec![1, 2, 3]));
}

#[test]
fn test_wasi_config_serialization() {
    let config = WasiConfig {
        inherit_env: true,
        inherit_stdio: false,
        allowed_dirs: vec![PathBuf::from("/tmp")],
        preopened_dirs: vec![],
        args: vec!["a".to_string()],
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: WasiConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.inherit_env, parsed.inherit_env);
    assert_eq!(config.args, parsed.args);
}
