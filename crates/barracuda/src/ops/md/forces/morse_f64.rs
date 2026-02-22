//! Morse Force Calculation (f64)
//!
//! **Physics**: Anharmonic bonded interactions (chemical bonds)
//! **Potential**: U(r) = D·[1 - exp(-a(r-r₀))]²
//! **Use Case**: Molecular mechanics, reactive MD, bond stretching/breaking
//!
//! **Deep Debt Compliance**:
//! - ✅ Pure WGSL shader (f64)
//! - ✅ Zero unsafe code
//! - ✅ Capability-based dispatch
//! - ✅ Agnostic (no hardcoded constants)

use crate::device::capabilities::WORKGROUP_SIZE_1D;
use crate::device::WgpuDevice;
use crate::error::Result;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// f64 Morse force calculation for bonded interactions
///
/// Computes forces and energies for chemical bonds using Morse potential.
/// Two paths: GPU (2-pass shader) for large bond counts, CPU fallback.
pub struct MorseForceF64 {
    device: Arc<WgpuDevice>,
}

/// Parameters for a single Morse bond
#[derive(Clone, Copy, Debug)]
pub struct MorseBond {
    /// Particle index i
    pub i: u32,
    /// Particle index j
    pub j: u32,
    /// Dissociation energy D (eV or kJ/mol)
    pub dissociation_energy: f64,
    /// Width parameter a (1/Å or 1/nm)
    pub width_param: f64,
    /// Equilibrium bond distance r₀ (Å or nm)
    pub equilibrium_dist: f64,
}

