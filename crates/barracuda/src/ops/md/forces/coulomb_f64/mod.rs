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

#[cfg(test)]
mod cpu_reference;

#[cfg(test)]
mod tests;

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

        self.compute_gpu_with_energy(positions, charges, k, cutoff, eps)
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

        let charges_buf =
            self.device
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

        let bind_group = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
            .compile_shader_f64(Self::wgsl_shader(), Some("Coulomb f64"));

        let pipeline_layout =
            self.device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Coulomb f64 Pipeline Layout"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline =
            self.device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Coulomb f64 Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point,
                    cache: None,
                    compilation_options: Default::default(),
                });

        let mut encoder =
            self.device
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
            tx.send(result)
                .expect("map_async callback: receiver must be waiting");
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

        let charges_buf =
            self.device
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

        let bind_group = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
            .compile_shader_f64(Self::wgsl_shader(), Some("Coulomb f64 Energy"));

        let pipeline_layout =
            self.device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Coulomb f64 Energy Pipeline Layout"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline =
            self.device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Coulomb f64 Energy Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "coulomb_with_energy_f64",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let mut encoder =
            self.device
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
            tx1.send(result)
                .expect("map_async callback: receiver must be waiting");
        });

        // Map energy
        let energy_slice = energy_staging.slice(..);
        let (tx2, rx2) = std::sync::mpsc::channel();
        energy_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx2.send(result)
                .expect("map_async callback: receiver must be waiting");
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
