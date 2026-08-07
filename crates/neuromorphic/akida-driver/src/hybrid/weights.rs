// SPDX-License-Identifier: AGPL-3.0-or-later

//! ESN weight container and validation.

use crate::error::{AkidaError, Result};

/// ESN weight matrices exported from hotSpring (or any training framework).
///
/// All weights are in tanh-training format (f32, row-major).
/// No quantization, no bounded-ReLU re-optimization required.
#[derive(Debug, Clone)]
pub struct EsnWeights {
    /// Input projection: `[reservoir_dim × input_dim]` row-major
    pub w_in: Vec<f32>,
    /// Recurrent weights: `[reservoir_dim × reservoir_dim]` row-major
    pub w_res: Vec<f32>,
    /// Readout weights: `[output_dim × reservoir_dim]` row-major
    pub w_out: Vec<f32>,
    /// Input dimensionality
    pub input_dim: usize,
    /// Reservoir dimensionality (number of NPs on hardware)
    pub reservoir_dim: usize,
    /// Output dimensionality
    pub output_dim: usize,
    /// Leak rate α ∈ (0, 1]
    pub leak_rate: f32,
}

impl EsnWeights {
    /// Construct from raw weight slices.
    ///
    /// Validates dimensions before accepting.
    ///
    /// # Errors
    ///
    /// Returns error if slice lengths are inconsistent with declared dimensions.
    pub fn new(
        w_in: Vec<f32>,
        w_res: Vec<f32>,
        w_out: Vec<f32>,
        input_dim: usize,
        reservoir_dim: usize,
        output_dim: usize,
        leak_rate: f32,
    ) -> Result<Self> {
        if w_in.len() != reservoir_dim * input_dim {
            return Err(AkidaError::capability_query_failed(format!(
                "w_in: expected {}×{}={}, got {}",
                reservoir_dim,
                input_dim,
                reservoir_dim * input_dim,
                w_in.len()
            )));
        }
        if w_res.len() != reservoir_dim * reservoir_dim {
            return Err(AkidaError::capability_query_failed(format!(
                "w_res: expected {}²={}, got {}",
                reservoir_dim,
                reservoir_dim * reservoir_dim,
                w_res.len()
            )));
        }
        if w_out.len() != output_dim * reservoir_dim {
            return Err(AkidaError::capability_query_failed(format!(
                "w_out: expected {}×{}={}, got {}",
                output_dim,
                reservoir_dim,
                output_dim * reservoir_dim,
                w_out.len()
            )));
        }
        if !(0.0..=1.0).contains(&leak_rate) {
            return Err(AkidaError::capability_query_failed(format!(
                "leak_rate {leak_rate} must be in (0, 1]"
            )));
        }
        Ok(Self {
            w_in,
            w_res,
            w_out,
            input_dim,
            reservoir_dim,
            output_dim,
            leak_rate,
        })
    }

    /// Spectral radius of `w_res` (rough estimate via power iteration).
    ///
    /// An ESN with tanh needs spectral radius < 1 for echo state property.
    /// Hardware ESNs may use higher values (bounded `ReLU` prevents explosion).
    /// After hybrid migration, ensure spectral radius < 1.
    #[must_use]
    pub fn spectral_radius_estimate(&self, iters: usize) -> f32 {
        let rs = self.reservoir_dim;
        let mut v = vec![1.0f32 / (rs as f32).sqrt(); rs];
        for _ in 0..iters {
            let mut mv = vec![0.0f32; rs];
            for (i, mv_slot) in mv.iter_mut().enumerate() {
                for (j, vj) in v.iter().enumerate().take(rs) {
                    *mv_slot += self.w_res[i * rs + j] * vj;
                }
            }
            let norm = mv.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-10);
            for (vi, mvi) in v.iter_mut().zip(mv.iter()) {
                *vi = mvi / norm;
            }
            let rayleigh: f32 = v
                .iter()
                .enumerate()
                .map(|(i, &vi)| vi * mv[i].max(-norm).min(norm))
                .sum();
            if rayleigh.abs() > 0.0 {
                return rayleigh.abs();
            }
        }
        1.0
    }
}