impl MorseForceF64 {
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("morse_f64.wgsl")
    }

    /// GPU threshold: below this bond count, CPU is faster due to dispatch overhead
    const GPU_BOND_THRESHOLD: usize = 64;

    /// Compute Morse forces for all bonds.
    /// Routes to GPU for large bond counts, CPU for small.
    pub fn compute_forces(&self, positions: &[f64], bonds: &[MorseBond]) -> Result<Vec<f64>> {
        let n_particles = positions.len() / 3;
        if bonds.is_empty() {
            return Ok(vec![0.0f64; n_particles * 3]);
        }

        if bonds.len() >= Self::GPU_BOND_THRESHOLD {
            if let Ok(forces) = self.compute_gpu(positions, bonds, n_particles) {
                return Ok(forces);
            }
        }
        Ok(self.compute_cpu(positions, bonds))
    }

    /// Compute Morse forces and energies for all bonds
    pub fn compute_forces_and_energy(
        &self,
        positions: &[f64],
        bonds: &[MorseBond],
    ) -> Result<(Vec<f64>, Vec<f64>)> {
        let n_particles = positions.len() / 3;
        if bonds.is_empty() {
            return Ok((vec![0.0f64; n_particles * 3], vec![]));
        }
        Ok(self.compute_cpu_with_energy(positions, bonds))
    }

    /// GPU 2-pass: (1) per-bond forces, (2) reduce to per-particle
    fn compute_gpu(
        &self,
        positions: &[f64],
        bonds: &[MorseBond],
        n_particles: usize,
    ) -> Result<Vec<f64>> {
        let n_bonds = bonds.len();
        let dev = &self.device;

        let pos_bytes: Vec<u8> = positions.iter().flat_map(|v| v.to_le_bytes()).collect();
        let pos_buf = dev
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("morse pos"),
                contents: &pos_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            });

        let mut pair_data = Vec::with_capacity(n_bonds * 2);
        let mut de_data = Vec::with_capacity(n_bonds);
        let mut wp_data = Vec::with_capacity(n_bonds);
        let mut eq_data = Vec::with_capacity(n_bonds);
        for b in bonds {
            pair_data.push(b.i);
            pair_data.push(b.j);
            de_data.push(b.dissociation_energy);
            wp_data.push(b.width_param);
            eq_data.push(b.equilibrium_dist);
        }

        let pairs_buf = dev
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("morse pairs"),
                contents: bytemuck::cast_slice(&pair_data),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let to_f64_buf = |label: &str, data: &[f64]| -> wgpu::Buffer {
            let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
            dev.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: &bytes,
                    usage: wgpu::BufferUsages::STORAGE,
                })
        };

        let de_buf = to_f64_buf("morse de", &de_data);
        let wp_buf = to_f64_buf("morse wp", &wp_data);
        let eq_buf = to_f64_buf("morse eq", &eq_data);

        let bond_forces_buf = dev.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("morse bf"),
            size: (n_bonds * 6 * 8) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            n_bonds: u32,
            _p0: u32,
            _p1: u32,
            _p2: u32,
        }
        let params_buf = dev
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::bytes_of(&Params {
                    n_bonds: n_bonds as u32,
                    _p0: 0,
                    _p1: 0,
                    _p2: 0,
                }),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let shader = dev.compile_shader_f64(Self::wgsl_shader(), Some("Morse f64"));

        // Pass 1: per-bond forces
        let bgl = dev
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &(0..7u32)
                    .map(|i| wgpu::BindGroupLayoutEntry {
                        binding: i,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: if i == 6 {
                                wgpu::BufferBindingType::Uniform
                            } else if i == 5 {
                                wgpu::BufferBindingType::Storage { read_only: false }
                            } else {
                                wgpu::BufferBindingType::Storage { read_only: true }
                            },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    })
                    .collect::<Vec<_>>(),
            });

        let pl = dev
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });
        let pipeline = dev
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("morse_bonds_f64"),
                layout: Some(&pl),
                module: &shader,
                entry_point: "morse_bonds_f64",
                cache: None,
                compilation_options: Default::default(),
            });

        let bufs: [&wgpu::Buffer; 7] = [
            &pos_buf,
            &pairs_buf,
            &de_buf,
            &wp_buf,
            &eq_buf,
            &bond_forces_buf,
            &params_buf,
        ];
        let bg = dev.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &bufs
                .iter()
                .enumerate()
                .map(|(i, b)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: b.as_entire_binding(),
                })
                .collect::<Vec<_>>(),
        });

        let wg = (n_bonds as u32).div_ceil(WORKGROUP_SIZE_1D);
        let mut enc = dev
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut p = enc.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            p.set_pipeline(&pipeline);
            p.set_bind_group(0, &bg, &[]);
            p.dispatch_workgroups(wg, 1, 1);
        }
        dev.queue.submit(Some(enc.finish()));

        // Pass 2: reduce bond forces to per-particle
        let particle_forces_buf = dev.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("morse pf"),
            size: (n_particles * 3 * 8) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut enc2 = dev
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        enc2.clear_buffer(&particle_forces_buf, 0, None);
        dev.queue.submit(Some(enc2.finish()));

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct ReduceParams {
            n_particles: u32,
            n_bonds: u32,
            _p0: u32,
            _p1: u32,
        }
        let rp_buf = dev
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::bytes_of(&ReduceParams {
                    n_particles: n_particles as u32,
                    n_bonds: n_bonds as u32,
                    _p0: 0,
                    _p1: 0,
                }),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let r_bgl = dev
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
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
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
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
        let r_pl = dev
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&r_bgl],
                push_constant_ranges: &[],
            });
        let reduce_pipeline =
            dev.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("reduce_bond_forces_f64"),
                    layout: Some(&r_pl),
                    module: &shader,
                    entry_point: "reduce_bond_forces_f64",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let r_bg = dev.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &r_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: rp_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bond_forces_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: pairs_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: particle_forces_buf.as_entire_binding(),
                },
            ],
        });

        let wg2 = (n_particles as u32).div_ceil(WORKGROUP_SIZE_1D);
        let mut enc3 = dev
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut p = enc3.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            p.set_pipeline(&reduce_pipeline);
            p.set_bind_group(0, &r_bg, &[]);
            p.dispatch_workgroups(wg2, 1, 1);
        }
        dev.queue.submit(Some(enc3.finish()));

        dev.read_f64_buffer(&particle_forces_buf, n_particles * 3)
    }

    fn compute_cpu(&self, positions: &[f64], bonds: &[MorseBond]) -> Vec<f64> {
        let n_particles = positions.len() / 3;
        let mut forces = vec![0.0f64; n_particles * 3];

        for bond in bonds {
            let (fx, fy, fz) = self.compute_bond_force(positions, bond);

            // Add force to particle i
            forces[bond.i as usize * 3] += fx;
            forces[bond.i as usize * 3 + 1] += fy;
            forces[bond.i as usize * 3 + 2] += fz;

            // Newton's third law: opposite force on particle j
            forces[bond.j as usize * 3] -= fx;
            forces[bond.j as usize * 3 + 1] -= fy;
            forces[bond.j as usize * 3 + 2] -= fz;
        }

        forces
    }

    fn compute_cpu_with_energy(
        &self,
        positions: &[f64],
        bonds: &[MorseBond],
    ) -> (Vec<f64>, Vec<f64>) {
        let n_particles = positions.len() / 3;
        let mut forces = vec![0.0f64; n_particles * 3];
        let mut energies = Vec::with_capacity(bonds.len());

        for bond in bonds {
            let (fx, fy, fz, energy) = self.compute_bond_force_and_energy(positions, bond);

            forces[bond.i as usize * 3] += fx;
            forces[bond.i as usize * 3 + 1] += fy;
            forces[bond.i as usize * 3 + 2] += fz;

            forces[bond.j as usize * 3] -= fx;
            forces[bond.j as usize * 3 + 1] -= fy;
            forces[bond.j as usize * 3 + 2] -= fz;

            energies.push(energy);
        }

        (forces, energies)
    }

    fn compute_bond_force(&self, positions: &[f64], bond: &MorseBond) -> (f64, f64, f64) {
        let xi = positions[bond.i as usize * 3];
        let yi = positions[bond.i as usize * 3 + 1];
        let zi = positions[bond.i as usize * 3 + 2];

        let xj = positions[bond.j as usize * 3];
        let yj = positions[bond.j as usize * 3 + 1];
        let zj = positions[bond.j as usize * 3 + 2];

        let dx = xj - xi;
        let dy = yj - yi;
        let dz = zj - zi;

        let r_sq = dx * dx + dy * dy + dz * dz;
        if r_sq < 1e-20 {
            return (0.0, 0.0, 0.0);
        }

        let r = r_sq.sqrt();

        // Morse force: F = 2Da·[1 - exp(-a(r-r₀))]·exp(-a(r-r₀))·r̂
        let delta_r = r - bond.equilibrium_dist;
        let exp_term = (-bond.width_param * delta_r).exp();
        let one_minus_exp = 1.0 - exp_term;

        let force_magnitude =
            2.0 * bond.dissociation_energy * bond.width_param * one_minus_exp * exp_term;

        let force_over_r = force_magnitude / r;
        (force_over_r * dx, force_over_r * dy, force_over_r * dz)
    }

    fn compute_bond_force_and_energy(
        &self,
        positions: &[f64],
        bond: &MorseBond,
    ) -> (f64, f64, f64, f64) {
        let xi = positions[bond.i as usize * 3];
        let yi = positions[bond.i as usize * 3 + 1];
        let zi = positions[bond.i as usize * 3 + 2];

        let xj = positions[bond.j as usize * 3];
        let yj = positions[bond.j as usize * 3 + 1];
        let zj = positions[bond.j as usize * 3 + 2];

        let dx = xj - xi;
        let dy = yj - yi;
        let dz = zj - zi;

        let r_sq = dx * dx + dy * dy + dz * dz;
        if r_sq < 1e-20 {
            return (0.0, 0.0, 0.0, 0.0);
        }

        let r = r_sq.sqrt();

        let delta_r = r - bond.equilibrium_dist;
        let exp_term = (-bond.width_param * delta_r).exp();
        let one_minus_exp = 1.0 - exp_term;

        // Force
        let force_magnitude =
            2.0 * bond.dissociation_energy * bond.width_param * one_minus_exp * exp_term;
        let force_over_r = force_magnitude / r;

        // Energy: U = D·[1 - exp(-a(r-r₀))]²
        let energy = bond.dissociation_energy * one_minus_exp * one_minus_exp;

        (
            force_over_r * dx,
            force_over_r * dy,
            force_over_r * dz,
            energy,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_device() -> Option<Arc<crate::device::WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_f64_gpu_available_sync()
    }

    #[test]
    fn test_morse_equilibrium() {
        let Some(device) = get_test_device() else {
            return;
        };
        let op = MorseForceF64::new(device).unwrap();

        // Two particles at equilibrium distance
        let positions = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0]; // r = 1.0
        let bonds = vec![MorseBond {
            i: 0,
            j: 1,
            dissociation_energy: 1.0,
            width_param: 2.0,
            equilibrium_dist: 1.0, // r₀ = 1.0
        }];

        let forces = op.compute_forces(&positions, &bonds).unwrap();

        // At equilibrium, force should be zero
        assert!(
            forces[0].abs() < 1e-10,
            "Force at equilibrium should be zero"
        );
        assert!(
            forces[3].abs() < 1e-10,
            "Force at equilibrium should be zero"
        );
    }

    #[test]
    fn test_morse_stretched() {
        let Some(device) = get_test_device() else {
            return;
        };
        let op = MorseForceF64::new(device).unwrap();

        // Two particles stretched beyond equilibrium
        let positions = vec![0.0, 0.0, 0.0, 1.5, 0.0, 0.0]; // r = 1.5
        let bonds = vec![MorseBond {
            i: 0,
            j: 1,
            dissociation_energy: 1.0,
            width_param: 2.0,
            equilibrium_dist: 1.0, // r₀ = 1.0
        }];

        let forces = op.compute_forces(&positions, &bonds).unwrap();

        // Stretched bond should pull particles together
        assert!(
            forces[0] > 0.0,
            "Particle 0 should be pulled toward particle 1"
        );
        assert!(
            forces[3] < 0.0,
            "Particle 1 should be pulled toward particle 0"
        );
    }

    #[test]
    fn test_morse_compressed() {
        let Some(device) = get_test_device() else {
            return;
        };
        let op = MorseForceF64::new(device).unwrap();

        // Two particles compressed below equilibrium
        let positions = vec![0.0, 0.0, 0.0, 0.5, 0.0, 0.0]; // r = 0.5
        let bonds = vec![MorseBond {
            i: 0,
            j: 1,
            dissociation_energy: 1.0,
            width_param: 2.0,
            equilibrium_dist: 1.0, // r₀ = 1.0
        }];

        let forces = op.compute_forces(&positions, &bonds).unwrap();

        // Compressed bond should push particles apart
        assert!(
            forces[0] < 0.0,
            "Particle 0 should be pushed away from particle 1"
        );
        assert!(
            forces[3] > 0.0,
            "Particle 1 should be pushed away from particle 0"
        );
    }

    #[test]
    fn test_morse_energy_minimum() {
        let Some(device) = get_test_device() else {
            return;
        };
        let op = MorseForceF64::new(device).unwrap();

        // At equilibrium
        let positions = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let bonds = vec![MorseBond {
            i: 0,
            j: 1,
            dissociation_energy: 1.0,
            width_param: 2.0,
            equilibrium_dist: 1.0,
        }];

        let (_, energies) = op.compute_forces_and_energy(&positions, &bonds).unwrap();

        // At equilibrium, energy should be zero (minimum of Morse potential)
        assert!(
            energies[0].abs() < 1e-10,
            "Energy at equilibrium should be zero"
        );
    }
}
