// SPDX-License-Identifier: AGPL-3.0-or-later
//! Generic NPU Dispatch — vendor-agnostic neuromorphic compute interface.
//!
//! `NpuDispatch` is toadStool's hardware-layer abstraction for neuromorphic
//! processors. Any NPU vendor (Akida, Loihi, SpiNNaker, etc.) implements this
//! trait so that barraCuda and other primals can dispatch compute without
//! vendor-specific knowledge.
//!
//! ## Relationship to `akida_driver::NpuBackend`
//!
//! `NpuBackend` is the low-level driver trait (init, load model bytes, raw
//! inference). `NpuDispatch` sits above it, providing:
//! - Typed capability queries (what operations can this NPU accelerate?)
//! - Batch dispatch with zero-copy input
//! - Power-aware scheduling hints
//! - Hot-plug lifecycle management
//!
//! ## Deep Debt Principles
//!
//! - **Vendor-agnostic**: No BrainChip, Intel, or SpiNNaker types leak through.
//! - **Runtime discovery**: Capabilities are probed, not assumed.
//! - **Capability-based**: Consumers ask "can you do X?" not "are you Akida?"

use std::borrow::Cow;
use std::fmt::Debug;

/// Capability flags that an NPU may advertise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NpuCapability {
    /// Standard forward inference on a loaded model.
    Inference,
    /// Reservoir computing (echo state networks).
    ReservoirComputing,
    /// On-chip learning / weight mutation.
    OnChipLearning,
    /// Spiking neural network execution.
    SpikingNetwork,
    /// Batch inference (multiple inputs per dispatch).
    BatchInference,
    /// Power measurement / thermal monitoring.
    PowerMonitoring,
}

/// Vendor-agnostic NPU identification.
#[derive(Debug, Clone)]
pub struct NpuInfo {
    /// Human-readable name (e.g. "Akida AKD1000", "Loihi 2").
    pub name: String,
    /// Vendor identifier (e.g. "brainchip", "intel").
    pub vendor: String,
    /// Number of processing elements.
    pub processing_elements: u32,
    /// On-chip memory in bytes.
    pub memory_bytes: u64,
    /// Capabilities this device actually supports.
    pub capabilities: Vec<NpuCapability>,
}

/// Result of a dispatch operation.
#[derive(Debug)]
pub struct DispatchResult {
    /// Output tensor data.
    pub output: Vec<f32>,
    /// Inference latency in microseconds.
    pub latency_us: u64,
    /// Power consumed during dispatch in milliwatts (if available).
    pub power_mw: Option<f32>,
}

/// Errors from NPU dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum NpuDispatchError {
    /// The requested capability is not supported by this NPU.
    #[error("capability not supported: {0:?}")]
    CapabilityNotSupported(NpuCapability),

    /// Device is not ready (not initialized, powered down, etc.).
    #[error("device not ready: {reason}")]
    DeviceNotReady { reason: String },

    /// Model loading failed.
    #[error("model load failed: {reason}")]
    ModelLoadFailed { reason: String },

    /// Dispatch / inference failed.
    #[error("dispatch failed: {reason}")]
    DispatchFailed { reason: String },

    /// Device was hot-unplugged or became unreachable.
    #[error("device lost: {reason}")]
    DeviceLost { reason: String },
}

/// Generic NPU dispatch trait — vendor-agnostic neuromorphic compute.
///
/// Implementors bridge from this trait to their vendor-specific driver
/// (e.g. `akida_driver::NpuBackend`).
pub trait NpuDispatch: Debug + Send + Sync {
    /// Query device information and capabilities (discovered at runtime).
    fn info(&self) -> &NpuInfo;

    /// Check whether a specific capability is supported.
    fn supports(&self, capability: NpuCapability) -> bool {
        self.info().capabilities.contains(&capability)
    }

    /// Load a model from opaque bytes. Returns a handle for subsequent dispatch.
    ///
    /// # Errors
    /// Returns `NpuDispatchError::ModelLoadFailed` if the model format is invalid
    /// or the device rejects the model.
    fn load_model(&mut self, model_data: &[u8]) -> Result<NpuModelHandle, NpuDispatchError>;

