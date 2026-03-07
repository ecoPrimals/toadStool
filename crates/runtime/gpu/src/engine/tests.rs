// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use toadstool::execution::RuntimeConfig;

#[tokio::test]
async fn test_engine_default() {
    let engine = UniversalGpuEngine::default();
    let devices = engine.get_available_devices().await;
    assert!(devices.is_empty());
}

#[tokio::test]
async fn test_engine_get_device_none() {
    let engine = UniversalGpuEngine::default();
    let device_id = DeviceId::new(GpuFramework::WebGpu, 0, "test-uuid".to_string());
    assert!(engine.get_device(&device_id).await.is_none());
}

#[tokio::test]
async fn test_engine_get_statistics_empty() {
    let engine = UniversalGpuEngine::default();
    let stats = engine.get_statistics().await;
    assert_eq!(stats.total_devices, 0);
    assert_eq!(stats.active_sessions, 0);
    assert_eq!(stats.frameworks_available, 0);
    assert_eq!(stats.recursive_sessions, 0);
    assert_eq!(stats.max_recursion_depth, 0);
}

#[test]
fn test_engine_get_capabilities() {
    let engine = UniversalGpuEngine::default();
    let caps = engine.get_capabilities();
    assert!(caps.supported_workloads.contains(&WorkloadType::Gpu));
    assert_eq!(caps.max_concurrent_executions, Some(64));
    assert!(caps
        .platform_features
        .get("parallel_compute")
        .copied()
        .unwrap_or(false));
}

#[test]
fn test_engine_supports_workload() {
    let engine = UniversalGpuEngine::default();
    assert!(engine.supports_workload(&WorkloadType::Gpu));
    assert!(!engine.supports_workload(&WorkloadType::Wasm));
}

#[tokio::test]
async fn test_engine_get_evolution_metrics() {
    let engine = UniversalGpuEngine::default();
    let metrics = engine.get_evolution_metrics().await;
    assert!(metrics.webgpu_ai_coverage >= 0.0);
}

#[tokio::test]
async fn test_engine_update_evolution_metrics() {
    let engine = UniversalGpuEngine::default();
    let mut metrics = engine.get_evolution_metrics().await;
    metrics.webgpu_ai_coverage = 0.5;
    engine.update_evolution_metrics(metrics.clone()).await;
    let updated = engine.get_evolution_metrics().await;
    assert!((updated.webgpu_ai_coverage - 0.5).abs() < 1e-6);
}

#[test]
fn test_engine_get_selection_strategy() {
    let engine = UniversalGpuEngine::default();
    let _strategy = engine.get_selection_strategy();
}

#[tokio::test]
async fn test_engine_select_framework_empty() {
    let engine = UniversalGpuEngine::default();
    let framework = engine
        .select_framework_for_workload(Some(&WorkloadType::Gpu))
        .await;
    assert!(framework.is_none());
}

