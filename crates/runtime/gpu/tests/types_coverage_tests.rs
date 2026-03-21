// SPDX-License-Identifier: AGPL-3.0-only
//! Integration tests targeting [`toadstool_runtime_gpu::types`] (`types.rs`).
//!
//! Covers `Default`, `Debug`, serde round-trips, helpers (`name`, `is_universal`,
//! `platform_compatibility`, `DeviceId::new`, `DeviceRequirements::minimal` /
//! `high_performance`), `Hash`/`Eq` where implemented, and non-serde structs
//! (`Clone`, `Debug`, construction). `types.rs` does not implement `Display`,
//! `From`/`Into`, or ordering traits on these types.

#![allow(clippy::float_cmp, clippy::unwrap_used)]

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;
use uuid::Uuid;

use toadstool_runtime_gpu::*;

fn assert_json_roundtrip<T>(v: &T)
where
    T: Serialize + DeserializeOwned,
{
    let json = serde_json::to_string(v).unwrap();
    let back: T = serde_json::from_str(&json).unwrap();
    assert_eq!(
        serde_json::to_value(v).unwrap(),
        serde_json::to_value(&back).unwrap(),
    );
}

fn sample_performance() -> PerformanceCharacteristics {
    PerformanceCharacteristics {
        peak_gflops_fp32: 1.0e4,
        peak_gflops_fp64: Some(5.0e3),
        peak_gflops_fp16: Some(2.0e4),
        peak_memory_bandwidth_utilization: 0.85,
        typical_power_watts: 200.0,
        max_power_watts: 350.0,
    }
}

fn sample_device_capabilities() -> DeviceCapabilities {
    let mut extensions = HashMap::new();
    extensions.insert("ext1".to_string(), true);
    extensions.insert("ext2".to_string(), false);
    DeviceCapabilities {
        compute_capability: "8.6".to_string(),
        total_memory_bytes: 16 * 1024 * 1024 * 1024,
        memory_bandwidth_gbps: 936.0,
        compute_units: 128,
        max_work_group_size: (1024, 1024, 64),
        supported_data_types: vec![DataType::Float32, DataType::Float64],
        extensions,
        performance: sample_performance(),
    }
}

fn sample_device_info() -> DeviceInfo {
    DeviceInfo {
        name: "Test GPU".to_string(),
        vendor: "TestVendor".to_string(),
        device_type: DeviceType::DiscreteGpu,
        driver_version: "1.0".to_string(),
        architecture: "test-arch".to_string(),
        physical_location: Some("bus:0".to_string()),
    }
}

// --- Default ---

#[test]
fn device_usage_default_matches_fields() {
    let u = DeviceUsage::default();
    assert_eq!(u.gpu_utilization_percent, 0.0);
    assert_eq!(u.memory_utilization_percent, 0.0);
    assert_eq!(u.memory_used_bytes, 0);
    assert!(u.temperature_celsius.is_none());
    assert!(u.power_usage_watts.is_none());
    assert_eq!(u.active_sessions, 0);
}

// --- Debug (Display is not implemented in types.rs) ---

