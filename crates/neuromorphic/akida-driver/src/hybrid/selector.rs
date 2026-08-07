// SPDX-License-Identifier: AGPL-3.0-or-later

//! Runtime substrate selector for toadStool's NPU dispatch.

use crate::error::Result;

use super::{EsnSubstrate, HybridEsn, SubstrateInfo, SubstrateMode};

/// Runtime substrate selector for toadStool's NPU dispatch system.
///
/// Discovers available substrates at construction time and selects the
/// optimal one (hardware if present, software if not). toadStool calls
/// `esn_step()` without knowing which substrate is executing.
///
/// ```no_run
/// use akida_driver::SubstrateSelector;
///
/// # let (w_in, w_res, w_out) = (vec![0.1f32; 128*4], vec![0.05f32; 128*128], vec![0.2f32; 128]);
/// # let features = vec![0.0f32; 4];
/// let mut selector = SubstrateSelector::for_weights(
///     &w_in, &w_res, &w_out, 0.3,
/// )?;
/// println!("Substrate: {}", selector.active_substrate().mode.description());
///
/// let prediction = selector.esn_step(&features)?;
/// # Ok::<(), akida_driver::AkidaError>(())
/// ```
pub struct SubstrateSelector {
    esn: HybridEsn,
}

impl SubstrateSelector {
    /// Build a selector with the given weights, auto-discovering hardware.
    ///
    /// Tries hardware discovery; falls back to software if no NPU found.
    ///
    /// # Errors
    ///
    /// Returns error only if weights are invalid. Hardware unavailability is
    /// silently handled by falling back to software.
    pub fn for_weights(w_in: &[f32], w_res: &[f32], w_out: &[f32], leak_rate: f32) -> Result<Self> {
        let esn = HybridEsn::from_weights(w_in, w_res, w_out, leak_rate)?;
        Ok(Self { esn })
    }

    /// Build from a pre-constructed `HybridEsn`.
    #[must_use]
    pub const fn from_esn(esn: HybridEsn) -> Self {
        Self { esn }
    }

    /// Active substrate information for toadStool's scheduler/telemetry.
    #[must_use]
    pub fn active_substrate(&self) -> SubstrateInfo {
        let mode = self.esn.mode().clone();
        SubstrateInfo {
            est_hz: self.esn.estimated_hz(),
            est_energy_uj: self.esn.estimated_energy_uj(),
            tanh_accurate: mode.is_tanh_accurate(),
            npu_nps: match &mode {
                SubstrateMode::PureSoftware => 0,
                _ => self.esn.weights().reservoir_dim,
            },
            mode,
        }
    }

    /// Single-step inference — dispatches to the active substrate.
    ///
    /// # Errors
    ///
    /// Returns error if the active substrate fails.
    pub fn esn_step(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        self.esn.step(input)
    }

    /// Reset reservoir state (call between independent input sequences).
    pub fn reset(&mut self) {
        self.esn.reset();
    }

    /// Expose the inner `HybridEsn` for direct access if needed.
    #[must_use]
    pub const fn inner(&mut self) -> &mut HybridEsn {
        &mut self.esn
    }
}
