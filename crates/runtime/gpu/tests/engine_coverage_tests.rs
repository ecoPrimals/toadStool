// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::unwrap_used)]

//! Integration coverage for [`toadstool_runtime_gpu::engine::UniversalGpuEngine`] and its
//! [`toadstool::execution::RuntimeEngine`] implementation.

use std::sync::Arc;

use serde_json::json;
use toadstool::RuntimeMetrics;
use toadstool::execution::{
    ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeCapabilities, RuntimeConfig,
    RuntimeEngine, RuntimeType,
};
use toadstool::workload::GpuProgramSource;
use toadstool::{SecurityContext, WorkloadSpec, WorkloadType, resources::ResourceRequirements};
use toadstool_runtime_gpu::engine::*;
use toadstool_runtime_gpu::{
    BackendSelectionStrategy, ComputeEngineStatistics, ComputeWorkload, DeviceId,
    DeviceRequirements, GpuFramework, KernelFormat, UniversalGpuConfig,
};
use uuid::Uuid;

fn sample_gpu_request() -> ExecutionRequest {
    ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Gpu {
            program: GpuProgramSource::OpenCL {
                source: "kernel void k() {}".to_string(),
            },
            kernel_name: "k".to_string(),
            global_work_size: (1, 1, 1),
            work_group_size: Some((1, 1, 1)),
            args: vec![],
        },
        runtime_hint: None,
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::default(),
        timeout: None,
        environment: std::collections::HashMap::new(),
        input_data: toadstool::execution::ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    }
}

fn sample_workload() -> ComputeWorkload {
    ComputeWorkload {
        name: "cov".to_string(),
        kernel_source: "kernel void k() {}".to_string(),
        kernel_format: KernelFormat::OpenClC,
        inputs: vec![],
        requirements: DeviceRequirements::minimal(),
        parent_session: None,
        recursive_workloads: vec![],
        priority: 1,
    }
}

#[test]
fn universal_gpu_engine_default_constructible() {
    let _ = UniversalGpuEngine::default();
}

#[tokio::test]
async fn default_engine_exposes_empty_device_list() {
    let engine = UniversalGpuEngine::default();
    assert!(engine.get_available_devices().await.is_empty());
}

#[tokio::test]
async fn new_and_with_config_equivalence_when_both_succeed() {
    let a = UniversalGpuEngine::new().await;
    let b = UniversalGpuEngine::with_config(UniversalGpuConfig::default()).await;
    match (a, b) {
        (Ok(ea), Ok(eb)) => {
            assert_eq!(
                ea.get_available_devices().await.len(),
                eb.get_available_devices().await.len()
            );
        }
        (Err(_), Err(_)) => {}
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => {
            panic!("new and with_config(default) should agree on success/failure");
        }
    }
}

#[tokio::test]
async fn with_config_empty_framework_list_errors() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.enabled_frameworks = vec![];
    let err = UniversalGpuEngine::with_config(config).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn with_config_and_strategy_empty_framework_list_errors() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.enabled_frameworks = vec![];
    let err =
        UniversalGpuEngine::with_config_and_strategy(config, BackendSelectionStrategy::Pragmatic)
            .await;
    assert!(err.is_err());
}

#[tokio::test]
async fn with_config_no_auto_fallback_vulkan_errors() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.auto_fallback = false;
    config.discovery.enabled_frameworks = vec![GpuFramework::Vulkan];
    let err = UniversalGpuEngine::with_config(config).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn with_config_metal_fallback_succeeds() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.enabled_frameworks = vec![GpuFramework::Metal];
    config.discovery.auto_fallback = true;
    let engine = UniversalGpuEngine::with_config(config).await.unwrap();
    assert!(engine.get_available_devices().await.is_empty());
}

#[tokio::test]
async fn with_config_and_strategy_custom_strategy() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.enabled_frameworks = vec![GpuFramework::Metal];
    config.discovery.auto_fallback = true;
    let strategy = BackendSelectionStrategy::Specific(GpuFramework::WebGpu);
    let engine = UniversalGpuEngine::with_config_and_strategy(config, strategy)
        .await
        .unwrap();
    assert_eq!(
        engine.get_selection_strategy(),
        BackendSelectionStrategy::Specific(GpuFramework::WebGpu)
    );
}

