// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::pedantic)]
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements
)]
//! Comprehensive coverage tests for NPU dispatch module
//! Target: exercise all types, error variants, and trait behavior.

use std::borrow::Cow;

use akida_driver::{
    BackendType, BatchCapabilities, Capabilities, ChipVersion, ModelHandle, NpuBackend, PcieConfig,
    WeightMutationSupport,
};
use toadstool_core::npu_dispatch::{
    AkidaNpuDispatch, DispatchResult, NpuCapability, NpuDispatch, NpuDispatchError,
    NpuInferenceRequest, NpuInfo, NpuModelHandle,
};

// ─── NpuModelHandle ────────────────────────────────────────────────────────

#[test]
fn npu_model_handle_new_and_id() {
    let h = NpuModelHandle::new(42);
    assert_eq!(h.id(), 42);
}

#[test]
fn npu_model_handle_equality() {
    let h1 = NpuModelHandle::new(1);
    let h2 = NpuModelHandle::new(1);
    let h3 = NpuModelHandle::new(2);
    assert_eq!(h1, h2);
    assert_ne!(h1, h3);
}

#[test]
fn npu_model_handle_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(NpuModelHandle::new(1));
    set.insert(NpuModelHandle::new(1));
    assert_eq!(set.len(), 1);
}

// ─── NpuCapability ─────────────────────────────────────────────────────────

#[test]
fn npu_capability_all_variants() {
    let _ = NpuCapability::Inference;
    let _ = NpuCapability::ReservoirComputing;
    let _ = NpuCapability::OnChipLearning;
    let _ = NpuCapability::SpikingNetwork;
    let _ = NpuCapability::BatchInference;
    let _ = NpuCapability::PowerMonitoring;
}

#[test]
fn npu_capability_equality() {
    assert_eq!(NpuCapability::Inference, NpuCapability::Inference);
    assert_ne!(NpuCapability::Inference, NpuCapability::SpikingNetwork);
}

// ─── NpuInfo ───────────────────────────────────────────────────────────────

#[test]
fn npu_info_construction() {
    let info = NpuInfo {
        name: "Test NPU".into(),
        vendor: "test".into(),
        processing_elements: 4,
        memory_bytes: 1024 * 1024,
        capabilities: vec![NpuCapability::Inference, NpuCapability::PowerMonitoring],
    };
    assert_eq!(info.processing_elements, 4);
    assert!(info.capabilities.contains(&NpuCapability::Inference));
}

// ─── NpuDispatchError ───────────────────────────────────────────────────────

#[test]
fn npu_dispatch_error_capability_not_supported() {
    let e = NpuDispatchError::CapabilityNotSupported(NpuCapability::SpikingNetwork);
    assert!(e.to_string().contains("SpikingNetwork"));
}

#[test]
fn npu_dispatch_error_device_not_ready() {
    let e = NpuDispatchError::DeviceNotReady {
        reason: "powered down".into(),
    };
    assert!(e.to_string().contains("powered down"));
}

#[test]
fn npu_dispatch_error_model_load_failed() {
    let e = NpuDispatchError::ModelLoadFailed {
        reason: "invalid format".into(),
    };
    assert!(e.to_string().contains("format") || e.to_string().contains("load"));
}

#[test]
fn npu_dispatch_error_dispatch_failed() {
    let e = NpuDispatchError::DispatchFailed {
        reason: "timeout".into(),
    };
    assert!(e.to_string().contains("timeout") || e.to_string().contains("dispatch"));
}

#[test]
fn npu_dispatch_error_device_lost() {
    let e = NpuDispatchError::DeviceLost {
        reason: "unplugged".into(),
    };
    assert!(e.to_string().contains("unplugged") || e.to_string().contains("lost"));
}

// ─── DispatchResult ────────────────────────────────────────────────────────

#[test]
fn dispatch_result_with_power() {
    let r = DispatchResult {
        output: vec![1.0, 2.0, 3.0],
        latency_us: 100,
        power_mw: Some(1500.0),
    };
    assert_eq!(r.output.len(), 3);
    assert_eq!(r.latency_us, 100);
    assert_eq!(r.power_mw, Some(1500.0));
}

#[test]
fn dispatch_result_without_power() {
    let r = DispatchResult {
        output: vec![1.0],
        latency_us: 50,
        power_mw: None,
    };
    assert!(r.power_mw.is_none());
}

// ─── NpuInferenceRequest ───────────────────────────────────────────────────

#[test]
fn npu_inference_request_full() {
    let req = NpuInferenceRequest {
        model: NpuModelHandle::new(1),
        input: vec![1.0, 2.0, 3.0],
        batch_size_hint: Some(4),
        priority: 0,
    };
    assert_eq!(req.model.id(), 1);
    assert_eq!(req.input.len(), 3);
    assert_eq!(req.batch_size_hint, Some(4));
    assert_eq!(req.priority, 0);
}

#[test]
fn npu_inference_request_minimal() {
    let req = NpuInferenceRequest {
        model: NpuModelHandle::new(0),
        input: vec![],
        batch_size_hint: None,
        priority: 255,
    };
    assert!(req.input.is_empty());
    assert!(req.batch_size_hint.is_none());
}

// ─── Mock NpuDispatch for supports() default ────────────────────────────────

struct MockNpuDispatch {
    info: NpuInfo,
}

impl std::fmt::Debug for MockNpuDispatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MockNpuDispatch")
    }
}

