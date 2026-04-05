// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage tests for under-covered modules in toadstool-core (S155).
//!
//! Target modules:
//! - hardware.rs (detection, classification)
//! - `npu_dispatch.rs` (scheduling, routing)
//! - `transport_router.rs` (filtering, routing logic)
//! - `hardware_transport.rs` (traits, types, frame protocol)
//! - `npu_controller.rs` (types, operations)

use std::borrow::Cow;

use toadstool_core::hardware::{HardwareDevice, HardwareError, HardwareManager, HardwareType};
use toadstool_core::hardware_transport::{
    FRAME_HEADER_SIZE, TransportDirection, TransportError, TransportInfo, TransportMedium,
    decode_frame, encode_frame,
};
use toadstool_core::npu_controller::{
    ControllerError, ParameterSuggestion, ProxyFeature, ProxyFeatureSet, SafetyClamp,
    SuggestionSource,
};
use toadstool_core::npu_dispatch::{
    DispatchResult, NpuCapability, NpuDispatch, NpuDispatchError, NpuInferenceRequest, NpuInfo,
    NpuModelHandle,
};
use toadstool_core::transport_router::{TransportFilter, TransportRouter};

// ============================================================================
// Hardware (hardware.rs)
// ============================================================================

#[test]
fn hardware_type_all_variants() {
    let _ = HardwareType::Gpu;
    let _ = HardwareType::Npu;
    let _ = HardwareType::Cpu;
    let _ = HardwareType::Fpga;
    let _ = HardwareType::Custom;
}

#[test]
fn hardware_device_full_structure() {
    let device = HardwareDevice {
        hardware_type: HardwareType::Npu,
        name: "Akida AKD1000".to_string(),
        pcie_address: Some("0000:01:00.0".to_string()),
        vendor_id: Some("1e7c".to_string()),
        device_id: Some("bca1".to_string()),
        driver_available: true,
        userspace_capable: false,
    };
    assert_eq!(device.name, "Akida AKD1000");
    assert_eq!(device.hardware_type, HardwareType::Npu);
    assert_eq!(device.pcie_address.as_deref(), Some("0000:01:00.0"));
    assert_eq!(device.vendor_id.as_deref(), Some("1e7c"));
    assert_eq!(device.device_id.as_deref(), Some("bca1"));
}

#[test]
fn hardware_manager_discover_and_devices() {
    let manager = HardwareManager::discover().expect("Discovery failed");
    assert!(!manager.devices().is_empty());
    assert!(manager.device_count() >= 1);
}

#[test]
fn hardware_manager_devices_by_type_all() {
    let manager = HardwareManager::discover().unwrap();
    for ht in [
        HardwareType::Gpu,
        HardwareType::Npu,
        HardwareType::Cpu,
        HardwareType::Fpga,
        HardwareType::Custom,
    ] {
        let devs = manager.devices_by_type(ht);
        for d in &devs {
            assert_eq!(d.hardware_type, ht);
        }
    }
}

#[test]
fn hardware_manager_has_gpu_npu() {
    let manager = HardwareManager::discover().unwrap();
    let _ = manager.has_gpu();
    let _ = manager.has_npu();
}

#[test]
fn hardware_manager_rescan() {
    let mut manager = HardwareManager::discover().unwrap();
    manager.rescan().expect("Rescan failed");
}

#[test]
fn hardware_error_npu_not_found() {
    let err = HardwareError::NpuNotFound {
        address: "0000:ff:00.0".to_string(),
    };
    let s = err.to_string();
    assert!(s.contains("NPU device not found"));
    assert!(s.contains("0000:ff:00.0"));
}

#[test]
fn hardware_enable_npu_userspace_nonexistent() {
    let manager = HardwareManager::discover().unwrap();
    let result = manager.enable_npu_userspace("0000:ff:00.0-nonexistent");
    assert!(result.is_err());
    assert!(matches!(result, Err(HardwareError::NpuNotFound { .. })));
}

// ============================================================================
// NPU Dispatch (npu_dispatch.rs)
// ============================================================================

#[test]
fn npu_model_handle_new_and_id() {
    let h = NpuModelHandle::new(99);
    assert_eq!(h.id(), 99);
}