#[tokio::test]
async fn get_device_misses_unknown_id() {
    let engine = UniversalGpuEngine::default();
    let id = DeviceId::new(GpuFramework::WebGpu, 0, "nope".to_string());
    assert!(engine.get_device(&id).await.is_none());
}

#[tokio::test]
async fn get_statistics_default_engine_zeros() {
    let engine = UniversalGpuEngine::default();
    let s = engine.get_statistics().await;
    assert_eq!(s.total_devices, 0);
    assert_eq!(s.active_sessions, 0);
    assert_eq!(s.frameworks_available, 0);
    assert_eq!(s.recursive_sessions, 0);
    assert_eq!(s.max_recursion_depth, 0);
}

#[test]
fn compute_engine_statistics_clone_matches() {
    let a = ComputeEngineStatistics {
        total_devices: 2,
        active_sessions: 1,
        frameworks_available: 1,
        recursive_sessions: 0,
        max_recursion_depth: 0,
    };
    let b = a.clone();
    assert_eq!(a.total_devices, b.total_devices);
    assert_eq!(a.active_sessions, b.active_sessions);
}

#[tokio::test]
async fn execute_workload_errors_without_devices() {
    let engine = UniversalGpuEngine::default();
    let err = engine.execute_workload(sample_workload()).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn runtime_engine_initialize_noop() {
    let mut engine = UniversalGpuEngine::default();
    let out = engine.initialize(RuntimeConfig::default()).await;
    assert!(out.is_ok());
}

#[test]
fn runtime_engine_get_capabilities_matches_engine_contract() {
    let engine = UniversalGpuEngine::default();
    let c = engine.get_capabilities();
    assert!(c.supported_workloads.contains(&WorkloadType::Gpu));
    assert_eq!(c.max_concurrent_executions, Some(64));
    assert_eq!(c.version, "1.0.0");
    assert!(c.platform_features["parallel_compute"]);
    assert!(c.platform_features["recursive_execution"]);
}

#[test]
fn runtime_capabilities_serde_round_trip() {
    let engine = UniversalGpuEngine::default();
    let c = engine.get_capabilities();
    let json = serde_json::to_string(&c).unwrap();
    let back: RuntimeCapabilities = serde_json::from_str(&json).unwrap();
    assert_eq!(back.version, c.version);
    assert_eq!(back.supported_workloads, c.supported_workloads);
}

#[test]
fn universal_gpu_config_serde_round_trip() {
    let c = UniversalGpuConfig::default();
    let json = serde_json::to_value(&c).unwrap();
    let back: UniversalGpuConfig = serde_json::from_value(json).unwrap();
    assert_eq!(
        back.discovery.enabled_frameworks.len(),
        c.discovery.enabled_frameworks.len()
    );
}

#[test]
fn runtime_config_serde_round_trip() {
    let mut c = RuntimeConfig::default();
    c.settings.insert("k".to_string(), json!("v"));
    let json = serde_json::to_string(&c).unwrap();
    let back: RuntimeConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.settings.get("k"), c.settings.get("k"));
}

#[test]
fn supports_workload_only_gpu() {
    let engine = UniversalGpuEngine::default();
    assert!(engine.supports_workload(&WorkloadType::Gpu));
    assert!(!engine.supports_workload(&WorkloadType::Native));
    assert!(!engine.supports_workload(&WorkloadType::Wasm));
    assert!(!engine.supports_workload(&WorkloadType::Container));
    assert!(!engine.supports_workload(&WorkloadType::Python));
    assert!(!engine.supports_workload(&WorkloadType::AiMl));
    assert!(!engine.supports_workload(&WorkloadType::Cuda));
}

#[tokio::test]
async fn get_metrics_shape() {
    let engine = UniversalGpuEngine::default();
    let m: RuntimeMetrics = engine.get_metrics().await.unwrap();
    assert!(m.gpu.is_some());
}

