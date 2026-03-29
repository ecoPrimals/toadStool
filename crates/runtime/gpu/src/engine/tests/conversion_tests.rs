// SPDX-License-Identifier: AGPL-3.0-only
//! Tests for `ExecutionRequest` → `ComputeWorkload` conversion.

use super::*;

#[test]
fn test_convert_request_to_workload_opencl() {
    use toadstool::workload::GpuProgramSource;
    let request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Gpu {
            program: GpuProgramSource::OpenCL {
                source: "void kernel main() {}".to_string(),
            },
            kernel_name: "main".to_string(),
            global_work_size: (1, 1, 1),
            work_group_size: Some((1, 1, 1)),
            args: vec![],
        },
        runtime_hint: None,
        resources: toadstool::resources::ResourceRequirements::default(),
        security_context: toadstool::SecurityContext::default(),
        timeout: None,
        environment: std::collections::HashMap::new(),
        input_data: toadstool::execution::ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    };
    let result = UniversalGpuEngine::convert_request_to_workload(&request);
    assert!(result.is_ok());
    let workload = result.unwrap();
    assert_eq!(workload.kernel_source, "void kernel main() {}");
}

#[test]
fn test_convert_request_to_workload_cuda() {
    use toadstool::workload::GpuProgramSource;
    let request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Gpu {
            program: GpuProgramSource::Cuda {
                source: "__global__ void kernel() {}".to_string(),
            },
            kernel_name: "kernel".to_string(),
            global_work_size: (1, 1, 1),
            work_group_size: Some((1, 1, 1)),
            args: vec![],
        },
        runtime_hint: None,
        resources: toadstool::resources::ResourceRequirements::default(),
        security_context: toadstool::SecurityContext::default(),
        timeout: None,
        environment: std::collections::HashMap::new(),
        input_data: toadstool::execution::ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    };
    let result = UniversalGpuEngine::convert_request_to_workload(&request);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().kernel_source, "__global__ void kernel() {}");
}

#[test]
fn test_convert_request_to_workload_vulkan_spirv() {
    use toadstool::workload::GpuProgramSource;
    let spirv_bytes = vec![0x03, 0x02, 0x23, 0x07, 0x00, 0x00, 0x01, 0x00];
    let request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Gpu {
            program: GpuProgramSource::Vulkan {
                spirv: spirv_bytes.clone(),
            },
            kernel_name: "main".to_string(),
            global_work_size: (1, 1, 1),
            work_group_size: Some((1, 1, 1)),
            args: vec![],
        },
        runtime_hint: None,
        resources: toadstool::resources::ResourceRequirements::default(),
        security_context: toadstool::SecurityContext::default(),
        timeout: None,
        environment: std::collections::HashMap::new(),
        input_data: toadstool::execution::ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    };
    let result = UniversalGpuEngine::convert_request_to_workload(&request);
    assert!(result.is_ok());
    let workload = result.unwrap();
    assert!(workload.kernel_source.contains("SPIR-V binary"));
    assert!(workload.kernel_source.contains("8 bytes"));
}

#[test]
fn test_convert_request_to_workload_non_gpu_fails() {
    use toadstool::workload::ExecutableSource;
    let request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: std::path::PathBuf::from("/bin/echo"),
            },
            args: Some(vec!["hello".to_string()]),
            working_dir: None,
            env_vars: std::collections::HashMap::new(),
            user: None,
        },
        runtime_hint: None,
        resources: toadstool::resources::ResourceRequirements::default(),
        security_context: toadstool::SecurityContext::default(),
        timeout: None,
        environment: std::collections::HashMap::new(),
        input_data: toadstool::execution::ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    };
    let result = UniversalGpuEngine::convert_request_to_workload(&request);
    assert!(result.is_err());
}

#[test]
fn test_convert_request_to_workload_with_errors_in_output() {
    use toadstool::workload::GpuProgramSource;
    let request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Gpu {
            program: GpuProgramSource::OpenCL {
                source: "kernel void main() {}".to_string(),
            },
            kernel_name: "main".to_string(),
            global_work_size: (1, 1, 1),
            work_group_size: Some((1, 1, 1)),
            args: vec![],
        },
        runtime_hint: None,
        resources: toadstool::resources::ResourceRequirements::default(),
        security_context: toadstool::SecurityContext::default(),
        timeout: None,
        environment: std::collections::HashMap::new(),
        input_data: toadstool::execution::ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    };
    let workload = UniversalGpuEngine::convert_request_to_workload(&request).unwrap();
    assert!(workload.recursive_workloads.is_empty());
    assert_eq!(workload.priority, 1);
}