    /// Run inference on the loaded model.
    ///
    /// `input` is borrowed to enable zero-copy when the caller already owns the
    /// buffer. Use `Cow::Borrowed` for slices, `Cow::Owned` when constructing.
    ///
    /// # Errors
    /// Returns `NpuDispatchError::DispatchFailed` on hardware errors or timeouts.
    fn dispatch(
        &mut self,
        model: NpuModelHandle,
        input: Cow<'_, [f32]>,
    ) -> Result<DispatchResult, NpuDispatchError>;

    /// Measure current power draw in milliwatts.
    ///
    /// # Errors
    /// Returns error if the device doesn't support power monitoring.
    fn power_mw(&self) -> Result<f32, NpuDispatchError>;

    /// Check if the device is still reachable and ready.
    fn is_alive(&self) -> bool;
}

/// Opaque handle to a model loaded on an NPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NpuModelHandle(u32);

impl NpuModelHandle {
    /// Create a new model handle.
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the handle's numeric ID.
    pub const fn id(&self) -> u32 {
        self.0
    }
}

/// Adapter: wraps an `akida_driver::NpuBackend` as a generic `NpuDispatch`.
///
/// This is the bridge between toadStool's generic dispatch layer and the
/// Akida-specific driver.
#[derive(Debug)]
pub struct AkidaNpuDispatch {
    info: NpuInfo,
    backend: Box<dyn akida_driver::NpuBackend>,
}

impl AkidaNpuDispatch {
    /// Create from an already-initialized Akida backend.
    pub fn from_backend(backend: Box<dyn akida_driver::NpuBackend>) -> Self {
        let caps = backend.capabilities();
        let mut capabilities = vec![NpuCapability::Inference, NpuCapability::PowerMonitoring];

        if caps.weight_mutation != akida_driver::WeightMutationSupport::None {
            capabilities.push(NpuCapability::OnChipLearning);
        }
        if caps.batch.as_ref().is_some_and(|b| b.max_batch > 1) {
            capabilities.push(NpuCapability::BatchInference);
        }
        capabilities.push(NpuCapability::ReservoirComputing);

        let name = format!("Akida {:?}", caps.chip_version);
        let vendor = "brainchip".to_string();

        let info = NpuInfo {
            name,
            vendor,
            processing_elements: caps.npu_count,
            memory_bytes: u64::from(caps.memory_mb) * 1024 * 1024,
            capabilities,
        };

        Self { info, backend }
    }

    /// Discover and initialize the best available Akida backend.
    ///
    /// # Errors
    /// Returns error if no Akida device is found or initialization fails.
    pub fn discover(device_id: &str) -> Result<Self, NpuDispatchError> {
        let backend = akida_driver::select_backend(akida_driver::BackendSelection::Auto, device_id)
            .map_err(|e| NpuDispatchError::DeviceNotReady {
                reason: e.to_string(),
            })?;
        Ok(Self::from_backend(backend))
    }
}

impl NpuDispatch for AkidaNpuDispatch {
    fn info(&self) -> &NpuInfo {
        &self.info
    }

    fn load_model(&mut self, model_data: &[u8]) -> Result<NpuModelHandle, NpuDispatchError> {
        let handle =
            self.backend
                .load_model(model_data)
                .map_err(|e| NpuDispatchError::ModelLoadFailed {
                    reason: e.to_string(),
                })?;
        Ok(NpuModelHandle::new(handle.id()))
    }

    fn dispatch(
        &mut self,
        _model: NpuModelHandle,
        input: Cow<'_, [f32]>,
    ) -> Result<DispatchResult, NpuDispatchError> {
        let start = std::time::Instant::now();
        let output = self
            .backend
            .infer(&input)
            .map_err(|e| NpuDispatchError::DispatchFailed {
                reason: e.to_string(),
            })?;
        let latency_us = start.elapsed().as_micros() as u64;

        let power_mw = self.backend.measure_power().ok();

        Ok(DispatchResult {
            output,
            latency_us,
            power_mw,
        })
    }

    fn power_mw(&self) -> Result<f32, NpuDispatchError> {
        self.backend
            .measure_power()
            .map_err(|e| NpuDispatchError::DispatchFailed {
                reason: e.to_string(),
            })
    }

    fn is_alive(&self) -> bool {
        self.backend.is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