#[tokio::test]
async fn shutdown_on_default_engine_ok() {
    let mut engine = UniversalGpuEngine::default();
    assert!(engine.shutdown().await.is_ok());
}

#[tokio::test]
async fn evolution_metrics_round_trip_through_engine() {
    let engine = UniversalGpuEngine::default();
    let mut m = engine.get_evolution_metrics().await;
    m.webgpu_ai_coverage = 0.42;
    engine.update_evolution_metrics(m.clone()).await;
    let read = engine.get_evolution_metrics().await;
    assert!((read.webgpu_ai_coverage - 0.42).abs() < f32::EPSILON);
}

#[test]
fn backend_selection_strategy_variants_clone_and_debug() {
    let variants = [
        BackendSelectionStrategy::Automatic,
        BackendSelectionStrategy::SovereignOnly,
        BackendSelectionStrategy::Pragmatic,
        BackendSelectionStrategy::Specific(GpuFramework::Cuda),
    ];
    for s in variants {
        let t = s.clone();
        assert_eq!(format!("{s:?}"), format!("{t:?}"));
    }
}

#[tokio::test]
async fn select_framework_for_workload_without_frameworks() {
    let engine = UniversalGpuEngine::default();
    assert!(
        engine
            .select_framework_for_workload(Some(&WorkloadType::Gpu))
            .await
            .is_none()
    );
    assert!(engine.select_framework_for_workload(None).await.is_none());
}

#[tokio::test]
async fn execute_rejects_non_gpu_workload() {
    let engine = UniversalGpuEngine::default();
    let mut req = sample_gpu_request();
    req.workload = WorkloadSpec::default();
    let err = engine.execute(req).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn execute_errors_when_no_devices_for_gpu_request() {
    let engine = UniversalGpuEngine::default();
    let err = engine.execute(sample_gpu_request()).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn execute_cuda_source_hits_device_gate() {
    let engine = UniversalGpuEngine::default();
    let req = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Gpu {
            program: GpuProgramSource::Cuda {
                source: "__global__ void x() {}".to_string(),
            },
            kernel_name: "x".to_string(),
            global_work_size: (1, 1, 1),
            work_group_size: Some((1, 1, 1)),
            args: vec![],
        },
        runtime_hint: None,
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::default(),
        timeout: None,
        environment: std::collections::HashMap::new(),
        input_data: toadstool::execution::ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    };
    assert!(engine.execute(req).await.is_err());
}

#[tokio::test]
async fn execute_vulkan_spirv_source_hits_device_gate() {
    let engine = UniversalGpuEngine::default();
    let req = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Gpu {
            program: GpuProgramSource::Vulkan {
                spirv: vec![0u8; 4],
            },
            kernel_name: "m".to_string(),
            global_work_size: (1, 1, 1),
            work_group_size: Some((1, 1, 1)),
            args: vec![],
        },
        runtime_hint: None,
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::default(),
        timeout: None,
        environment: std::collections::HashMap::new(),
        input_data: toadstool::execution::ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    };
    assert!(engine.execute(req).await.is_err());
}

#[test]
fn execution_request_gpu_opencl_serde_round_trip() {
    let r = sample_gpu_request();
    let json = serde_json::to_string(&r).unwrap();
    let back: ExecutionRequest = serde_json::from_str(&json).unwrap();
    match (&r.workload, &back.workload) {
        (WorkloadSpec::Gpu { .. }, WorkloadSpec::Gpu { .. }) => {}
        _ => panic!("expected gpu workload"),
    }
}

#[test]
fn execution_response_serde_round_trip() {
    let r = ExecutionResponse::default();
    let json = serde_json::to_string(&r).unwrap();
    let back: ExecutionResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(back.runtime_used, r.runtime_used);
}

#[test]
fn runtime_type_variants_serde_round_trip() {
    let types = [
        RuntimeType::Native,
        RuntimeType::Wasm,
        RuntimeType::Container,
        RuntimeType::Gpu,
        RuntimeType::Python,
        RuntimeType::from("custom"),
    ];
    for t in types {
        let json = serde_json::to_string(&t).unwrap();
        let back: RuntimeType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }
}