impl NpuDispatch for MockNpuDispatch {
    fn info(&self) -> &NpuInfo {
        &self.info
    }
    fn load_model(&mut self, _: &[u8]) -> Result<NpuModelHandle, NpuDispatchError> {
        Ok(NpuModelHandle::new(1))
    }
    fn dispatch(
        &mut self,
        _: NpuModelHandle,
        input: Cow<'_, [f32]>,
    ) -> Result<DispatchResult, NpuDispatchError> {
        Ok(DispatchResult {
            output: input.to_vec(),
            latency_us: 0,
            power_mw: None,
        })
    }
    fn power_mw(&self) -> Result<f32, NpuDispatchError> {
        Err(NpuDispatchError::CapabilityNotSupported(
            NpuCapability::PowerMonitoring,
        ))
    }
    fn is_alive(&self) -> bool {
        true
    }
}

#[test]
fn npu_dispatch_supports_default_impl() {
    let info = NpuInfo {
        name: "Mock".into(),
        vendor: "test".into(),
        processing_elements: 4,
        memory_bytes: 4096,
        capabilities: vec![NpuCapability::Inference, NpuCapability::BatchInference],
    };
    let npu = MockNpuDispatch { info };
    assert!(npu.supports(NpuCapability::Inference));
    assert!(npu.supports(NpuCapability::BatchInference));
    assert!(!npu.supports(NpuCapability::OnChipLearning));
}

#[test]
fn npu_dispatch_dispatch_request_default() {
    let info = NpuInfo {
        name: "Mock".into(),
        vendor: "test".into(),
        processing_elements: 4,
        memory_bytes: 4096,
        capabilities: vec![NpuCapability::Inference],
    };
    let mut npu = MockNpuDispatch { info };
    let handle = npu.load_model(&[]).unwrap();
    let req = NpuInferenceRequest {
        model: handle,
        input: vec![1.0, 2.0, 3.0],
        batch_size_hint: None,
        priority: 0,
    };
    let result = npu.dispatch_request(req);
    assert!(result.is_ok());
    let r = result.unwrap();
    assert_eq!(r.output, vec![1.0, 2.0, 3.0]);
}

// ─── Mock NpuBackend for AkidaNpuDispatch ────────────────────────────────────

/// Mock akida-driver backend to exercise AkidaNpuDispatch without hardware.
#[derive(Debug)]
struct MockAkidaBackend {
    caps: Capabilities,
    model_counter: std::sync::atomic::AtomicU32,
}

impl MockAkidaBackend {
    fn new() -> Self {
        let caps = Capabilities {
            chip_version: ChipVersion::Akd1000,
            npu_count: 80,
            memory_mb: 10,
            pcie: PcieConfig::new(3, 8),
            power_mw: None,
            temperature_c: None,
            mesh: None,
            clock_mode: None,
            batch: Some(BatchCapabilities {
                max_batch: 8,
                optimal_batch: 8,
                optimal_speedup: 2.35,
            }),
            weight_mutation: WeightMutationSupport::None,
        };
        Self {
            caps,
            model_counter: std::sync::atomic::AtomicU32::new(0),
        }
    }
}

impl NpuBackend for MockAkidaBackend {
    fn init(_device_id: &str) -> akida_driver::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self::new())
    }

    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }

    fn load_model(&mut self, _model: &[u8]) -> akida_driver::Result<ModelHandle> {
        let id = self
            .model_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        Ok(ModelHandle::new(id))
    }

    fn load_reservoir(&mut self, _w_in: &[f32], _w_res: &[f32]) -> akida_driver::Result<()> {
        Ok(())
    }

    fn infer(&mut self, input: &[f32]) -> akida_driver::Result<Vec<f32>> {
        Ok(input.to_vec())
    }

    fn measure_power(&self) -> akida_driver::Result<f32> {
        Ok(1500.0)
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Userspace
    }

    fn is_ready(&self) -> bool {
        true
    }
}

#[test]
fn akida_npu_dispatch_from_backend_info() {
    let backend = Box::new(MockAkidaBackend::new());
    let dispatch = AkidaNpuDispatch::from_backend(backend);
    let info = dispatch.info();
    assert!(info.name.starts_with("Akida"));
    assert_eq!(info.vendor, "brainchip");
    assert_eq!(info.processing_elements, 80);
    assert!(dispatch.supports(NpuCapability::Inference));
    assert!(dispatch.supports(NpuCapability::PowerMonitoring));
    assert!(dispatch.supports(NpuCapability::ReservoirComputing));
    assert!(dispatch.supports(NpuCapability::BatchInference));
    assert!(!dispatch.supports(NpuCapability::OnChipLearning));
}

#[test]
fn akida_npu_dispatch_load_and_dispatch() {
    let backend = Box::new(MockAkidaBackend::new());
    let mut dispatch = AkidaNpuDispatch::from_backend(backend);
    let handle = dispatch.load_model(b"fake_model_data").unwrap();
    assert_eq!(handle.id(), 1);

    let result = dispatch
        .dispatch(handle, Cow::Borrowed(&[1.0, 2.0, 3.0]))
        .unwrap();
    assert_eq!(result.output, vec![1.0, 2.0, 3.0]);
    assert!(result.latency_us > 0, "mock should return positive latency");
    assert_eq!(result.power_mw, Some(1500.0));
}

#[test]
fn akida_npu_dispatch_power_mw() {
    let backend = Box::new(MockAkidaBackend::new());
    let dispatch = AkidaNpuDispatch::from_backend(backend);
    let power = dispatch.power_mw().unwrap();
    assert!((power - 1500.0).abs() < f32::EPSILON);
}

#[test]
fn akida_npu_dispatch_is_alive() {
    let backend = Box::new(MockAkidaBackend::new());
    let dispatch = AkidaNpuDispatch::from_backend(backend);
    assert!(dispatch.is_alive());
}
