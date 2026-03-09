// SPDX-License-Identifier: AGPL-3.0-only
//! Coverage tests for runtime/gpu/src/engine/mod.rs
//!
//! Focus: GPU engine creation, configuration, capability queries, error handling.
//! Uses UniversalGpuEngine::default() to avoid GPU initialization (avoids SIGSEGV on headless CI).

#![allow(clippy::pedantic)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use toadstool::execution::{ExecutionRequest, RuntimeConfig, RuntimeEngine};
use toadstool::{SecurityContext, WorkloadSpec, WorkloadType};
use toadstool_runtime_gpu::config::{GpuDiscoveryConfig, UniversalGpuConfig};
use toadstool_runtime_gpu::engine::UniversalGpuEngine;
use toadstool_runtime_gpu::strategy::{BackendSelectionStrategy, EvolutionMetrics};
use toadstool_runtime_gpu::types::{
    ComputeWorkload, DeviceId, DeviceRequirements, GpuFramework, KernelFormat,
};
use uuid::Uuid;

// -----------------------------------------------------------------------------
// Engine creation and configuration
// -----------------------------------------------------------------------------

#[tokio::test]
async fn engine_default_creation() {
    let engine = UniversalGpuEngine::default();
    let stats = engine.get_statistics().await;
    assert_eq!(stats.total_devices, 0);
    assert_eq!(stats.active_sessions, 0);
    assert_eq!(stats.frameworks_available, 0);
}

#[test]
fn engine_config_default_values() {
    let config = UniversalGpuConfig::default();
    assert!(!config.discovery.enabled_frameworks.is_empty());
    assert!(config.discovery.auto_fallback);
    assert!(config.discovery.discovery_timeout > Duration::ZERO);
}

#[test]
fn engine_config_serialization_roundtrip() {
    let config = UniversalGpuConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let restored: UniversalGpuConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(
        config.discovery.enabled_frameworks.len(),
        restored.discovery.enabled_frameworks.len()
    );
}

#[test]
fn engine_discovery_config() {
    let discovery = GpuDiscoveryConfig {
        enabled_frameworks: vec![GpuFramework::WebGpu, GpuFramework::Metal],
        auto_fallback: false,
        discovery_timeout: Duration::from_secs(5),
        min_requirements: DeviceRequirements::minimal(),
    };
    assert_eq!(discovery.enabled_frameworks.len(), 2);
    assert!(!discovery.auto_fallback);
}

#[tokio::test]
async fn engine_with_config_and_strategy() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.enabled_frameworks = vec![GpuFramework::Metal];
    config.discovery.auto_fallback = true;
    let strategy = BackendSelectionStrategy::default();
    let result = UniversalGpuEngine::with_config_and_strategy(config, strategy).await;
    assert!(result.is_ok());
    let engine = result.unwrap();
    let devices = engine.get_available_devices().await;
    assert!(devices.is_empty());
}

// -----------------------------------------------------------------------------
// Capability queries
// -----------------------------------------------------------------------------

#[test]
fn engine_get_capabilities() {
    let engine = UniversalGpuEngine::default();
    let caps = engine.get_capabilities();
    assert!(caps.supported_workloads.contains(&WorkloadType::Gpu));
    assert_eq!(caps.max_concurrent_executions, Some(64));
    assert_eq!(caps.version, "1.0.0");
}

#[test]
fn engine_capabilities_platform_features() {
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
    assert!(caps
        .platform_features
        .get("multi_framework")
        .copied()
        .unwrap_or(false));
    assert!(caps.supported_architectures.contains(&"x86_64".to_string()));
}

#[test]
fn engine_supports_workload() {
    let engine = UniversalGpuEngine::default();
    assert!(engine.supports_workload(&WorkloadType::Gpu));
    assert!(!engine.supports_workload(&WorkloadType::Native));
    assert!(!engine.supports_workload(&WorkloadType::Wasm));
}

// -----------------------------------------------------------------------------
// Shader compilation / workload execution (error paths)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn engine_execute_workload_no_devices_error() {
    let engine = UniversalGpuEngine::default();
    let workload = ComputeWorkload {
        name: "test-kernel".to_string(),
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
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("No devices") || err.to_string().contains("device"),
        "expected no devices error: {}",
        err
    );
}

#[tokio::test]
async fn engine_execute_non_gpu_request_error() {
    use toadstool::workload::ExecutableSource;
    let engine = UniversalGpuEngine::default();
    let request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: std::path::PathBuf::from("/bin/echo"),
            },
            args: Some(vec!["hello".to_string()]),
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        },
        runtime_hint: None,
        resources: toadstool::resources::ResourceRequirements::default(),
        security_context: SecurityContext::default(),
        timeout: None,
        environment: HashMap::new(),
        input_data: toadstool::execution::ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    };
    let result = engine.execute(request).await;
    assert!(result.is_err());
}