#[tokio::test]
async fn test_engine_initialize() {
    let mut engine = UniversalGpuEngine::default();
    let result = engine.initialize(RuntimeConfig::default()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_engine_get_metrics() {
    let engine = UniversalGpuEngine::default();
    let result = engine.get_metrics().await;
    assert!(result.is_ok());
    let metrics = result.unwrap();
    assert!(metrics.gpu.is_some());
}

#[tokio::test]
async fn test_engine_execute_workload_no_devices() {
    let engine = UniversalGpuEngine::default();
    let workload = ComputeWorkload {
        name: "test".to_string(),
        kernel_source: "void kernel main() {}".to_string(),
        kernel_format: KernelFormat::OpenClC,
        inputs: vec![],
        requirements: DeviceRequirements::minimal(),
        parent_session: None,
        recursive_workloads: vec![],
        priority: 1,
    };
    let result = engine.execute_workload(workload).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_engine_shutdown_empty() {
    let mut engine = UniversalGpuEngine::default();
    let result = engine.shutdown().await;
    assert!(result.is_ok());
}

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

#[tokio::test]
async fn test_engine_execute_non_gpu_workload_fails() {
    use toadstool::workload::ExecutableSource;
    let engine = UniversalGpuEngine::default();
    let request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: std::path::PathBuf::from("/bin/echo"),
            },
            args: Some(vec!["test".to_string()]),
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
    let result = engine.execute(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_engine_select_framework_none_workload() {
    let engine = UniversalGpuEngine::default();
    let framework = engine.select_framework_for_workload(None).await;
    assert!(framework.is_none());
}

#[tokio::test]
async fn test_engine_with_config_and_strategy() {
    use crate::strategy::BackendSelectionStrategy;
    let mut config = UniversalGpuConfig::default();
    config.discovery.enabled_frameworks = vec![GpuFramework::Metal];
    config.discovery.auto_fallback = true;
    let strategy = BackendSelectionStrategy::default();
    let result = UniversalGpuEngine::with_config_and_strategy(config, strategy).await;
    assert!(result.is_ok());
    let engine = result.unwrap();
    let devices = engine.get_available_devices().await;
    assert!(devices.is_empty(), "FallbackFramework returns no devices");
}

#[tokio::test]
async fn test_engine_with_resource_monitor() {
    use toadstool::resources::{ResourceMonitor, RuntimeMetrics, SystemResources};
    struct MockMonitor;
    impl ResourceMonitor for MockMonitor {
        fn start_monitoring(&self, _workload_id: &str) -> toadstool::ToadStoolResult<()> {
            Ok(())
        }
        fn stop_monitoring(&self, _workload_id: &str) -> toadstool::ToadStoolResult<()> {
            Ok(())
        }
        fn get_metrics(
            &self,
            _workload_id: &str,
        ) -> Pin<Box<dyn Future<Output = toadstool::ToadStoolResult<RuntimeMetrics>> + Send + '_>>
        {
            Box::pin(async { Ok(toadstool::resources::RuntimeMetrics::default()) })
        }
        fn get_system_resources(
            &self,
        ) -> Pin<Box<dyn Future<Output = toadstool::ToadStoolResult<SystemResources>> + Send + '_>>
        {
            Box::pin(async { Ok(toadstool::resources::SystemResources::default()) })
        }
    }
    let engine = UniversalGpuEngine::default().with_resource_monitor(Arc::new(MockMonitor));
    let stats = engine.get_statistics().await;
    assert_eq!(stats.total_devices, 0);
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

#[tokio::test]
async fn test_engine_auto_fallback_on_framework_failure() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.auto_fallback = true;
    config.discovery.enabled_frameworks = vec![
        GpuFramework::Vulkan,
        GpuFramework::OpenCl,
        GpuFramework::Metal,
    ];
    let result = UniversalGpuEngine::with_config(config).await;
    assert!(result.is_ok(), "Should fallback when Vulkan/OpenCL fail");
    let engine = result.unwrap();
    let frameworks = engine.get_statistics().await;
    assert!(frameworks.frameworks_available >= 1);
}

#[tokio::test]
async fn test_engine_no_fallback_fails_on_first_error() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.auto_fallback = false;
    config.discovery.enabled_frameworks = vec![GpuFramework::Vulkan];
    let result = UniversalGpuEngine::with_config(config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_engine_execute_gpu_workload_no_devices() {
    use toadstool::workload::GpuProgramSource;
    let engine = UniversalGpuEngine::default();
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
    let result = engine.execute(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_engine_get_statistics_with_sessions() {
    let engine = UniversalGpuEngine::default();
    let stats = engine.get_statistics().await;
    assert_eq!(stats.total_devices, 0);
    assert_eq!(stats.active_sessions, 0);
    assert_eq!(stats.frameworks_available, 0);
}

#[test]
fn test_engine_capabilities_platform_features() {
    let engine = UniversalGpuEngine::default();
    let caps = engine.get_capabilities();
    assert!(caps
        .platform_features
        .get("parallel_compute")
        .copied()
        .unwrap_or(false));
    assert!(caps
        .platform_features
        .get("recursive_execution")
        .copied()
        .unwrap_or(false));
    assert!(caps.supported_architectures.contains(&"x86_64".to_string()));
}

#[test]
fn test_engine_capabilities_version() {
    let engine = UniversalGpuEngine::default();
    let caps = engine.get_capabilities();
    assert_eq!(caps.version, "1.0.0");
}

#[tokio::test]
async fn test_engine_with_metal_only() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.enabled_frameworks = vec![GpuFramework::Metal];
    config.discovery.auto_fallback = true;
    let result = UniversalGpuEngine::with_config(config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_engine_with_rocm_only() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.enabled_frameworks = vec![GpuFramework::Rocm];
    config.discovery.auto_fallback = true;
    let result = UniversalGpuEngine::with_config(config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_engine_with_direct_compute_only() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.enabled_frameworks = vec![GpuFramework::DirectCompute];
    config.discovery.auto_fallback = true;
    let result = UniversalGpuEngine::with_config(config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_engine_with_cuda_only() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.enabled_frameworks = vec![GpuFramework::Cuda];
    config.discovery.auto_fallback = true;
    let result = UniversalGpuEngine::with_config(config).await;
    assert!(result.is_ok());
}

#[test]
fn test_engine_default_has_capabilities() {
    let engine = UniversalGpuEngine::default();
    let caps = engine.get_capabilities();
    assert!(!caps.supported_workloads.is_empty());
    assert!(caps.max_concurrent_executions.is_some());
}
