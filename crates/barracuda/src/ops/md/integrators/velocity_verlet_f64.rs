//! VELOCITY-VERLET F64 — Symplectic integrator — f64 WGSL shader dispatch
//!
//! All math originates as `velocity_verlet_f64.wgsl`.
//! Three entry points: `main` (full step), `velocity_half_step`, `position_update`.
//!
//! Applications:
//! - Molecular dynamics
//! - N-body simulations
//! - Long-time energy conservation

use crate::device::WgpuDevice;
use crate::error::Result;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

const SHADER: &str = include_str!("velocity_verlet_f64.wgsl");
const WG: u32 = 256;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct VvParams {
    n_particles: u32,
    _pad0: u32,
    dt: f64,
}

/// GPU-accelerated Velocity-Verlet integrator (f64).
///
/// Algorithm:
/// 1. x(t+Δt) = x(t) + v(t)Δt + ½a(t)Δt²
/// 2. v(t+Δt) = v(t) + ½[a(t) + a(t+Δt)]Δt
pub struct VelocityVerletF64 {
    device: Arc<WgpuDevice>,
    step_pipeline: wgpu::ComputePipeline,
    half_vel_pipeline: wgpu::ComputePipeline,
    pos_update_pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
}

impl VelocityVerletF64 {
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        let module = device.compile_shader_f64(SHADER, Some("velocity_verlet_f64"));

        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("VV:bgl"),
                entries: &[
                    storage_ro(0), // positions
                    storage_ro(1), // velocities
                    storage_ro(2), // forces_old
                    storage_ro(3), // forces_new
                    storage_ro(4), // masses
                    storage_rw(5), // positions_new
                    storage_rw(6), // velocities_new
                    uniform(7),    // params
                ],
            });

        let layout = device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VV:layout"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let make_pipe = |entry: &str, label: &str| {
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(label),
                    layout: Some(&layout),
                    module: &module,
                    entry_point: entry,
                    compilation_options: Default::default(),
                    cache: None,
                })
        };

        let step_pipeline = make_pipe("main", "VV:step");
        let half_vel_pipeline = make_pipe("velocity_half_step", "VV:half_vel");
        let pos_update_pipeline = make_pipe("position_update", "VV:pos_update");

        Ok(Self {
            device,
            step_pipeline,
            half_vel_pipeline,
            pos_update_pipeline,
            bgl,
        })
    }

    /// Full Velocity-Verlet step on GPU.
    pub fn step(
        &self,
        positions: &[f64],
        velocities: &[f64],
        forces_old: &[f64],
        forces_new: &[f64],
        masses: &[f64],
        dt: f64,
    ) -> Result<(Vec<f64>, Vec<f64>)> {
        let n = positions.len() / 3;
        let n3 = n * 3;

        let d = &self.device.device;
        let q = &self.device.queue;

        let buf = |label, data: &[f64]| {
            d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };

        let pos_buf = buf("VV:pos", positions);
        let vel_buf = buf("VV:vel", velocities);
        let fo_buf = buf("VV:fo", forces_old);
        let fn_buf = buf("VV:fn", forces_new);
        let mass_buf = buf("VV:m", masses);

        let out_size = (n3 * 8) as u64;
        let make_out = |label| {
            d.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: out_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        };
        let pos_out = make_out("VV:pos_out");
        let vel_out = make_out("VV:vel_out");

        let params = VvParams {
            n_particles: n as u32,
            _pad0: 0,
            dt,
        };
        let params_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VV:params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bg = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VV:bg"),
            layout: &self.bgl,
            entries: &[
                entry(0, &pos_buf),
                entry(1, &vel_buf),
                entry(2, &fo_buf),
                entry(3, &fn_buf),
                entry(4, &mass_buf),
                entry(5, &pos_out),
                entry(6, &vel_out),
                entry(7, &params_buf),
            ],
        });

        let wg_count = (n as u32).div_ceil(WG);
        let mut enc = d.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.step_pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(wg_count, 1, 1);
        }

        let rb_pos = readback_buf(d, out_size);
        let rb_vel = readback_buf(d, out_size);
        enc.copy_buffer_to_buffer(&pos_out, 0, &rb_pos, 0, out_size);
        enc.copy_buffer_to_buffer(&vel_out, 0, &rb_vel, 0, out_size);
        q.submit(Some(enc.finish()));

        let new_pos = map_read_f64(d, &rb_pos)?;
        let new_vel = map_read_f64(d, &rb_vel)?;

        Ok((new_pos, new_vel))
    }

    /// Half-step velocity update on GPU (first half of leapfrog).
    pub fn velocity_half_step(
        &self,
        velocities: &[f64],
        forces: &[f64],
        masses: &[f64],
        dt: f64,
    ) -> Result<Vec<f64>> {
        let n = velocities.len() / 3;
        let n3 = n * 3;
        let d = &self.device.device;
        let q = &self.device.queue;

        let vel_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VV:hv_vel"),
            contents: bytemuck::cast_slice(velocities),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let forces_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VV:hv_f"),
            contents: bytemuck::cast_slice(forces),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let mass_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VV:hv_m"),
            contents: bytemuck::cast_slice(masses),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let out_size = (n3 * 8) as u64;
        let dummy = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VV:hv_dummy"),
            size: out_size.max(8),
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let vel_out = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VV:hv_out"),
            size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params = VvParams {
            n_particles: n as u32,
            _pad0: 0,
            dt,
        };
        let params_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VV:hv_p"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bg = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VV:hv_bg"),
            layout: &self.bgl,
            entries: &[
                entry(0, &dummy),      // positions (unused by half_step)
                entry(1, &vel_buf),    // velocities
                entry(2, &forces_buf), // forces_old (used for forces)
                entry(3, &dummy),      // forces_new (unused)
                entry(4, &mass_buf),   // masses
                entry(5, &dummy),      // positions_new (unused)
                entry(6, &vel_out),    // velocities_new (output)
                entry(7, &params_buf),
            ],
        });

        let mut enc = d.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.half_vel_pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups((n as u32).div_ceil(WG), 1, 1);
        }
        let rb = readback_buf(d, out_size);
        enc.copy_buffer_to_buffer(&vel_out, 0, &rb, 0, out_size);
        q.submit(Some(enc.finish()));

        map_read_f64(d, &rb)
    }

    /// Position update on GPU using velocities.
    pub fn position_update(
        &self,
        positions: &[f64],
        velocities: &[f64],
        dt: f64,
    ) -> Result<Vec<f64>> {
        let n = positions.len() / 3;
        let n3 = n * 3;
        let d = &self.device.device;
        let q = &self.device.queue;

        let pos_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VV:pu_pos"),
            contents: bytemuck::cast_slice(positions),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let vel_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VV:pu_vel"),
            contents: bytemuck::cast_slice(velocities),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let out_size = (n3 * 8) as u64;
        let dummy = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VV:pu_dummy"),
            size: out_size.max(8),
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let pos_out = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VV:pu_out"),
            size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params = VvParams {
            n_particles: n as u32,
            _pad0: 0,
            dt,
        };
        let params_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VV:pu_p"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let mass_dummy = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VV:pu_md"),
            size: 8,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let bg = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VV:pu_bg"),
            layout: &self.bgl,
            entries: &[
                entry(0, &pos_buf),    // positions
                entry(1, &vel_buf),    // velocities (half-step)
                entry(2, &dummy),      // forces_old (unused)
                entry(3, &dummy),      // forces_new (unused)
                entry(4, &mass_dummy), // masses (unused)
                entry(5, &pos_out),    // positions_new (output)
                entry(6, &dummy),      // velocities_new (unused)
                entry(7, &params_buf),
            ],
        });

        let mut enc = d.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pos_update_pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups((n as u32).div_ceil(WG), 1, 1);
        }
        let rb = readback_buf(d, out_size);
        enc.copy_buffer_to_buffer(&pos_out, 0, &rb, 0, out_size);
        q.submit(Some(enc.finish()));

        map_read_f64(d, &rb)
    }
}

fn entry(binding: u32, buf: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buf.as_entire_binding(),
    }
}

fn storage_ro(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn storage_rw(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn readback_buf(d: &wgpu::Device, size: u64) -> wgpu::Buffer {
    d.create_buffer(&wgpu::BufferDescriptor {
        label: Some("VV:readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn map_read_f64(d: &wgpu::Device, buf: &wgpu::Buffer) -> Result<Vec<f64>> {
    let slice = buf.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).ok();
    });
    d.poll(wgpu::Maintain::Wait);
    rx.recv()
        .map_err(|_| crate::error::BarracudaError::Gpu("VV readback channel".into()))?
        .map_err(|e| crate::error::BarracudaError::Gpu(format!("VV map: {e}")))?;
    let data = slice.get_mapped_range();
    let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    buf.unmap();
    Ok(result)
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
