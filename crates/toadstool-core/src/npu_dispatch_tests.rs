// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

#[derive(Debug)]
struct MockNpuDispatch {
    info: NpuInfo,
    next_handle: u32,
}

impl NpuDispatch for MockNpuDispatch {
    fn info(&self) -> &NpuInfo {
        &self.info
    }

    fn load_model(&mut self, _model_data: &[u8]) -> Result<NpuModelHandle, NpuDispatchError> {
        let handle = NpuModelHandle::new(self.next_handle);
        self.next_handle += 1;
        Ok(handle)
    }

    fn dispatch(
        &mut self,
        _model: NpuModelHandle,
        input: Cow<'_, [f32]>,
    ) -> Result<DispatchResult, NpuDispatchError> {
        Ok(DispatchResult {
            output: input.into_owned(),
            latency_us: 10,
            power_mw: Some(100.0),
        })
    }

    fn power_mw(&self) -> Result<f32, NpuDispatchError> {
        Ok(100.0)
    }

    fn is_alive(&self) -> bool {
        true
    }
}

#[test]
fn test_npu_model_handle() {
    let handle = NpuModelHandle::new(42);
    assert_eq!(handle.id(), 42);
}

#[test]
fn test_npu_info_capabilities() {
    let info = NpuInfo {
        name: "Test NPU".into(),
        vendor: "test".into(),
        processing_elements: 4,
        memory_bytes: 1024 * 1024,
        capabilities: vec![NpuCapability::Inference, NpuCapability::PowerMonitoring],
    };
    assert_eq!(info.processing_elements, 4);
    assert!(info.capabilities.contains(&NpuCapability::Inference));
    assert!(!info.capabilities.contains(&NpuCapability::OnChipLearning));
}

#[test]
fn test_dispatch_error_display() {
    let err = NpuDispatchError::CapabilityNotSupported(NpuCapability::SpikingNetwork);
    assert!(err.to_string().contains("SpikingNetwork"));

    let err = NpuDispatchError::DeviceNotReady {
        reason: "powered down".into(),
    };
    assert!(err.to_string().contains("powered down"));
}

#[test]
fn test_dispatch_result_structure() {
    let result = DispatchResult {
        output: vec![1.0, 2.0, 3.0],
        latency_us: 100,
        power_mw: Some(1500.0),
    };
    assert_eq!(result.output.len(), 3);
    assert_eq!(result.latency_us, 100);
    assert_eq!(result.power_mw, Some(1500.0));
}

#[test]
fn test_npu_capability_variants() {
    let _ = NpuCapability::Inference;
    let _ = NpuCapability::ReservoirComputing;
    let _ = NpuCapability::OnChipLearning;
    let _ = NpuCapability::SpikingNetwork;
    let _ = NpuCapability::BatchInference;
    let _ = NpuCapability::PowerMonitoring;
}

#[test]
fn test_npu_dispatch_error_variants() {
    let _ = NpuDispatchError::ModelLoadFailed {
        reason: "invalid format".into(),
    };
    let _ = NpuDispatchError::DispatchFailed {
        reason: "timeout".into(),
    };
    let _ = NpuDispatchError::DeviceLost {
        reason: "unplugged".into(),
    };
}

#[test]
fn test_npu_inference_request_construction() {
    let handle = NpuModelHandle::new(1);
    let request = NpuInferenceRequest {
        model: handle,
        input: vec![1.0, 2.0, 3.0],
        batch_size_hint: Some(4),
        priority: 0,
    };
    assert_eq!(request.model.id(), 1);
    assert_eq!(request.input.len(), 3);
    assert_eq!(request.batch_size_hint, Some(4));
    assert_eq!(request.priority, 0);
}

#[test]
fn test_npu_inference_request_minimal() {
    let request = NpuInferenceRequest {
        model: NpuModelHandle::new(0),
        input: vec![],
        batch_size_hint: None,
        priority: 255,
    };
    assert!(request.input.is_empty());
    assert!(request.batch_size_hint.is_none());
    assert_eq!(request.priority, 255);
}

#[test]
fn test_npu_model_handle_equality() {
    let h1 = NpuModelHandle::new(1);
    let h2 = NpuModelHandle::new(1);
    let h3 = NpuModelHandle::new(2);
    assert_eq!(h1, h2);
    assert_ne!(h1, h3);
}

#[test]
fn test_npu_capability_supports_logic() {
    let info = NpuInfo {
        name: "Test".into(),
        vendor: "v".into(),
        processing_elements: 4,
        memory_bytes: 4096,
        capabilities: vec![NpuCapability::Inference, NpuCapability::BatchInference],
    };
    assert!(info.capabilities.contains(&NpuCapability::Inference));
    assert!(!info.capabilities.contains(&NpuCapability::OnChipLearning));
}

#[test]
fn test_npu_dispatch_error_model_load_failed() {
    let err = NpuDispatchError::ModelLoadFailed {
        reason: "invalid format".into(),
    };
    assert!(err.to_string().contains("format") || err.to_string().contains("load"));
}

#[test]
fn test_npu_dispatch_error_device_lost() {
    let err = NpuDispatchError::DeviceLost {
        reason: "unplugged".into(),
    };
    assert!(err.to_string().contains("unplugged") || err.to_string().contains("lost"));
}

#[test]
fn test_dispatch_result_power_none() {
    let result = DispatchResult {
        output: vec![1.0],
        latency_us: 50,
        power_mw: None,
    };
    assert!(result.power_mw.is_none());
}

#[test]
fn test_npu_inference_request_cow_borrowed() {
    let input: [f32; 3] = [1.0, 2.0, 3.0];
    let request = NpuInferenceRequest {
        model: NpuModelHandle::new(1),
        input: input.to_vec(),
        batch_size_hint: Some(1),
        priority: 0,
    };
    assert_eq!(request.input.len(), 3);
}

#[test]
fn test_npu_info_memory_bytes() {
    let info = NpuInfo {
        name: "A".into(),
        vendor: "B".into(),
        processing_elements: 8,
        memory_bytes: 16 * 1024 * 1024,
        capabilities: vec![NpuCapability::Inference],
    };
    assert_eq!(info.memory_bytes, 16 * 1024 * 1024);
}

#[test]
fn test_npu_dispatch_supports_trait_default() {
    let info = NpuInfo {
        name: "Test".into(),
        vendor: "v".into(),
        processing_elements: 4,
        memory_bytes: 4096,
        capabilities: vec![NpuCapability::Inference, NpuCapability::SpikingNetwork],
    };
    let npu = MockNpuDispatch {
        info,
        next_handle: 0,
    };
    assert!(npu.supports(NpuCapability::SpikingNetwork));
    assert!(!npu.supports(NpuCapability::BatchInference));
}

#[test]
fn test_dispatch_with_cow_borrowed() {
    let info = NpuInfo {
        name: "Test".into(),
        vendor: "v".into(),
        processing_elements: 4,
        memory_bytes: 4096,
        capabilities: vec![NpuCapability::Inference],
    };
    let mut npu = MockNpuDispatch {
        info,
        next_handle: 0,
    };
    let handle = npu.load_model(&[]).unwrap();
    let input: [f32; 2] = [1.5, 2.5];
    let result = npu.dispatch(handle, Cow::Borrowed(&input)).unwrap();
    assert_eq!(result.output, vec![1.5, 2.5]);
}