#[test]
fn debug_all_public_types_non_empty() {
    let fw = GpuFramework::Custom("plugin".to_string());
    assert!(!format!("{fw:?}").is_empty());

    let id = DeviceId::new(GpuFramework::Vulkan, 2, "uuid".into());
    assert!(!format!("{id:?}").is_empty());

    let usage = Arc::new(RwLock::new(DeviceUsage::default()));
    let ucd = UniversalComputeDevice {
        id: id.clone(),
        info: sample_device_info(),
        capabilities: sample_device_capabilities(),
        usage,
        framework_handle: Some(FrameworkHandle::Unavailable {
            name: "n".into(),
            reason: "r".into(),
        }),
    };
    assert!(!format!("{ucd:?}").is_empty());

    let cap = sample_device_capabilities();
    assert!(!format!("{cap:?}").is_empty());

    let perf = sample_performance();
    assert!(!format!("{perf:?}").is_empty());

    let dt = DeviceType::Other("misc".into());
    assert!(!format!("{dt:?}").is_empty());

    let dtype = DataType::Custom("myfp".into());
    assert!(!format!("{dtype:?}").is_empty());

    let fh = FrameworkHandle::Unavailable {
        name: "x".into(),
        reason: "y".into(),
    };
    assert!(!format!("{fh:?}").is_empty());

    let sess = ComputeSession {
        id: Uuid::nil(),
        device_id: id,
        parent_session: Some(Uuid::from_u128(u128::MAX)),
        child_sessions: vec![Uuid::nil()],
        recursion_depth: 3,
        start_time: Instant::now(),
        resource_allocation: ResourceAllocation {
            memory_bytes: 1,
            compute_units: 2,
            priority: 0,
        },
        status: SessionStatus::Failed("e".into()),
    };
    assert!(!format!("{sess:?}").is_empty());

    let kf = KernelFormat::Tucl;
    assert!(!format!("{kf:?}").is_empty());

    let ck = CompiledKernel {
        id: "kid".into(),
        binary: Bytes::from_static(b"bin"),
        framework: GpuFramework::WebGpu,
        compiled_at: Instant::now(),
        optimization_level: OptimizationLevel::Adaptive,
        resource_requirements: ResourceAllocation {
            memory_bytes: 64,
            compute_units: 1,
            priority: 0,
        },
    };
    assert!(!format!("{ck:?}").is_empty());

    let ki = KernelInput {
        name: "in".into(),
        data: Bytes::from_static(b"d"),
        data_type: DataType::Float32,
        access_pattern: AccessPattern::ReadWrite,
    };
    assert!(!format!("{ki:?}").is_empty());

    let ko = KernelOutput {
        buffers: HashMap::from([("a".into(), Bytes::from_static(b"x"))]),
        metrics: ExecutionMetrics {
            execution_time: Duration::from_millis(1),
            memory_used: 0,
            compute_units_used: 1,
            energy_consumed: Some(1.0),
            throughput: Some(ThroughputMetrics {
                ops_per_second: 1.0,
                bytes_per_second: 2.0,
                memory_bandwidth_utilization: 0.5,
            }),
        },
        errors: vec!["err".into()],
    };
    assert!(!format!("{ko:?}").is_empty());

    let em = ExecutionMetrics {
        execution_time: Duration::ZERO,
        memory_used: 0,
        compute_units_used: 0,
        energy_consumed: None,
        throughput: None,
    };
    assert!(!format!("{em:?}").is_empty());

    let cw = ComputeWorkload {
        name: "w".into(),
        kernel_source: "src".into(),
        kernel_format: KernelFormat::Wgsl,
        inputs: vec![],
        requirements: DeviceRequirements::minimal(),
        parent_session: None,
        recursive_workloads: vec![],
        priority: 0,
    };
    assert!(!format!("{cw:?}").is_empty());

    let cr = ComputeResult {
        session_id: Uuid::nil(),
        device_id: DeviceId::new(GpuFramework::OpenCl, 0, "u".into()),
        primary_output: ko.clone(),
        recursive_results: vec![],
        total_execution_time: Duration::from_secs(1),
    };
    assert!(!format!("{cr:?}").is_empty());

    let ces = ComputeEngineStatistics {
        total_devices: 0,
        active_sessions: 0,
        frameworks_available: 0,
        recursive_sessions: 0,
        max_recursion_depth: 0,
    };
    assert!(!format!("{ces:?}").is_empty());

    let rp = ResourcePool {
        total_memory: 1,
        allocated_memory: 0,
        total_compute_units: 8,
        allocated_compute_units: 0,
        allocation_queue: vec![],
    };
    assert!(!format!("{rp:?}").is_empty());
}

// --- Serde round-trips ---

#[test]
fn serde_gpu_framework_all_variants() {
    let variants = vec![
        GpuFramework::WebGpu,
        GpuFramework::Vulkan,
        GpuFramework::OpenCl,
        GpuFramework::Cuda,
        GpuFramework::Metal,
        GpuFramework::Rocm,
        GpuFramework::DirectCompute,
        GpuFramework::Custom("unicode-α".into()),
    ];
    for fw in &variants {
        let json = serde_json::to_string(fw).unwrap();
        let back: GpuFramework = serde_json::from_str(&json).unwrap();
        assert_eq!(*fw, back);
    }
}

#[test]
fn serde_device_id_roundtrip() {
    let id = DeviceId::new(GpuFramework::Metal, 7, "device-uuid".into());
    let json = serde_json::to_string(&id).unwrap();
    let back: DeviceId = serde_json::from_str(&json).unwrap();
    assert_eq!(id, back);
}

#[test]
fn serde_device_info_roundtrip() {
    assert_json_roundtrip(&sample_device_info());
}

#[test]
fn serde_device_type_all_variants() {
    let variants = vec![
        DeviceType::DiscreteGpu,
        DeviceType::IntegratedGpu,
        DeviceType::Apu,
        DeviceType::ComputeOnly,
        DeviceType::VirtualGpu,
        DeviceType::Other("legacy".into()),
    ];
    assert_json_roundtrip(&variants);
}

#[test]
fn serde_device_capabilities_roundtrip() {
    assert_json_roundtrip(&sample_device_capabilities());
}

#[test]
fn serde_performance_characteristics_roundtrip() {
    assert_json_roundtrip(&sample_performance());
}

