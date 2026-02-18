//! VELOCITY-VERLET F64 - Symplectic integrator - f64 precision WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//!
//! Applications:
//! - Molecular dynamics
//! - N-body simulations
//! - Long-time energy conservation

use crate::device::WgpuDevice;
use crate::error::Result;
use std::sync::Arc;

/// f64 Velocity-Verlet integrator
///
/// Algorithm:
/// 1. x(t+Δt) = x(t) + v(t)Δt + ½a(t)Δt²
/// 2. v(t+Δt) = v(t) + ½[a(t) + a(t+Δt)]Δt
pub struct VelocityVerletF64 {
    #[allow(dead_code)]
    device: Arc<WgpuDevice>,
}

impl VelocityVerletF64 {
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    #[allow(dead_code)]
    fn wgsl_shader() -> &'static str {
        include_str!("velocity_verlet_f64.wgsl")
    }

    /// Perform full Velocity-Verlet step
    ///
    /// # Arguments
    /// * `positions` - Current positions [N*3]
    /// * `velocities` - Current velocities [N*3]
    /// * `forces_old` - Forces at time t [N*3]
    /// * `forces_new` - Forces at time t+Δt [N*3]
    /// * `masses` - Particle masses [N]
    /// * `dt` - Time step
    ///
    /// # Returns
    /// (new_positions, new_velocities)
    pub fn step(
        &self,
        positions: &[f64],
        velocities: &[f64],
        forces_old: &[f64],
        forces_new: &[f64],
        masses: &[f64],
        dt: f64,
    ) -> Result<(Vec<f64>, Vec<f64>)> {
        // CPU implementation
        Ok(self.step_cpu(positions, velocities, forces_old, forces_new, masses, dt))
    }

    /// Half-step velocity update (first half of leapfrog)
    pub fn velocity_half_step(
        &self,
        velocities: &[f64],
        forces: &[f64],
        masses: &[f64],
        dt: f64,
    ) -> Result<Vec<f64>> {
        let n = velocities.len() / 3;
        let half_dt = 0.5 * dt;
        let mut new_vel = vec![0.0; n * 3];

        for i in 0..n {
            let inv_m = 1.0 / masses[i];
            for k in 0..3 {
                let idx = i * 3 + k;
                new_vel[idx] = velocities[idx] + forces[idx] * inv_m * half_dt;
            }
        }

        Ok(new_vel)
    }

    /// Position update using velocities
    pub fn position_update(
        &self,
        positions: &[f64],
        velocities: &[f64],
        dt: f64,
    ) -> Result<Vec<f64>> {
        let n = positions.len() / 3;
        let mut new_pos = vec![0.0; n * 3];

        for i in 0..(n * 3) {
            new_pos[i] = positions[i] + velocities[i] * dt;
        }

        Ok(new_pos)
    }

    fn step_cpu(
        &self,
        positions: &[f64],
        velocities: &[f64],
        forces_old: &[f64],
        forces_new: &[f64],
        masses: &[f64],
        dt: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        let n = positions.len() / 3;
        let dt_sq = dt * dt;
        let mut new_pos = vec![0.0; n * 3];
        let mut new_vel = vec![0.0; n * 3];

        for i in 0..n {
            let inv_m = 1.0 / masses[i];

            for k in 0..3 {
                let idx = i * 3 + k;

                let a_old = forces_old[idx] * inv_m;
                let a_new = forces_new[idx] * inv_m;

                // x(t+Δt) = x(t) + v(t)Δt + ½a(t)Δt²
                new_pos[idx] = positions[idx] + velocities[idx] * dt + 0.5 * a_old * dt_sq;

                // v(t+Δt) = v(t) + ½[a(t) + a(t+Δt)]Δt
                new_vel[idx] = velocities[idx] + 0.5 * (a_old + a_new) * dt;
            }
        }

        (new_pos, new_vel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_device() -> Option<Arc<crate::device::WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_f64_gpu_available_sync()
    }

    #[test]
    fn test_free_particle() -> Result<()> {
        let Some(device) = create_test_device() else {
            return Ok(());
        };
        let vv = VelocityVerletF64::new(device)?;

        // Particle moving with constant velocity (no force)
        let pos = vec![0.0, 0.0, 0.0];
        let vel = vec![1.0, 2.0, 3.0];
        let forces = vec![0.0, 0.0, 0.0];
        let masses = vec![1.0];
        let dt = 0.1;

        let (new_pos, new_vel) = vv.step(&pos, &vel, &forces, &forces, &masses, dt)?;

        // Position: x = x₀ + v*dt
        assert!((new_pos[0] - 0.1).abs() < 1e-10);
        assert!((new_pos[1] - 0.2).abs() < 1e-10);
        assert!((new_pos[2] - 0.3).abs() < 1e-10);

        // Velocity unchanged (no acceleration)
        assert!((new_vel[0] - 1.0).abs() < 1e-10);
        assert!((new_vel[1] - 2.0).abs() < 1e-10);
        assert!((new_vel[2] - 3.0).abs() < 1e-10);

        Ok(())
    }

    #[test]
    fn test_constant_acceleration() -> Result<()> {
        let Some(device) = create_test_device() else {
            return Ok(());
        };
        let vv = VelocityVerletF64::new(device)?;

        // Particle under constant force
        let pos = vec![0.0, 0.0, 0.0];
        let vel = vec![0.0, 0.0, 0.0];
        let forces = vec![1.0, 0.0, 0.0]; // F = 1 in x
        let masses = vec![1.0];
        let dt = 0.1;

        let (new_pos, new_vel) = vv.step(&pos, &vel, &forces, &forces, &masses, dt)?;

        // a = F/m = 1
        // x = ½at² = 0.005
        // v = at = 0.1
        assert!((new_pos[0] - 0.005).abs() < 1e-10);
        assert!((new_vel[0] - 0.1).abs() < 1e-10);

        Ok(())
    }

    #[test]
    fn test_symplectic_energy_conservation() -> Result<()> {
        let Some(device) = create_test_device() else {
            return Ok(());
        };
        let vv = VelocityVerletF64::new(device)?;

        // Simple harmonic oscillator: F = -kx, k=1
        let mut pos = vec![1.0, 0.0, 0.0]; // Initial displacement
        let mut vel = vec![0.0, 0.0, 0.0];
        let masses = vec![1.0];
        let dt = 0.01;

        // Compute initial energy: E = ½kx² + ½mv² = 0.5
        let initial_energy = 0.5 * pos[0] * pos[0] + 0.5 * vel[0] * vel[0];

        // Run for many steps
        for _ in 0..1000 {
            let forces_old = vec![-pos[0], 0.0, 0.0]; // F = -x
                                                      // Half step for position
            let half_vel = vv.velocity_half_step(&vel, &forces_old, &masses, dt)?;
            pos = vv.position_update(&pos, &half_vel, dt)?;
            let forces_new = vec![-pos[0], 0.0, 0.0];
            // Half step for velocity
            vel = vv.velocity_half_step(&half_vel, &forces_new, &masses, dt)?;
        }

        // Check energy conservation
        let final_energy = 0.5 * pos[0] * pos[0] + 0.5 * vel[0] * vel[0];
        let rel_err = (final_energy - initial_energy).abs() / initial_energy;

        assert!(
            rel_err < 1e-4, // 0.01% tolerance for 1000-step integration
            "Energy drift {} too large ({}% error)",
            final_energy - initial_energy,
            rel_err * 100.0
        );

        Ok(())
    }
}
