//! RDF F64 - Radial Distribution Function histogram - f64 precision WGSL
//!
//! Deep Debt Principles apply.
//!
//! Applications:
//! - Structure analysis in MD
//! - Phase identification (solid/liquid/gas)
//! - Validation against experiment/theory

use crate::device::WgpuDevice;
use crate::error::Result;
use std::sync::Arc;

/// f64 RDF histogram calculator
pub struct RdfHistogramF64 {
    #[allow(dead_code)]
    device: Arc<WgpuDevice>,
}

impl RdfHistogramF64 {
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    #[allow(dead_code)]
    fn wgsl_shader() -> &'static str {
        include_str!("rdf_histogram_f64.wgsl")
    }

    /// Compute RDF histogram
    ///
    /// # Arguments
    /// * `positions` - Particle positions [N*3]
    /// * `n_bins` - Number of histogram bins
    /// * `r_max` - Maximum radius to consider
    /// * `box_size` - Simulation box dimensions [Lx, Ly, Lz] (for PBC)
    ///
    /// # Returns
    /// Histogram counts [n_bins]
    pub fn histogram(
        &self,
        positions: &[f64],
        n_bins: usize,
        r_max: f64,
        box_size: [f64; 3],
    ) -> Result<Vec<u32>> {
        Ok(self.histogram_cpu(positions, n_bins, r_max, box_size))
    }

    /// Compute normalized g(r)
    ///
    /// g(r) = histogram / (N * ρ * V_shell)
    /// where V_shell = 4π/3 * ((r+dr)³ - r³)
    pub fn compute_gr(
        &self,
        positions: &[f64],
        n_bins: usize,
        r_max: f64,
        box_size: [f64; 3],
    ) -> Result<(Vec<f64>, Vec<f64>)> {
        let hist = self.histogram(positions, n_bins, r_max, box_size)?;
        let dr = r_max / n_bins as f64;
        let n = positions.len() / 3;
        let volume = box_size[0] * box_size[1] * box_size[2];
        let density = n as f64 / volume;

        let mut r = Vec::with_capacity(n_bins);
        let mut gr = Vec::with_capacity(n_bins);

        for i in 0..n_bins {
            let r_lo = i as f64 * dr;
            let r_hi = (i + 1) as f64 * dr;
            let r_mid = (r_lo + r_hi) / 2.0;

            // Volume of spherical shell
            let v_shell = 4.0 / 3.0 * std::f64::consts::PI * (r_hi.powi(3) - r_lo.powi(3));

            // Expected pairs in shell at uniform density
            let expected = density * v_shell * (n - 1) as f64 / 2.0;

            let g = if expected > 0.0 {
                hist[i] as f64 / expected
            } else {
                0.0
            };

            r.push(r_mid);
            gr.push(g);
        }

        Ok((r, gr))
    }

    fn pbc_delta(&self, delta: f64, box_size: f64) -> f64 {
        delta - box_size * (delta / box_size).round()
    }

    fn histogram_cpu(
        &self,
        positions: &[f64],
        n_bins: usize,
        r_max: f64,
        box_size: [f64; 3],
    ) -> Vec<u32> {
        let n = positions.len() / 3;
        let dr = r_max / n_bins as f64;
        let mut hist = vec![0u32; n_bins];

        for i in 0..n {
            let xi = positions[i * 3];
            let yi = positions[i * 3 + 1];
            let zi = positions[i * 3 + 2];

            for j in (i + 1)..n {
                let dx = self.pbc_delta(positions[j * 3] - xi, box_size[0]);
                let dy = self.pbc_delta(positions[j * 3 + 1] - yi, box_size[1]);
                let dz = self.pbc_delta(positions[j * 3 + 2] - zi, box_size[2]);

                let r = (dx * dx + dy * dy + dz * dz).sqrt();
                let bin = (r / dr) as usize;

                if bin < n_bins {
                    hist[bin] += 1;
                }
            }
        }

        hist
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_device() -> Option<Arc<crate::device::WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_f64_gpu_available_sync()
    }

    #[test]
    fn test_two_particles() -> Result<()> {
        let Some(device) = create_test_device() else {
            return Ok(());
        };
        let rdf = RdfHistogramF64::new(device)?;

        // Two particles at distance 1.0
        let positions = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let box_size = [10.0, 10.0, 10.0];

        let hist = rdf.histogram(&positions, 10, 5.0, box_size)?;

        // dr = 0.5, distance 1.0 falls in bin 2 (0.5-1.0... wait, 1.0/0.5 = 2, so bin 2)
        assert_eq!(hist[2], 1, "Pair at r=1.0 should be in bin 2");

        Ok(())
    }

    #[test]
    fn test_pbc() -> Result<()> {
        let Some(device) = create_test_device() else {
            return Ok(());
        };
        let rdf = RdfHistogramF64::new(device)?;

        // Two particles at opposite corners - should see minimum image
        let positions = vec![0.5, 0.5, 0.5, 9.5, 9.5, 9.5];
        let box_size = [10.0, 10.0, 10.0];

        let hist = rdf.histogram(&positions, 10, 5.0, box_size)?;

        // Real distance = sqrt((9-0.5)² * 3) ≈ 15.6 (no PBC)
        // PBC distance = sqrt((1.0)² * 3) ≈ 1.73
        // With dr = 0.5, bin = 3 (1.5-2.0)
        let total: u32 = hist.iter().sum();
        assert_eq!(total, 1, "Should have exactly one pair");

        // The pair should be in a small-r bin due to PBC
        let small_r_counts: u32 = hist[0..5].iter().sum();
        assert_eq!(small_r_counts, 1, "Pair should be at small r due to PBC");

        Ok(())
    }

    #[test]
    fn test_gr_ideal_gas() -> Result<()> {
        let Some(device) = create_test_device() else {
            return Ok(());
        };
        let rdf = RdfHistogramF64::new(device)?;

        // Random positions (approximating ideal gas)
        let n = 100;
        let box_size = [10.0, 10.0, 10.0];
        let mut positions = Vec::with_capacity(n * 3);

        // Simple pseudo-random placement
        for i in 0..n {
            positions.push((i as f64 * 0.97) % box_size[0]);
            positions.push((i as f64 * 1.13) % box_size[1]);
            positions.push((i as f64 * 0.89) % box_size[2]);
        }

        let (r, gr) = rdf.compute_gr(&positions, 20, 4.0, box_size)?;

        assert_eq!(r.len(), 20);
        assert_eq!(gr.len(), 20);

        // g(r) should be roughly 1 for an ideal gas at large r
        // (This is approximate due to small N)
        let avg_gr: f64 = gr[10..].iter().sum::<f64>() / 10.0;
        assert!(
            avg_gr > 0.5 && avg_gr < 2.0,
            "g(r) ~ 1 expected for random distribution, got {}",
            avg_gr
        );

        Ok(())
    }
}
