//! KINETIC ENERGY F64 - Per-particle and total kinetic energy - f64 precision WGSL
//!
//! Deep Debt Principles apply.
//!
//! Applications:
//! - Temperature calculation: T = 2*KE_total / (3*N*k_B)
//! - Energy monitoring in MD
//! - Thermostat validation

use crate::device::WgpuDevice;
use crate::error::Result;
use std::sync::Arc;

/// f64 Kinetic energy calculator
pub struct KineticEnergyF64 {
    #[allow(dead_code)]
    device: Arc<WgpuDevice>,
}

impl KineticEnergyF64 {
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    #[allow(dead_code)]
    fn wgsl_shader() -> &'static str {
        include_str!("kinetic_energy_f64.wgsl")
    }

    /// Compute per-particle kinetic energy KE_i = ½m_i v_i²
    ///
    /// # Arguments
    /// * `velocities` - Particle velocities [N*3]
    /// * `masses` - Particle masses [N]
    ///
    /// # Returns
    /// Per-particle kinetic energies [N]
    pub fn per_particle(
        &self,
        velocities: &[f64],
        masses: &[f64],
    ) -> Result<Vec<f64>> {
        Ok(self.per_particle_cpu(velocities, masses))
    }

    /// Compute total kinetic energy
    pub fn total(
        &self,
        velocities: &[f64],
        masses: &[f64],
    ) -> Result<f64> {
        let per_particle = self.per_particle(velocities, masses)?;
        Ok(per_particle.iter().sum())
    }

    /// Compute temperature from kinetic energy
    ///
    /// T = 2*KE_total / (3*N*k_B) for 3D system with N particles
    ///
    /// # Arguments
    /// * `velocities` - Particle velocities [N*3]
    /// * `masses` - Particle masses [N]
    /// * `k_b` - Boltzmann constant (in appropriate units)
    pub fn temperature(
        &self,
        velocities: &[f64],
        masses: &[f64],
        k_b: f64,
    ) -> Result<f64> {
        let n = masses.len();
        if n == 0 {
            return Ok(0.0);
        }
        let ke_total = self.total(velocities, masses)?;
        // T = 2*KE / (3*N*k_B)
        Ok(2.0 * ke_total / (3.0 * n as f64 * k_b))
    }

    fn per_particle_cpu(&self, velocities: &[f64], masses: &[f64]) -> Vec<f64> {
        let n = masses.len();
        let mut ke = Vec::with_capacity(n);

        for i in 0..n {
            let vx = velocities[i * 3];
            let vy = velocities[i * 3 + 1];
            let vz = velocities[i * 3 + 2];
            let v_sq = vx * vx + vy * vy + vz * vz;
            ke.push(0.5 * masses[i] * v_sq);
        }

        ke
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
    fn test_single_particle() -> Result<()> {
        let device = create_test_device()?;
        let ke_calc = KineticEnergyF64::new(device)?;

        let velocities = vec![1.0, 0.0, 0.0];
        let masses = vec![2.0];

        let per_particle = ke_calc.per_particle(&velocities, &masses)?;

        // KE = ½mv² = ½ * 2 * 1² = 1
        assert!((per_particle[0] - 1.0).abs() < 1e-10);

        Ok(())
    }

    #[test]
    fn test_3d_velocity() -> Result<()> {
        let device = create_test_device()?;
        let ke_calc = KineticEnergyF64::new(device)?;

        let velocities = vec![1.0, 2.0, 2.0]; // |v| = 3
        let masses = vec![1.0];

        let per_particle = ke_calc.per_particle(&velocities, &masses)?;

        // KE = ½ * 1 * 9 = 4.5
        assert!((per_particle[0] - 4.5).abs() < 1e-10);

        Ok(())
    }

    #[test]
    fn test_total_energy() -> Result<()> {
        let device = create_test_device()?;
        let ke_calc = KineticEnergyF64::new(device)?;

        let velocities = vec![
            1.0, 0.0, 0.0, // Particle 0
            0.0, 2.0, 0.0, // Particle 1
        ];
        let masses = vec![1.0, 1.0];

        let total = ke_calc.total(&velocities, &masses)?;

        // KE = ½*1*1 + ½*1*4 = 0.5 + 2.0 = 2.5
        assert!((total - 2.5).abs() < 1e-10);

        Ok(())
    }

    #[test]
    fn test_temperature() -> Result<()> {
        let device = create_test_device()?;
        let ke_calc = KineticEnergyF64::new(device)?;

        // Ideal gas: equipartition gives each DOF ½k_B T
        // For N particles in 3D: KE = 3/2 N k_B T
        // So T = 2 KE / (3 N k_B)

        let n = 100;
        let k_b = 1.0;
        let target_temp = 300.0;
        let target_ke_per_particle = 1.5 * k_b * target_temp;
        let v_rms = (2.0_f64 * target_ke_per_particle).sqrt(); // For m=1

        // Create velocities with RMS speed
        let mut velocities = vec![0.0; n * 3];
        for i in 0..n {
            // Distribute velocity isotropically (simplified)
            let scale = v_rms / 3.0_f64.sqrt();
            velocities[i * 3] = scale;
            velocities[i * 3 + 1] = scale;
            velocities[i * 3 + 2] = scale;
        }
        let masses = vec![1.0; n];

        let temp = ke_calc.temperature(&velocities, &masses, k_b)?;

        let rel_err = (temp - target_temp).abs() / target_temp;
        assert!(rel_err < 0.01, "Temperature {} not close to {}", temp, target_temp);

        Ok(())
    }
}