#[test]
fn npu_model_handle_equality_hash() {
    let h1 = NpuModelHandle::new(1);
    let h2 = NpuModelHandle::new(1);
    let h3 = NpuModelHandle::new(2);
    assert_eq!(h1, h2);
    assert_ne!(h1, h3);
}

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
fn npu_info_and_capabilities() {
    let info = NpuInfo {
        name: "Test NPU".into(),
        vendor: "test".into(),
        processing_elements: 8,
        memory_bytes: 16 * 1024 * 1024,
        capabilities: vec![
            NpuCapability::Inference,
            NpuCapability::BatchInference,
            NpuCapability::PowerMonitoring,
        ],
    };
    assert_eq!(info.processing_elements, 8);
    assert!(info.capabilities.contains(&NpuCapability::Inference));
}

#[test]
fn npu_dispatch_error_all_variants() {
    let _ = NpuDispatchError::CapabilityNotSupported(NpuCapability::SpikingNetwork);
    let _ = NpuDispatchError::DeviceNotReady {
        reason: "off".into(),
    };
    let _ = NpuDispatchError::ModelLoadFailed {
        reason: "bad format".into(),
    };
    let _ = NpuDispatchError::DispatchFailed {
        reason: "timeout".into(),
    };
    let _ = NpuDispatchError::DeviceLost {
        reason: "unplugged".into(),
    };
}

#[test]
fn npu_dispatch_error_display() {
    let e = NpuDispatchError::CapabilityNotSupported(NpuCapability::OnChipLearning);
    assert!(e.to_string().contains("OnChipLearning"));
}

#[test]
fn dispatch_result_structure() {
    let r = DispatchResult {
        output: vec![1.0, 2.0, 3.0],
        latency_us: 100,
        power_mw: Some(500.0),
    };
    assert_eq!(r.output.len(), 3);
    assert_eq!(r.latency_us, 100);
    assert_eq!(r.power_mw, Some(500.0));
}

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