#[test]
fn serde_data_type_all_variants() {
    let variants = vec![
        DataType::Int8,
        DataType::Int16,
        DataType::Int32,
        DataType::Int64,
        DataType::UInt8,
        DataType::UInt16,
        DataType::UInt32,
        DataType::UInt64,
        DataType::Float16,
        DataType::Float32,
        DataType::Float64,
        DataType::Complex64,
        DataType::Complex128,
        DataType::Bool,
        DataType::Custom("ext".into()),
    ];
    assert_json_roundtrip(&variants);
}

#[test]
fn serde_device_usage_roundtrip() {
    let u = DeviceUsage {
        gpu_utilization_percent: 12.5,
        memory_utilization_percent: 50.0,
        memory_used_bytes: 1024,
        temperature_celsius: Some(65.0),
        power_usage_watts: Some(120.0),
        active_sessions: 2,
    };
    assert_json_roundtrip(&u);
}

#[test]
fn serde_device_requirements_minimal_and_high_performance() {
    assert_json_roundtrip(&DeviceRequirements::minimal());
    assert_json_roundtrip(&DeviceRequirements::high_performance());
}

#[test]
fn serde_resource_allocation_roundtrip() {
    let r = ResourceAllocation {
        memory_bytes: 4096,
        compute_units: 4,
        priority: 10,
    };
    assert_json_roundtrip(&r);
}

#[test]
fn serde_session_status_all_variants() {
    let variants = vec![
        SessionStatus::Initializing,
        SessionStatus::Running,
        SessionStatus::Paused,
        SessionStatus::Completed,
        SessionStatus::Failed("boom".into()),
        SessionStatus::Cancelled,
    ];
    assert_json_roundtrip(&variants);
}

#[test]
fn serde_kernel_format_all_variants() {
    let variants = vec![
        KernelFormat::OpenClC,
        KernelFormat::CudaC,
        KernelFormat::Hlsl,
        KernelFormat::Glsl,
        KernelFormat::Msl,
        KernelFormat::Spirv,
        KernelFormat::LlvmIr,
        KernelFormat::Wasm,
        KernelFormat::Wgsl,
        KernelFormat::Tucl,
    ];
    assert_json_roundtrip(&variants);
}

#[test]
fn serde_access_pattern_all_variants() {
    let variants = vec![
        AccessPattern::ReadOnly,
        AccessPattern::WriteOnly,
        AccessPattern::ReadWrite,
    ];
    assert_json_roundtrip(&variants);
}

// --- Validation / factory helpers ---

#[test]
fn gpu_framework_name_and_universal_and_platforms() {
    assert_eq!(GpuFramework::WebGpu.name(), "WebGPU");
    assert_eq!(GpuFramework::Vulkan.name(), "Vulkan");
    assert_eq!(GpuFramework::OpenCl.name(), "OpenCL");
    assert_eq!(GpuFramework::Cuda.name(), "CUDA");
    assert_eq!(GpuFramework::Metal.name(), "Metal");
    assert_eq!(GpuFramework::Rocm.name(), "ROCm");
    assert_eq!(GpuFramework::DirectCompute.name(), "DirectCompute");
    assert_eq!(GpuFramework::Custom("MyFw".into()).name(), "MyFw");

    assert!(GpuFramework::WebGpu.is_universal());
    assert!(GpuFramework::Vulkan.is_universal());
    assert!(GpuFramework::OpenCl.is_universal());
    assert!(!GpuFramework::Cuda.is_universal());

    let custom = GpuFramework::Custom("x".into());
    let p = custom.platform_compatibility();
    assert_eq!(p, vec!["Unknown"]);

    assert!(
        GpuFramework::WebGpu
            .platform_compatibility()
            .contains(&"Web")
    );
    assert!(
        GpuFramework::Vulkan
            .platform_compatibility()
            .contains(&"Android")
    );
}

#[test]
fn device_id_new_stores_fields() {
    let id = DeviceId::new(GpuFramework::Cuda, 42, "abc".into());
    assert_eq!(id.framework, GpuFramework::Cuda);
    assert_eq!(id.device_index, 42);
    assert_eq!(id.uuid, "abc");
}

#[test]
fn device_requirements_minimal_and_high_performance_invariants() {
    let m = DeviceRequirements::minimal();
    assert_eq!(m.min_memory_bytes, Some(64 * 1024 * 1024));
    assert_eq!(m.min_compute_units, Some(1));
    assert_eq!(m.required_data_types.len(), 1);
    assert!(matches!(&m.required_data_types[0], DataType::Float32));

    let h = DeviceRequirements::high_performance();
    assert!(h.min_memory_bytes.unwrap() > m.min_memory_bytes.unwrap());
    assert!(h.min_compute_units.unwrap() >= m.min_compute_units.unwrap());
    assert!(
        h.required_data_types
            .iter()
            .any(|d| matches!(d, DataType::Float64))
    );
    assert_eq!(h.min_compute_capability.as_deref(), Some("6.0"));
}