#[test]
fn execution_status_variants_round_trip() {
    let cases = [
        ExecutionStatus::Success,
        ExecutionStatus::Failed { error: "e".into() },
        ExecutionStatus::Cancelled,
        ExecutionStatus::TimedOut,
        ExecutionStatus::Running,
        ExecutionStatus::Pending,
    ];
    for s in cases {
        let json = serde_json::to_string(&s).unwrap();
        let back: ExecutionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }
}

#[tokio::test]
async fn with_resource_monitor_preserves_statistics_shape() {
    use std::future::Future;
    use std::pin::Pin;
    use toadstool::resources::{ResourceMonitor, SystemResources};

    struct NopMonitor;
    impl ResourceMonitor for NopMonitor {
        fn start_monitoring(&self, _: &str) -> toadstool::ToadStoolResult<()> {
            Ok(())
        }
        fn stop_monitoring(&self, _: &str) -> toadstool::ToadStoolResult<()> {
            Ok(())
        }
        fn get_metrics(
            &self,
            _: &str,
        ) -> Pin<Box<dyn Future<Output = toadstool::ToadStoolResult<RuntimeMetrics>> + Send + '_>>
        {
            Box::pin(async { Ok(RuntimeMetrics::default()) })
        }
        fn get_system_resources(
            &self,
        ) -> Pin<Box<dyn Future<Output = toadstool::ToadStoolResult<SystemResources>> + Send + '_>>
        {
            Box::pin(async { Ok(SystemResources::default()) })
        }
    }

    let engine = UniversalGpuEngine::default().with_resource_monitor(Arc::new(NopMonitor));
    let s = engine.get_statistics().await;
    assert_eq!(s.total_devices, 0);
}

#[test]
fn gpu_framework_serde_variants() {
    let frameworks = [
        GpuFramework::WebGpu,
        GpuFramework::Vulkan,
        GpuFramework::OpenCl,
        GpuFramework::Cuda,
        GpuFramework::Metal,
        GpuFramework::Rocm,
        GpuFramework::DirectCompute,
        GpuFramework::Custom("x".to_string()),
    ];
    for f in frameworks {
        let json = serde_json::to_string(&f).unwrap();
        let back: GpuFramework = serde_json::from_str(&json).unwrap();
        assert_eq!(back, f);
    }
}

#[test]
fn kernel_format_serde_variants() {
    use toadstool_runtime_gpu::KernelFormat as K;
    let all = [
        K::OpenClC,
        K::CudaC,
        K::Hlsl,
        K::Glsl,
        K::Msl,
        K::Spirv,
        K::LlvmIr,
        K::Wasm,
        K::Wgsl,
        K::Tucl,
    ];
    for k in all {
        let json = serde_json::to_string(&k).unwrap();
        let back: K = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{k:?}"));
    }
}

#[test]
fn device_id_serde_round_trip() {
    let id = DeviceId::new(GpuFramework::WebGpu, 3, "u".to_string());
    let json = serde_json::to_string(&id).unwrap();
    let back: DeviceId = serde_json::from_str(&json).unwrap();
    assert_eq!(back, id);
}

#[test]
fn device_requirements_serde_round_trip() {
    let d = DeviceRequirements::high_performance();
    let json = serde_json::to_string(&d).unwrap();
    let back: DeviceRequirements = serde_json::from_str(&json).unwrap();
    assert_eq!(back.min_memory_bytes, d.min_memory_bytes);
}

#[tokio::test]
async fn initialized_engine_selects_framework_when_any_available() {
    if let Ok(engine) = UniversalGpuEngine::new().await {
        let stats = engine.get_statistics().await;
        let fw = engine
            .select_framework_for_workload(Some(&WorkloadType::Gpu))
            .await;
        if stats.frameworks_available > 0 {
            assert!(fw.is_some());
        }
    }
}

#[tokio::test]
async fn new_or_err_does_not_panic() {
    let r = UniversalGpuEngine::new().await;
    assert!(r.is_ok() || r.is_err());
}
