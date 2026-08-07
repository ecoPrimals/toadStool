// SPDX-License-Identifier: AGPL-3.0-or-later

//! Substrate mode, trait, and info types.

use crate::error::{AkidaError, Result};

/// Which substrate is currently executing the ESN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubstrateMode {
    /// Pure CPU f32 with tanh activation (`SoftwareBackend`).
    /// Available today. Accuracy: hotSpring's validated software performance.
    PureSoftware,

    /// AKD1000 hardware linear transform + host tanh activation.
    /// Pending `metalForge/experiments/004_HYBRID_TANH` validation.
    /// Accuracy: same as software (tanh preserved). Throughput: 18,500 Hz.
    HardwareLinear,

    /// AKD1000 hardware with bounded `ReLU` (SDK default mode).
    /// Requires purpose-designed reservoir weights (MetaTF-trained).
    /// Accuracy: 86.1% on QCD (3.6% below software tanh).
    HardwareNative,
}

impl SubstrateMode {
    /// Human-readable description for logging and toadStool telemetry.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::PureSoftware => "CPU f32 + tanh  (~800 Hz, ~44 mJ/inf)",
            Self::HardwareLinear => "AKD1000 linear + host tanh  (18,500 Hz, 1.4 µJ/inf)",
            Self::HardwareNative => "AKD1000 bounded ReLU  (18,500 Hz, 1.4 µJ/inf, -3.6% acc)",
        }
    }

    /// Whether this substrate preserves tanh-trained weight accuracy.
    #[must_use]
    pub const fn is_tanh_accurate(&self) -> bool {
        matches!(self, Self::PureSoftware | Self::HardwareLinear)
    }
}

/// Unified interface for ESN inference across all substrates.
///
/// hotSpring implements its simulation runner against this trait.
/// toadStool's substrate dispatch uses this trait for NPU-aware scheduling.
///
/// All implementors must preserve the temporal state between calls — a `step()`
/// call advances the reservoir state, and the next call sees the updated state.
/// Call `reset()` to clear state between independent sequences.
pub trait EsnSubstrate: Send + Sync {
    /// Advance reservoir by one timestep and return readout.
    ///
    /// `input` must have length == `input_dim()`.
    /// Returns `output_dim()` float values.
    ///
    /// # Errors
    ///
    /// Returns error if input dimension mismatches or substrate is not ready.
    fn step(&mut self, input: &[f32]) -> Result<Vec<f32>>;

    /// Process a sequence of inputs, returning the final readout.
    ///
    /// Equivalent to calling `step()` `inputs.len() / input_dim()` times.
    ///
    /// # Errors
    ///
    /// Returns error if input length is not a multiple of `input_dim()`.
    fn run_sequence(&mut self, inputs: &[f32]) -> Result<Vec<f32>> {
        let is = self.input_dim();
        if !inputs.len().is_multiple_of(is) {
            return Err(AkidaError::capability_query_failed(format!(
                "run_sequence: input length {} not divisible by input_dim {}",
                inputs.len(),
                is
            )));
        }
        let mut out = vec![0.0f32; self.output_dim()];
        for chunk in inputs.chunks(is) {
            out = self.step(chunk)?;
        }
        Ok(out)
    }

    /// Reset reservoir state to zero (start of new sequence).
    fn reset(&mut self);

    /// Current reservoir state vector (for cross-substrate comparison / debug).
    fn reservoir_state(&self) -> Vec<f32>;

    /// Input dimension (number of floats expected per `step()` call).
    fn input_dim(&self) -> usize;

    /// Reservoir dimension (number of NPs / simulated neurons).
    fn reservoir_dim(&self) -> usize;

    /// Output dimension (number of floats returned per `step()` call).
    fn output_dim(&self) -> usize;

    /// Which substrate is executing this instance.
    fn substrate_mode(&self) -> SubstrateMode;

    /// Estimated throughput in inferences/second.
    ///
    /// Used by toadStool's scheduler to select the fastest available substrate.
    fn estimated_hz(&self) -> f64 {
        match self.substrate_mode() {
            SubstrateMode::PureSoftware => 800.0,
            SubstrateMode::HardwareLinear | SubstrateMode::HardwareNative => 18_500.0,
        }
    }

    /// Estimated energy per inference in µJ.
    fn estimated_energy_uj(&self) -> f64 {
        match self.substrate_mode() {
            SubstrateMode::PureSoftware => 44_000.0, // ~44 mJ
            SubstrateMode::HardwareLinear | SubstrateMode::HardwareNative => 1.4,
        }
    }
}

/// Substrate information returned to toadStool's scheduler.
#[derive(Debug, Clone)]
pub struct SubstrateInfo {
    /// Which mode is active.
    pub mode: SubstrateMode,
    /// Estimated throughput in inferences/second.
    pub est_hz: f64,
    /// Estimated energy per inference in µJ.
    pub est_energy_uj: f64,
    /// Whether tanh-trained weights are fully accurate on this substrate.
    pub tanh_accurate: bool,
    /// Number of NPs consumed (0 if software).
    pub npu_nps: usize,
}