// -----------------------------------------------------------------------------
// Error handling
// -----------------------------------------------------------------------------

#[tokio::test]
async fn engine_get_device_nonexistent() {
    let engine = UniversalGpuEngine::default();
    let device_id = DeviceId::new(GpuFramework::WebGpu, 999, "nonexistent".to_string());
    assert!(engine.get_device(&device_id).await.is_none());
}

#[tokio::test]
async fn engine_shutdown_empty_ok() {
    let mut engine = UniversalGpuEngine::default();
    let result = engine.shutdown().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn engine_initialize_idempotent() {
    let mut engine = UniversalGpuEngine::default();
    let r1 = engine.initialize(RuntimeConfig::default()).await;
    let r2 = engine.initialize(RuntimeConfig::default()).await;
    assert!(r1.is_ok());
    assert!(r2.is_ok());
}

#[tokio::test]
async fn engine_get_metrics() {
    let engine = UniversalGpuEngine::default();
    let result = engine.get_metrics().await;
    assert!(result.is_ok());
    let metrics = result.unwrap();
    assert!(metrics.gpu.is_some());
}

// -----------------------------------------------------------------------------
// Evolution metrics and strategy
// -----------------------------------------------------------------------------

#[tokio::test]
async fn engine_evolution_metrics() {
    let engine = UniversalGpuEngine::default();
    let metrics = engine.get_evolution_metrics().await;
    assert!(metrics.webgpu_ai_coverage >= 0.0);
}

#[tokio::test]
async fn engine_update_evolution_metrics() {
    let engine = UniversalGpuEngine::default();
    let mut metrics = engine.get_evolution_metrics().await;
    metrics.webgpu_ai_coverage = 0.75;
    engine.update_evolution_metrics(metrics.clone()).await;
    let updated = engine.get_evolution_metrics().await;
    assert!((updated.webgpu_ai_coverage - 0.75).abs() < 1e-6);
}

#[test]
fn engine_get_selection_strategy() {
    let engine = UniversalGpuEngine::default();
    let strategy = engine.get_selection_strategy();
    assert!(matches!(
        strategy,
        BackendSelectionStrategy::Automatic | BackendSelectionStrategy::SovereignOnly
    ));
}

#[tokio::test]
async fn engine_select_framework_empty_devices() {
    let engine = UniversalGpuEngine::default();
    let framework = engine
        .select_framework_for_workload(Some(&WorkloadType::Gpu))
        .await;
    assert!(framework.is_none());
}

#[tokio::test]
async fn engine_select_framework_none_workload() {
    let engine = UniversalGpuEngine::default();
    let framework = engine.select_framework_for_workload(None).await;
    assert!(framework.is_none());
}

// -----------------------------------------------------------------------------
// Resource monitor
// -----------------------------------------------------------------------------

#[tokio::test]
async fn engine_with_resource_monitor() {
    use std::future::Future;
    use std::pin::Pin;
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
            Box::pin(async { Ok(RuntimeMetrics::default()) })
        }
        fn get_system_resources(
            &self,
        ) -> Pin<Box<dyn Future<Output = toadstool::ToadStoolResult<SystemResources>> + Send + '_>>
        {
            Box::pin(async { Ok(SystemResources::default()) })
        }
    }
    let engine = UniversalGpuEngine::default().with_resource_monitor(Arc::new(MockMonitor));
    let stats = engine.get_statistics().await;
    assert_eq!(stats.total_devices, 0);
}

// -----------------------------------------------------------------------------
// ComputeWorkload and KernelFormat
// -----------------------------------------------------------------------------

#[test]
fn compute_workload_creation() {
    let workload = ComputeWorkload {
        name: "vec_add".to_string(),
        kernel_source: "kernel void add() {}".to_string(),
        kernel_format: KernelFormat::Wgsl,
        inputs: vec![],
        requirements: DeviceRequirements::minimal(),
        parent_session: None,
        recursive_workloads: vec![],
        priority: 2,
    };
    assert_eq!(workload.name, "vec_add");
    assert!(format!("{:?}", workload.kernel_format).contains("Wgsl"));
    assert_eq!(workload.priority, 2);
}

#[test]
fn kernel_format_variants() {
    let fmt = KernelFormat::OpenClC;
    let debug_str = format!("{fmt:?}");
    assert!(debug_str.contains("OpenCl") || debug_str.contains("OpenCL"));
    let fmt2 = KernelFormat::Wgsl;
    let debug_str2 = format!("{fmt2:?}");
    assert!(debug_str2.contains("Wgsl") || debug_str2.contains("WGSL"));
}

#[test]
fn evolution_metrics_default() {
    let metrics = EvolutionMetrics::default();
    assert!(metrics.webgpu_ai_coverage >= 0.0);
}
