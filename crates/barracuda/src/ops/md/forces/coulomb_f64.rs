//! Coulomb Force Calculation (f64)
//!
//! **Physics**: Electrostatic interactions between charged particles
//! **Formula**: F = k * q_i * q_j / r² * r̂
//! **Use Case**: Ions, proteins, charged molecules, nuclei
//!
//! **Deep Debt Compliance**:
//! - ✅ Pure WGSL shader (f64)
//! - ✅ Zero unsafe code
//! - ✅ Capability-based dispatch
//! - ✅ Agnostic (no hardcoded constants)
//!
//! **Precision**: f64 is critical for:
//! - Large systems where small forces accumulate
//! - Nuclear physics (fine structure constant precision)
//! - Long timescale simulations

use crate::device::capabilities::WORKGROUP_SIZE_1D;
use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// f64 Coulomb force calculation operation
///
/// Computes electrostatic forces between all particle pairs.
/// Uses softened potential to avoid singularities.
pub struct CoulombForceF64 {
    device: Arc<WgpuDevice>,
}

impl CoulombForceF64 {
    /// Create new Coulomb f64 force calculation
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("coulomb_f64.wgsl")
    }

    /// Execute Coulomb force calculation
    ///
    /// # Arguments
    /// * `positions` - Particle positions [N*3] (x,y,z interleaved)
    /// * `charges` - Particle charges [N]
    /// * `coulomb_constant` - Coulomb constant k (default: 1.0)
    /// * `cutoff_radius` - Cutoff distance (default: infinity)
    /// * `softening` - Softening parameter (default: 1e-10)
    ///
    /// # Returns
    /// Force vectors [N*3] containing force for each particle
    pub fn compute_forces(
        &self,
        positions: &[f64],
        charges: &[f64],
        coulomb_constant: Option<f64>,
        cutoff_radius: Option<f64>,
        softening: Option<f64>,
    ) -> Result<Vec<f64>> {
        let n = charges.len();
        if positions.len() != n * 3 {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Position length {} != 3 * charges length {}",
                    positions.len(),
                    n * 3
                ),
            });
        }

        if n < 32 {
            return Ok(self.compute_cpu(
                positions,
                charges,
                coulomb_constant.unwrap_or(1.0),
                cutoff_radius.unwrap_or(f64::INFINITY),
                softening.unwrap_or(1e-10),
            ));
        }

        self.compute_gpu(
            positions,
            charges,
            coulomb_constant.unwrap_or(1.0),
            cutoff_radius.unwrap_or(f64::INFINITY),
            softening.unwrap_or(1e-10),
            "coulomb_f64",
        )
    }

    /// Compute forces with potential energy output
    pub fn compute_forces_and_energy(
        &self,
        positions: &[f64],
        charges: &[f64],
        coulomb_constant: Option<f64>,
        cutoff_radius: Option<f64>,
        softening: Option<f64>,
    ) -> Result<(Vec<f64>, Vec<f64>)> {
        let n = charges.len();
        if positions.len() != n * 3 {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Position length {} != 3 * charges length {}",
                    positions.len(),
                    n * 3
                ),
            });
        }

        let k = coulomb_constant.unwrap_or(1.0);
        let cutoff = cutoff_radius.unwrap_or(f64::INFINITY);
        let eps = softening.unwrap_or(1e-10);

        if n < 32 {
            return Ok(self.compute_cpu_with_energy(positions, charges, k, cutoff, eps));
        }

        self.compute_gpu_with_energy(positions, charges, k, cutoff, eps)
    }

    fn compute_cpu(
        &self,
        positions: &[f64],
        charges: &[f64],
        k: f64,
        cutoff: f64,
        eps: f64,
    ) -> Vec<f64> {
        let n = charges.len();
        let cutoff_sq = cutoff * cutoff;
        let eps_sq = eps * eps;
        let mut forces = vec![0.0f64; n * 3];

        for i in 0..n {
            let xi = positions[i * 3];
            let yi = positions[i * 3 + 1];
            let zi = positions[i * 3 + 2];
            let qi = charges[i];

            for j in 0..n {
                if i == j {
                    continue;
                }

                let xj = positions[j * 3];
                let yj = positions[j * 3 + 1];
                let zj = positions[j * 3 + 2];
                let qj = charges[j];

                let dx = xj - xi;
                let dy = yj - yi;
                let dz = zj - zi;

                let r_sq = dx * dx + dy * dy + dz * dz + eps_sq;
                if r_sq > cutoff_sq {
                    continue;
                }

                let r = r_sq.sqrt();
                let force_magnitude = k * qi * qj / r_sq;
                let force_over_r = -force_magnitude / r;

                forces[i * 3] += force_over_r * dx;
                forces[i * 3 + 1] += force_over_r * dy;
                forces[i * 3 + 2] += force_over_r * dz;
            }
        }

        forces
    }

    fn compute_cpu_with_energy(
        &self,
        positions: &[f64],
        charges: &[f64],
        k: f64,
        cutoff: f64,
        eps: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        let n = charges.len();
        let cutoff_sq = cutoff * cutoff;
        let eps_sq = eps * eps;
        let mut forces = vec![0.0f64; n * 3];
        let mut energies = vec![0.0f64; n];

        for i in 0..n {
            let xi = positions[i * 3];
            let yi = positions[i * 3 + 1];
            let zi = positions[i * 3 + 2];
            let qi = charges[i];

            for j in 0..n {
                if i == j {
                    continue;
                }

                let xj = positions[j * 3];
                let yj = positions[j * 3 + 1];
                let zj = positions[j * 3 + 2];
                let qj = charges[j];

                let dx = xj - xi;
                let dy = yj - yi;
                let dz = zj - zi;

                let r_sq = dx * dx + dy * dy + dz * dz + eps_sq;
                if r_sq > cutoff_sq {
                    continue;
                }

                let r = r_sq.sqrt();
                let force_magnitude = k * qi * qj / r_sq;
                let force_over_r = -force_magnitude / r;

                forces[i * 3] += force_over_r * dx;
                forces[i * 3 + 1] += force_over_r * dy;
                forces[i * 3 + 2] += force_over_r * dz;

                // Half to avoid double counting
                energies[i] += 0.5 * k * qi * qj / r;
            }
        }

        (forces, energies)
    }

    fn compute_gpu(
        &self,
        positions: &[f64],
        charges: &[f64],
        k: f64,
        cutoff: f64,
        eps: f64,
        entry_point: &str,
    ) -> Result<Vec<f64>> {
        let n = charges.len();

        let pos_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Coulomb f64 Positions"),
                contents: bytemuck::cast_slice(positions),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let charges_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Coulomb f64 Charges"),
                contents: bytemuck::cast_slice(charges),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let forces_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Coulomb f64 Forces"),
            size: (n * 3 * std::mem::size_of::<f64>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            n_particles: u32,
            _pad0: u32,
            coulomb_constant: f64,
            cutoff_radius: f64,
            cutoff_radius_sq: f64,
            softening: f64,
        }

        let params = Params {
            n_particles: n as u32,
            _pad0: 0,
            coulomb_constant: k,
            cutoff_radius: cutoff,
            cutoff_radius_sq: cutoff * cutoff,
            softening: eps,
        };

        let params_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Coulomb f64 Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bgl = self
            .device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Coulomb f64 BGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bind_group = self.device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Coulomb f64 Bind Group"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: pos_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: charges_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: forces_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buf.as_entire_binding(),
                },
            ],
        });

        let shader = self
            .device
            .compile_shader(Self::wgsl_shader(), Some("Coulomb f64"));

        let pipeline_layout =
            self.device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Coulomb f64 Pipeline Layout"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline = self
            .device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Coulomb f64 Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point,
            cache: None,
            compilation_options: Default::default(),
            });

        let mut encoder = self
            .device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Coulomb f64 Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Coulomb f64 Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (n as u32).div_ceil(WORKGROUP_SIZE_1D);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Read back results
        let staging_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Coulomb f64 Staging"),
            size: (n * 3 * std::mem::size_of::<f64>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(
            &forces_buf,
            0,
            &staging_buf,
            0,
            (n * 3 * std::mem::size_of::<f64>()) as u64,
        );

        self.device.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.device.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| BarracudaError::Device(format!("Buffer map failed: {}", e)))?
            .map_err(|e| BarracudaError::Device(format!("Buffer map error: {:?}", e)))?;

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        Ok(result)
    }

    fn compute_gpu_with_energy(
        &self,
        positions: &[f64],
        charges: &[f64],
        k: f64,
        cutoff: f64,
        eps: f64,
    ) -> Result<(Vec<f64>, Vec<f64>)> {
        let n = charges.len();

        let pos_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Coulomb f64 Positions"),
                contents: bytemuck::cast_slice(positions),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let charges_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Coulomb f64 Charges"),
                contents: bytemuck::cast_slice(charges),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let forces_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Coulomb f64 Forces"),
            size: (n * 3 * std::mem::size_of::<f64>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let energy_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Coulomb f64 Energy"),
            size: std::mem::size_of_val(charges) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            n_particles: u32,
            _pad0: u32,
            coulomb_constant: f64,
            cutoff_radius: f64,
            cutoff_radius_sq: f64,
            softening: f64,
        }

        let params = Params {
            n_particles: n as u32,
            _pad0: 0,
            coulomb_constant: k,
            cutoff_radius: cutoff,
            cutoff_radius_sq: cutoff * cutoff,
            softening: eps,
        };

        let params_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Coulomb f64 Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bgl = self
            .device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Coulomb f64 Energy BGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bind_group = self.device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Coulomb f64 Energy Bind Group"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: pos_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: charges_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: forces_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: energy_buf.as_entire_binding(),
                },
            ],
        });

        let shader = self
            .device
            .compile_shader(Self::wgsl_shader(), Some("Coulomb f64 Energy"));

        let pipeline_layout =
            self.device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Coulomb f64 Energy Pipeline Layout"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline = self
            .device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Coulomb f64 Energy Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "coulomb_with_energy_f64",
            cache: None,
            compilation_options: Default::default(),
            });

        let mut encoder = self
            .device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Coulomb f64 Energy Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Coulomb f64 Energy Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (n as u32).div_ceil(WORKGROUP_SIZE_1D);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Read back forces
        let forces_staging = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Coulomb f64 Forces Staging"),
            size: (n * 3 * std::mem::size_of::<f64>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Read back energy
        let energy_staging = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Coulomb f64 Energy Staging"),
            size: std::mem::size_of_val(charges) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(
            &forces_buf,
            0,
            &forces_staging,
            0,
            (n * 3 * std::mem::size_of::<f64>()) as u64,
        );

        encoder.copy_buffer_to_buffer(
            &energy_buf,
            0,
            &energy_staging,
            0,
            std::mem::size_of_val(charges) as u64,
        );

        self.device.queue.submit(Some(encoder.finish()));

        // Map forces
        let forces_slice = forces_staging.slice(..);
        let (tx1, rx1) = std::sync::mpsc::channel();
        forces_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx1.send(result).unwrap();
        });

        // Map energy
        let energy_slice = energy_staging.slice(..);
        let (tx2, rx2) = std::sync::mpsc::channel();
        energy_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx2.send(result).unwrap();
        });

        self.device.device.poll(wgpu::Maintain::Wait);

        rx1.recv()
            .map_err(|e| BarracudaError::Device(format!("Forces buffer map failed: {}", e)))?
            .map_err(|e| BarracudaError::Device(format!("Forces buffer map error: {:?}", e)))?;

        rx2.recv()
            .map_err(|e| BarracudaError::Device(format!("Energy buffer map failed: {}", e)))?
            .map_err(|e| BarracudaError::Device(format!("Energy buffer map error: {:?}", e)))?;

        let forces_data = forces_slice.get_mapped_range();
        let forces: Vec<f64> = bytemuck::cast_slice(&forces_data).to_vec();
        drop(forces_data);
        forces_staging.unmap();

        let energy_data = energy_slice.get_mapped_range();
        let energies: Vec<f64> = bytemuck::cast_slice(&energy_data).to_vec();
        drop(energy_data);
        energy_staging.unmap();

        Ok((forces, energies))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_device() -> Option<Arc<crate::device::WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_f64_gpu_available_sync()
    }

    #[test]
    fn test_coulomb_f64_two_particles() {
        let Some(device) = get_test_device() else { return; };
        let op = CoulombForceF64::new(device).unwrap();

        // Two particles with opposite charges
        let positions = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0]; // Along x-axis, 1 unit apart
        let charges = vec![1.0, -1.0]; // Opposite charges

        let forces = op
            .compute_forces(&positions, &charges, Some(1.0), None, Some(1e-10))
            .unwrap();

        // Force should attract: F ~ k * q1 * q2 / r^2 = 1 * 1 * (-1) / 1 = -1
        // Particle 0 should be pulled toward particle 1 (positive x direction)
        assert!(forces[0] > 0.0, "F_x on particle 0 should be positive");
        assert!(forces[1].abs() < 1e-10, "F_y on particle 0 should be ~0");
        assert!(forces[2].abs() < 1e-10, "F_z on particle 0 should be ~0");

        // Particle 1 should be pulled toward particle 0 (negative x direction)
        assert!(forces[3] < 0.0, "F_x on particle 1 should be negative");

        // Forces should be equal and opposite
        assert!(
            (forces[0] + forces[3]).abs() < 1e-10,
            "Forces should sum to zero"
        );
    }

    #[test]
    fn test_coulomb_f64_repulsion() {
        let Some(device) = get_test_device() else { return; };
        let op = CoulombForceF64::new(device).unwrap();

        // Two particles with same charges
        let positions = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let charges = vec![1.0, 1.0]; // Same sign

        let forces = op
            .compute_forces(&positions, &charges, Some(1.0), None, Some(1e-10))
            .unwrap();

        // Force should repel: particle 0 pushed in negative x direction
        assert!(forces[0] < 0.0, "F_x on particle 0 should be negative (repulsion)");
        assert!(forces[3] > 0.0, "F_x on particle 1 should be positive (repulsion)");
    }

    #[test]
    fn test_coulomb_f64_distance_scaling() {
        let Some(device) = get_test_device() else { return; };
        let op = CoulombForceF64::new(device).unwrap();

        // Two particles at distance 1
        let positions1 = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let charges = vec![1.0, 1.0];

        let forces1 = op
            .compute_forces(&positions1, &charges, Some(1.0), None, Some(1e-10))
            .unwrap();

        // Two particles at distance 2
        let positions2 = vec![0.0, 0.0, 0.0, 2.0, 0.0, 0.0];
        let forces2 = op
            .compute_forces(&positions2, &charges, Some(1.0), None, Some(1e-10))
            .unwrap();

        // Force should scale as 1/r^2, so F(2) = F(1)/4
        let ratio = forces1[0].abs() / forces2[0].abs();
        assert!(
            (ratio - 4.0).abs() < 0.01,
            "Force should scale as 1/r^2, ratio = {}",
            ratio
        );
    }

    #[test]
    fn test_coulomb_f64_with_energy_gpu() {
        let Some(device) = get_test_device() else { return; };
        let op = CoulombForceF64::new(device).unwrap();

        // Need at least 32 particles to use GPU path
        let n = 40;
        let mut positions = vec![0.0; n * 3];
        let mut charges = vec![0.0; n];

        // Arrange particles in a line with alternating charges
        for i in 0..n {
            positions[i * 3] = i as f64; // x position
            charges[i] = if i % 2 == 0 { 1.0 } else { -1.0 };
        }

        let (forces, energies) = op
            .compute_forces_and_energy(&positions, &charges, Some(1.0), None, Some(1e-10))
            .unwrap();

        assert_eq!(forces.len(), n * 3, "Forces should have 3N elements");
        assert_eq!(energies.len(), n, "Energies should have N elements");

        // Total energy should be negative (attractive system with alternating charges)
        let total_energy: f64 = energies.iter().sum();
        assert!(total_energy < 0.0, "Total energy should be negative for alternating charges");

        // Forces on interior particles should be small (nearly balanced)
        // First and last particles see unbalanced forces
        let mid = n / 2;
        let fx_mid = forces[mid * 3].abs();
        let fx_first = forces[0].abs();

        // Interior forces should be smaller than boundary forces
        assert!(
            fx_mid < fx_first,
            "Interior force {} should be less than boundary force {}",
            fx_mid, fx_first
        );
    }
}