// --- Hash / equality ---

#[test]
fn gpu_framework_eq_hash_as_map_key() {
    let a = GpuFramework::OpenCl;
    let b = GpuFramework::OpenCl;
    assert_eq!(a, b);

    let mut m = HashMap::new();
    m.insert(GpuFramework::Vulkan, 1u32);
    m.insert(GpuFramework::Cuda, 2);
    assert_eq!(m[&GpuFramework::Vulkan], 1);

    let mut set = HashSet::new();
    set.insert(GpuFramework::Metal);
    assert!(set.contains(&GpuFramework::Metal));
}

#[test]
fn device_id_eq_hash() {
    let id = DeviceId::new(GpuFramework::WebGpu, 0, "same".into());
    let id2 = id.clone();
    assert_eq!(id, id2);

    let mut h = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut h);
    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    id2.hash(&mut h2);
    assert_eq!(h.finish(), h2.finish());
}

#[test]
fn kernel_format_hash_stable_for_same_variant() {
    let mut h1 = std::collections::hash_map::DefaultHasher::new();
    KernelFormat::Wgsl.hash(&mut h1);
    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    KernelFormat::Wgsl.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
}

// --- Clone / non-serde construction ---

#[test]
fn universal_compute_device_clone_shares_usage_arc() {
    let usage = Arc::new(RwLock::new(DeviceUsage::default()));
    let d = UniversalComputeDevice {
        id: DeviceId::new(GpuFramework::WebGpu, 0, "u".into()),
        info: sample_device_info(),
        capabilities: sample_device_capabilities(),
        usage: Arc::clone(&usage),
        framework_handle: None,
    };
    let d2 = d.clone();
    assert!(Arc::ptr_eq(&d.usage, &d2.usage));
}

#[tokio::test]
async fn universal_compute_device_usage_rwlock_readable() {
    let usage = Arc::new(RwLock::new(DeviceUsage {
        gpu_utilization_percent: 1.0,
        memory_utilization_percent: 2.0,
        memory_used_bytes: 3,
        temperature_celsius: None,
        power_usage_watts: None,
        active_sessions: 0,
    }));
    let d = UniversalComputeDevice {
        id: DeviceId::new(GpuFramework::OpenCl, 0, "u".into()),
        info: sample_device_info(),
        capabilities: sample_device_capabilities(),
        usage: Arc::clone(&usage),
        framework_handle: None,
    };
    let u = d.usage.read().await;
    assert_eq!(u.gpu_utilization_percent, 1.0);
}

#[test]
fn framework_handle_unavailable_clone_roundtrip() {
    let a = FrameworkHandle::Unavailable {
        name: "n".into(),
        reason: "r".into(),
    };
    let b = a.clone();
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
}

#[test]
fn compute_workload_recursive_structure() {
    let inner = ComputeWorkload {
        name: "inner".into(),
        kernel_source: "k".into(),
        kernel_format: KernelFormat::OpenClC,
        inputs: vec![],
        requirements: DeviceRequirements::minimal(),
        parent_session: None,
        recursive_workloads: vec![],
        priority: 1,
    };
    let outer = ComputeWorkload {
        name: "outer".into(),
        kernel_source: "k2".into(),
        kernel_format: KernelFormat::CudaC,
        inputs: vec![KernelInput {
            name: "i".into(),
            data: Bytes::from_static(b"z"),
            data_type: DataType::UInt8,
            access_pattern: AccessPattern::ReadOnly,
        }],
        requirements: DeviceRequirements::high_performance(),
        parent_session: Some(Uuid::nil()),
        recursive_workloads: vec![inner],
        priority: 2,
    };
    assert_eq!(outer.recursive_workloads.len(), 1);
    assert_eq!(outer.inputs.len(), 1);
}

#[test]
fn compute_result_recursive_clone() {
    let out = KernelOutput {
        buffers: HashMap::new(),
        metrics: ExecutionMetrics {
            execution_time: Duration::from_nanos(1),
            memory_used: 0,
            compute_units_used: 0,
            energy_consumed: None,
            throughput: None,
        },
        errors: vec![],
    };
    let r = ComputeResult {
        session_id: Uuid::new_v4(),
        device_id: DeviceId::new(GpuFramework::Vulkan, 0, "id".into()),
        primary_output: out.clone(),
        recursive_results: vec![ComputeResult {
            session_id: Uuid::new_v4(),
            device_id: DeviceId::new(GpuFramework::Vulkan, 1, "id2".into()),
            primary_output: out,
            recursive_results: vec![],
            total_execution_time: Duration::ZERO,
        }],
        total_execution_time: Duration::from_secs(2),
    };
    let r2 = r.clone();
    assert_eq!(r2.recursive_results.len(), 1);
}
