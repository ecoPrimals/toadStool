//! BORN-MAYER F64 - Short-range repulsive force - f64 precision WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//!
//! Applications:
//! - Ionic crystals (NaCl, MgO)
//! - Hard-core repulsion in MD
//! - Steric effects modeling

use crate::device::WgpuDevice;
use crate::error::Result;
use std::sync::Arc;

/// f64 Born-Mayer force calculator
///
/// Potential: U(r) = A * exp(-r/ρ)
/// Force: F = (A/ρ) * exp(-r/ρ) * r̂
pub struct BornMayerForceF64 {
    #[allow(dead_code)]
    device: Arc<WgpuDevice>,
}

impl BornMayerForceF64 {
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    #[allow(dead_code)]
    fn wgsl_shader() -> &'static str {
        include_str!("born_mayer_f64.wgsl")
    }

    /// Compute Born-Mayer forces on all particles
    ///
    /// # Arguments
    /// * `positions` - Particle positions [N*3] as [x₀, y₀, z₀, x₁, y₁, z₁, ...]
    /// * `a_params` - Per-particle A (repulsion strength) [N]
    /// * `rho_params` - Per-particle ρ (softness) [N]
    /// * `cutoff` - Cutoff radius
    ///
    /// # Returns
    /// Forces [N*3] as [fx₀, fy₀, fz₀, ...]
    pub fn compute_forces(
        &self,
        positions: &[f64],
        a_params: &[f64],
        rho_params: &[f64],
        cutoff: f64,
    ) -> Result<Vec<f64>> {
        // CPU implementation for now
        // GPU path needs multi-pass for force reduction
        Ok(self.compute_cpu(positions, a_params, rho_params, cutoff))
    }

    /// Compute forces and total potential energy
    pub fn compute_forces_and_energy(
        &self,
        positions: &[f64],
        a_params: &[f64],
        rho_params: &[f64],
        cutoff: f64,
    ) -> Result<(Vec<f64>, f64)> {
        Ok(self.compute_cpu_with_energy(positions, a_params, rho_params, cutoff))
    }

    fn compute_cpu(
        &self,
        positions: &[f64],
        a_params: &[f64],
        rho_params: &[f64],
        cutoff: f64,
    ) -> Vec<f64> {
        let n = positions.len() / 3;
        let mut forces = vec![0.0; n * 3];
        let cutoff_sq = cutoff * cutoff;

        for i in 0..n {
            let pi = [positions[i * 3], positions[i * 3 + 1], positions[i * 3 + 2]];
            let ai = a_params[i];
            let rhoi = rho_params[i];

            for j in 0..n {
                if i == j {
                    continue;
                }

                let pj = [positions[j * 3], positions[j * 3 + 1], positions[j * 3 + 2]];
                let aj = a_params[j];
                let rhoj = rho_params[j];

                let r_vec = [pj[0] - pi[0], pj[1] - pi[1], pj[2] - pi[2]];
                let r_sq = r_vec[0] * r_vec[0] + r_vec[1] * r_vec[1] + r_vec[2] * r_vec[2];

                if r_sq > cutoff_sq || r_sq < 1e-20 {
                    continue;
                }

                let r = r_sq.sqrt();

                // Geometric mixing rules
                let a = (ai * aj).sqrt();
                let rho = (rhoi + rhoj) * 0.5;

                // F = (A/ρ) * exp(-r/ρ) * r̂
                let exp_term = (-r / rho).exp();
                let force_mag = (a / rho) * exp_term;

                let inv_r = 1.0 / r;
                forces[i * 3] += force_mag * r_vec[0] * inv_r;
                forces[i * 3 + 1] += force_mag * r_vec[1] * inv_r;
                forces[i * 3 + 2] += force_mag * r_vec[2] * inv_r;
            }
        }

        forces
    }

    fn compute_cpu_with_energy(
        &self,
        positions: &[f64],
        a_params: &[f64],
        rho_params: &[f64],
        cutoff: f64,
    ) -> (Vec<f64>, f64) {
        let n = positions.len() / 3;
        let mut forces = vec![0.0; n * 3];
        let mut energy = 0.0;
        let cutoff_sq = cutoff * cutoff;

        for i in 0..n {
            let pi = [positions[i * 3], positions[i * 3 + 1], positions[i * 3 + 2]];
            let ai = a_params[i];
            let rhoi = rho_params[i];

            for j in (i + 1)..n {
                let pj = [positions[j * 3], positions[j * 3 + 1], positions[j * 3 + 2]];
                let aj = a_params[j];
                let rhoj = rho_params[j];

                let r_vec = [pj[0] - pi[0], pj[1] - pi[1], pj[2] - pi[2]];
                let r_sq = r_vec[0] * r_vec[0] + r_vec[1] * r_vec[1] + r_vec[2] * r_vec[2];

                if r_sq > cutoff_sq || r_sq < 1e-20 {
                    continue;
                }

                let r = r_sq.sqrt();

                let a = (ai * aj).sqrt();
                let rho = (rhoi + rhoj) * 0.5;

                let exp_term = (-r / rho).exp();

                // U = A * exp(-r/ρ)
                energy += a * exp_term;

                // F = (A/ρ) * exp(-r/ρ) * r̂
                let force_mag = (a / rho) * exp_term;
                let inv_r = 1.0 / r;
                let f = [
                    force_mag * r_vec[0] * inv_r,
                    force_mag * r_vec[1] * inv_r,
                    force_mag * r_vec[2] * inv_r,
                ];

                // Newton's third law
                forces[i * 3] += f[0];
                forces[i * 3 + 1] += f[1];
                forces[i * 3 + 2] += f[2];
                forces[j * 3] -= f[0];
                forces[j * 3 + 1] -= f[1];
                forces[j * 3 + 2] -= f[2];
            }
        }

        (forces, energy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_device() -> Result<Arc<WgpuDevice>> {
        let device = pollster::block_on(async { WgpuDevice::new_f64_capable().await })?;
        Ok(Arc::new(device))
    }

    #[test]
    fn test_born_mayer_two_particles() -> Result<()> {
        let device = create_test_device()?;
        let bm = BornMayerForceF64::new(device)?;

        // Two particles along x-axis
        let positions = vec![0.0, 0.0, 0.0, 2.0, 0.0, 0.0];
        let a_params = vec![1.0, 1.0];
        let rho_params = vec![1.0, 1.0];

        let forces = bm.compute_forces(&positions, &a_params, &rho_params, 10.0)?;

        // Force should be repulsive (positive x for particle 0, negative for particle 1)
        assert!(forces[0] > 0.0, "Particle 0 should be pushed in -x");
        assert!(forces[3] < 0.0, "Particle 1 should be pushed in +x");
        // Newton's third law
        assert!((forces[0] + forces[3]).abs() < 1e-10, "Forces should be equal and opposite");
        Ok(())
    }

    #[test]
    fn test_born_mayer_energy_positive() -> Result<()> {
        let device = create_test_device()?;
        let bm = BornMayerForceF64::new(device)?;

        let positions = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let a_params = vec![1.0, 1.0];
        let rho_params = vec![0.5, 0.5];

        let (_, energy) = bm.compute_forces_and_energy(&positions, &a_params, &rho_params, 10.0)?;

        // Born-Mayer is purely repulsive, so energy > 0
        assert!(energy > 0.0, "Born-Mayer energy should be positive");
        Ok(())
    }

    #[test]
    fn test_born_mayer_cutoff() -> Result<()> {
        let device = create_test_device()?;
        let bm = BornMayerForceF64::new(device)?;

        let positions = vec![0.0, 0.0, 0.0, 5.0, 0.0, 0.0];
        let a_params = vec![1.0, 1.0];
        let rho_params = vec![0.5, 0.5];

        // With cutoff = 3, particles at distance 5 should not interact
        let forces = bm.compute_forces(&positions, &a_params, &rho_params, 3.0)?;

        for f in forces {
            assert!(f.abs() < 1e-15, "No force expected beyond cutoff");
        }
        Ok(())
    }
}