/// Mock `NpuDispatch` for testing `supports()` and `dispatch_request()` default impl.
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
fn npu_dispatch_supports_default() {
    let info = NpuInfo {
        name: "Mock".into(),
        vendor: "test".into(),
        processing_elements: 4,
        memory_bytes: 4096,
        capabilities: vec![NpuCapability::Inference, NpuCapability::BatchInference],
    };
    let npu = MockNpuDispatch { info };
    assert!(npu.supports(NpuCapability::Inference));
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

// ============================================================================
// Transport Router (transport_router.rs)
// ============================================================================

/// Minimal loopback transport for testing.
struct LoopbackTransport {
    info: TransportInfo,
    buf: Vec<u8>,
    bandwidth: u64,
}

impl LoopbackTransport {
    fn new(id: &str, direction: TransportDirection, bandwidth: u64) -> Self {
        Self {
            info: TransportInfo {
                id: id.to_string(),
                label: id.to_string(),
                medium: TransportMedium::Serial,
                direction,
            },
            buf: Vec::new(),
            bandwidth,
        }
    }
}

impl toadstool_core::HardwareTransport for LoopbackTransport {
    fn info(&self) -> &TransportInfo {
        &self.info
    }
    fn bandwidth_bps(&self) -> u64 {
        self.bandwidth
    }
    fn is_available(&self) -> bool {
        true
    }
    fn send(&mut self, data: &[u8]) -> Result<usize, TransportError> {
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        let n = buf.len().min(self.buf.len());
        buf[..n].copy_from_slice(&self.buf[..n]);
        self.buf.drain(..n);
        Ok(n)
    }
}

#[test]
fn transport_filter_tx_rx() {
    let tx = TransportFilter::tx();
    assert_eq!(tx.direction, Some(TransportDirection::Tx));

    let rx = TransportFilter::rx();
    assert_eq!(rx.direction, Some(TransportDirection::Rx));
}

#[test]
fn transport_filter_with_medium() {
    let f = TransportFilter::tx().with_medium(TransportMedium::Display);
    assert_eq!(f.medium, Some(TransportMedium::Display));
}

#[test]
fn transport_filter_with_min_bandwidth() {
    let f = TransportFilter::tx().with_min_bandwidth(10_000_000_000);
    assert_eq!(f.min_bandwidth_bps, 10_000_000_000);
}

#[test]
fn transport_router_new_and_register() {
    let mut router = TransportRouter::new();
    router.register(Box::new(LoopbackTransport::new(
        "a",
        TransportDirection::Tx,
        1_000_000,
    )));
    assert_eq!(router.list().len(), 1);
}

#[test]
fn transport_router_unregister() {
    let mut router = TransportRouter::new();
    router.register(Box::new(LoopbackTransport::new(
        "x",
        TransportDirection::Rx,
        1_000_000,
    )));
    let removed = router.unregister("x");
    assert!(removed.is_some());
    assert!(router.unregister("x").is_none());
}

#[test]
fn transport_router_find_by_direction() {
    let mut router = TransportRouter::new();
    router.register(Box::new(LoopbackTransport::new(
        "tx1",
        TransportDirection::Tx,
        1_000_000,
    )));
    router.register(Box::new(LoopbackTransport::new(
        "rx1",
        TransportDirection::Rx,
        1_000_000,
    )));
    let tx_ids = router.find(&TransportFilter::tx());
    assert!(tx_ids.contains(&"tx1".to_string()));
    assert!(!tx_ids.contains(&"rx1".to_string()));
}

#[test]
fn transport_router_find_by_bandwidth() {
    let mut router = TransportRouter::new();
    router.register(Box::new(LoopbackTransport::new(
        "slow",
        TransportDirection::Tx,
        100_000,
    )));
    let high = router.find(&TransportFilter::tx().with_min_bandwidth(10_000_000_000));
    assert!(high.is_empty());
    let low = router.find(&TransportFilter::tx().with_min_bandwidth(50_000));
    assert_eq!(low.len(), 1);
}

#[test]
fn transport_router_route_once() {
    let mut router = TransportRouter::new();
    let mut rx = LoopbackTransport::new("rx", TransportDirection::Bidirectional, 1_000_000);
    rx.buf = b"hello".to_vec();
    router.register(Box::new(rx));
    router.register(Box::new(LoopbackTransport::new(
        "tx",
        TransportDirection::Bidirectional,
        1_000_000,
    )));
    let n = router.route_once("rx", "tx", 1024).unwrap();
    assert_eq!(n, 5);
}

#[test]
fn transport_router_route_same_id_rejected() {
    let mut router = TransportRouter::new();
    router.register(Box::new(LoopbackTransport::new(
        "self",
        TransportDirection::Bidirectional,
        1_000_000,
    )));
    let result = router.route_once("self", "self", 64);
    assert!(result.is_err());
}

#[test]
fn transport_router_route_rx_not_found() {
    let mut router = TransportRouter::new();
    router.register(Box::new(LoopbackTransport::new(
        "tx",
        TransportDirection::Tx,
        1_000_000,
    )));
    let result = router.route_once("nonexistent-rx", "tx", 64);
    assert!(result.is_err());
}

#[test]
fn transport_router_route_tx_not_found() {
    let mut router = TransportRouter::new();
    router.register(Box::new(LoopbackTransport::new(
        "rx",
        TransportDirection::Rx,
        1_000_000,
    )));
    let result = router.route_once("rx", "nonexistent-tx", 64);
    assert!(result.is_err());
}

#[test]
fn transport_router_get() {
    let mut router = TransportRouter::new();
    router.register(Box::new(LoopbackTransport::new(
        "a",
        TransportDirection::Tx,
        1_000_000,
    )));
    let t = router.get("a");
    assert!(t.is_some());
    assert!(router.get("b").is_none());
}

#[test]
fn transport_router_default() {
    let router = TransportRouter::default();
    assert!(router.list().is_empty());
}

#[test]
fn transport_router_filter_by_medium() {
    let mut router = TransportRouter::new();
    router.register(Box::new(LoopbackTransport::new(
        "serial1",
        TransportDirection::Tx,
        1_000_000,
    )));
    router.register(Box::new(LoopbackTransport::new(
        "serial2",
        TransportDirection::Rx,
        1_000_000,
    )));
    let serial_ids = router.find(&TransportFilter::tx().with_medium(TransportMedium::Serial));
    assert_eq!(serial_ids.len(), 1);
    assert!(serial_ids.contains(&"serial1".to_string()));
}

#[test]
fn transport_router_route_loop() {
    let mut router = TransportRouter::new();
    let mut rx = LoopbackTransport::new("rx", TransportDirection::Bidirectional, 1_000_000);
    rx.buf = b"chunk1".to_vec();
    router.register(Box::new(rx));
    router.register(Box::new(LoopbackTransport::new(
        "tx",
        TransportDirection::Bidirectional,
        1_000_000,
    )));
    let mut chunk_count = 0;
    let total = router
        .route_loop("rx", "tx", 1024, |n| {
            chunk_count += 1;
            chunk_count < 3 && n > 0
        })
        .unwrap();
    assert!(total > 0, "route_loop should transfer data");
    assert!(chunk_count >= 1, "route_loop should run at least once");
}

#[test]
fn transport_router_route_once_empty_rx_returns_zero() {
    let mut router = TransportRouter::new();
    let rx = LoopbackTransport::new("rx", TransportDirection::Bidirectional, 1_000_000);
    router.register(Box::new(rx));
    router.register(Box::new(LoopbackTransport::new(
        "tx",
        TransportDirection::Bidirectional,
        1_000_000,
    )));
    let n = router.route_once("rx", "tx", 1024).unwrap();
    assert_eq!(n, 0, "empty rx buffer should return 0 bytes transferred");
}

#[test]
fn transport_router_route_loop_stops_on_callback_false() {
    let mut router = TransportRouter::new();
    let mut rx = LoopbackTransport::new("rx", TransportDirection::Bidirectional, 1_000_000);
    rx.buf = b"data".to_vec();
    router.register(Box::new(rx));
    router.register(Box::new(LoopbackTransport::new(
        "tx",
        TransportDirection::Bidirectional,
        1_000_000,
    )));
    let mut iterations = 0;
    let total = router
        .route_loop("rx", "tx", 1024, |n| {
            iterations += 1;
            n > 0 && iterations < 1
        })
        .unwrap();
    assert_eq!(total, 4, "should transfer 4 bytes in first iteration");
    assert_eq!(iterations, 1);
}

#[test]
fn transport_router_get_mut() {
    let mut router = TransportRouter::new();
    router.register(Box::new(LoopbackTransport::new(
        "a",
        TransportDirection::Tx,
        1_000_000,
    )));
    let t = router.get_mut("a");
    assert!(t.is_some());
    assert!(router.get_mut("b").is_none());
}

// ============================================================================
// Hardware Transport (hardware_transport.rs)
// ============================================================================

#[test]
fn transport_direction_display() {
    assert_eq!(format!("{}", TransportDirection::Tx), "Tx");
    assert_eq!(format!("{}", TransportDirection::Rx), "Rx");
    assert_eq!(format!("{}", TransportDirection::Bidirectional), "Bidi");
}

#[test]
fn transport_medium_all_display() {
    assert_eq!(format!("{}", TransportMedium::Display), "Display");
    assert_eq!(format!("{}", TransportMedium::Capture), "Capture");
    assert_eq!(format!("{}", TransportMedium::Serial), "Serial");
    assert_eq!(format!("{}", TransportMedium::Pcie), "PCIe");
    assert_eq!(format!("{}", TransportMedium::NvLink), "NVLink");
}

#[test]
fn transport_error_unavailable() {
    let e = TransportError::Unavailable("disconnected".into());
    assert!(e.to_string().contains("unavailable"));
}

#[test]
fn transport_error_direction_mismatch() {
    let e = TransportError::DirectionMismatch {
        transport_dir: TransportDirection::Rx,
        required: TransportDirection::Tx,
    };
    assert!(e.to_string().contains("direction"));
}

#[test]
fn transport_error_frame_protocol() {
    let e = TransportError::FrameProtocol("bad magic".into());
    assert!(e.to_string().contains("frame"));
}

#[test]
fn encode_decode_frame_round_trip() {
    let payload = b"test payload";
    let mut buf = vec![0u8; FRAME_HEADER_SIZE + payload.len() + 64];
    let written = encode_frame(42, payload, &mut buf).unwrap();
    assert_eq!(written, FRAME_HEADER_SIZE + payload.len());
    let (seq, decoded) = decode_frame(&buf[..written]).unwrap();
    assert_eq!(seq, 42);
    assert_eq!(decoded, payload);
}

#[test]
fn encode_decode_frame_empty_payload() {
    let payload: &[u8] = &[];
    let mut buf = vec![0u8; FRAME_HEADER_SIZE + 16];
    let written = encode_frame(0, payload, &mut buf).unwrap();
    assert_eq!(written, FRAME_HEADER_SIZE);
    let (seq, decoded) = decode_frame(&buf[..written]).unwrap();
    assert_eq!(seq, 0);
    assert!(decoded.is_empty());
}

#[test]
fn encode_decode_frame_odd_length_payload() {
    let payload = b"abc";
    let mut buf = vec![0u8; FRAME_HEADER_SIZE + 8];
    let written = encode_frame(1, payload, &mut buf).unwrap();
    assert_eq!(written, FRAME_HEADER_SIZE + 3);
    let (seq, decoded) = decode_frame(&buf[..written]).unwrap();
    assert_eq!(seq, 1);
    assert_eq!(decoded, payload);
}

#[test]
fn encode_frame_too_small() {
    let mut buf = [0u8; 4];
    assert!(encode_frame(0, b"data", &mut buf).is_none());
}

#[test]
fn decode_frame_truncated() {
    assert!(decode_frame(&[0; 4]).is_err());
}

#[test]
fn decode_frame_bad_magic() {
    let mut buf = vec![0u8; 64];
    buf[0..4].copy_from_slice(b"CAFE");
    assert!(decode_frame(&buf).is_err());
}

#[test]
fn decode_frame_bad_version() {
    let mut buf = vec![0u8; 64];
    buf[0..4].copy_from_slice(b"TSXP");
    buf[4] = 99;
    assert!(decode_frame(&buf).is_err());
}

#[test]
fn decode_frame_checksum_mismatch() {
    let mut buf = vec![0u8; FRAME_HEADER_SIZE + 4];
    buf[0..4].copy_from_slice(b"TSXP");
    buf[4] = 1;
    buf[9..13].copy_from_slice(&4u32.to_le_bytes());
    buf[13..17].copy_from_slice(&0xDEAD_BEEF_u32.to_le_bytes());
    assert!(decode_frame(&buf).is_err());
}

#[test]
fn decode_frame_payload_too_small() {
    let mut buf = vec![0u8; FRAME_HEADER_SIZE];
    buf[0..4].copy_from_slice(b"TSXP");
    buf[4] = 1;
    buf[9..13].copy_from_slice(&100u32.to_le_bytes());
    assert!(decode_frame(&buf).is_err());
}

#[test]
fn transport_error_io_from_std() {
    use std::io;
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let transport_err = TransportError::from(io_err);
    assert!(
        transport_err.to_string().contains("file not found")
            || transport_err.to_string().contains("I/O")
    );
}

#[test]
fn frame_header_size_constant() {
    assert_eq!(FRAME_HEADER_SIZE, 17);
}

// ============================================================================
// NPU Controller (npu_controller.rs)
// ============================================================================

#[derive(Debug, Clone)]
struct TestParams {
    step_size: f64,
}

#[test]
fn proxy_feature_new() {
    let f = ProxyFeature::new("acceptance_rate", 0.75);
    assert_eq!(f.name, "acceptance_rate");
    assert!((f.value - 0.75).abs() < f64::EPSILON);
    assert!(f.target.is_none());
    assert!((f.weight - 1.0).abs() < f64::EPSILON);
}

#[test]
fn proxy_feature_with_target_weight() {
    let f = ProxyFeature::new("residual", 1e-6)
        .with_target(1e-8)
        .with_weight(2.0);
    assert_eq!(f.target, Some(1e-8));
    assert!((f.weight - 2.0).abs() < f64::EPSILON);
}

#[test]
fn proxy_feature_set() {
    let set: ProxyFeatureSet = vec![
        ProxyFeature::new("a", 1.0),
        ProxyFeature::new("b", 2.0).with_target(3.0),
    ];
    assert_eq!(set.len(), 2);
}

#[test]
fn parameter_suggestion_structure() {
    let s = ParameterSuggestion {
        params: TestParams { step_size: 0.01 },
        confidence: 0.8,
        source: SuggestionSource::NpuModel,
    };
    assert_eq!(s.source, SuggestionSource::NpuModel);
    assert!((s.confidence - 0.8).abs() < f64::EPSILON);
}

#[test]
fn suggestion_source_variants() {
    let _ = SuggestionSource::NpuModel;
    let _ = SuggestionSource::Heuristic;
    let _ = SuggestionSource::Default;
}

#[test]
fn safety_clamp_structure() {
    let clamp = SafetyClamp {
        min: TestParams { step_size: 0.001 },
        max: TestParams { step_size: 0.02 },
    };
    assert!(clamp.min.step_size < clamp.max.step_size);
}

#[test]
fn controller_error_variants() {
    let _ = ControllerError::NpuUnavailable("off".into());
    let _ = ControllerError::FeatureExtraction("failed".into());
    let _ = ControllerError::ModelNotTrained;
    let _ = ControllerError::Other("misc".into());
}

#[test]
fn controller_error_display() {
    let e = ControllerError::NpuUnavailable("powered down".into());
    assert!(e.to_string().contains("powered down"));

    let e = ControllerError::ModelNotTrained;
    assert!(e.to_string().contains("not trained"));
}
