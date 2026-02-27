//! 3D Fast Fourier Transform — Batched GPU Dispatch (f64 Precision)
//!
//! Performs 3D FFT via dimension-wise decomposition with a single batched
//! shader per axis. Instead of dispatching N² individual 1D FFTs per axis
//! (each with its own pipeline creation, buffer upload, and synchronous
//! readback), this implementation:
//!
//! 1. Uploads the full 3D complex array to the GPU **once**
//! 2. Pre-compiles the shader and pipelines **once** in `new()`
//! 3. For each axis (Z → Y → X): dispatches `1 + log₂(N)` compute passes
//!    that process **all** pencils simultaneously via strided addressing
//! 4. Reads back the result **once**
//!
//! For an 8×8×8 mesh this is **~12 dispatches in one command buffer**
//! vs the previous **192 individual GPU round-trips**.

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;

const WORKGROUP_SIZE: u32 = 256;

/// 3D Complex FFT operation (f64 precision) — batched GPU dispatch
///
/// Shader compilation and pipeline creation happen once at construction.
/// Each `forward`/`inverse` call submits a single command buffer containing
/// all axis passes, with one GPU readback at the end.
pub struct Fft3DF64 {
    device: Arc<WgpuDevice>,
    nx: usize,
    ny: usize,
    nz: usize,
    pipeline_butterfly: wgpu::ComputePipeline,
    pipeline_bit_reverse: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    twiddles: HashMap<u32, (wgpu::Buffer, wgpu::Buffer)>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BatchedFftParams {
    degree: u32,
    stage: u32,
    inverse: u32,
    element_stride: u32,
    dim1_stride: u32,
    dim2_stride: u32,
    dim1_count: u32,
    dim2_count: u32,
}

struct AxisConfig {
    degree: usize,
    element_stride: usize,
    dim1_stride: usize,
    dim2_stride: usize,
    dim1_count: usize,
    dim2_count: usize,
}

impl Fft3DF64 {
    /// Create a new 3D FFT operation.
    ///
    /// Pre-compiles the batched shader, creates pipelines, and precomputes
    /// twiddle factor GPU buffers for each unique axis length.
    pub fn new(device: Arc<WgpuDevice>, nx: usize, ny: usize, nz: usize) -> Result<Self> {
        if !nx.is_power_of_two() || !ny.is_power_of_two() || !nz.is_power_of_two() {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "FFT 3D dimensions must be powers of 2, got ({}, {}, {})",
                    nx, ny, nz
                ),
            });
        }

        let shader_src = include_str!("fft_3d_batched_f64.wgsl");
        let shader = device.compile_shader_f64(shader_src, Some("FFT 3D Batched f64"));

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("FFT 3D Batched BGL"),
                    entries: &[
                        bgl_entry(0, true),
                        bgl_entry(1, false),
                        bgl_entry(2, true),
                        bgl_entry(3, true),
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("FFT 3D Batched PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline_butterfly =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("FFT 3D Butterfly"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "main",
                    cache: device.pipeline_cache(),
                    compilation_options: Default::default(),
                });

        let pipeline_bit_reverse =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("FFT 3D Bit Reverse"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "bit_reverse",
                    cache: device.pipeline_cache(),
                    compilation_options: Default::default(),
                });

        let mut twiddles = HashMap::new();
        for &n in &[nx, ny, nz] {
            let n32 = n as u32;
            twiddles.entry(n32).or_insert_with(|| {
                let pi = std::f64::consts::PI;
                let mut re = Vec::with_capacity(n);
                let mut im = Vec::with_capacity(n);
                for k in 0..n {
                    let angle = -2.0 * pi * (k as f64) / (n as f64);
                    re.push(angle.cos());
                    im.push(angle.sin());
                }
                let re_buf = device
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("FFT3D Twiddle Re"),
                        contents: bytemuck::cast_slice(&re),
                        usage: wgpu::BufferUsages::STORAGE,
                    });
                let im_buf = device
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("FFT3D Twiddle Im"),
                        contents: bytemuck::cast_slice(&im),
                        usage: wgpu::BufferUsages::STORAGE,
                    });
                (re_buf, im_buf)
            });
        }

        Ok(Self {
            device,
            nx,
            ny,
            nz,
            pipeline_butterfly,
            pipeline_bit_reverse,
            bind_group_layout,
            twiddles,
        })
    }

    pub async fn forward(&self, data: &[f64]) -> Result<Vec<f64>> {
        self.execute_internal(data, false).await
    }

    pub async fn inverse(&self, data: &[f64]) -> Result<Vec<f64>> {
        self.execute_internal(data, true).await
    }

    async fn execute_internal(&self, data: &[f64], inverse: bool) -> Result<Vec<f64>> {
        let size = self.nx * self.ny * self.nz;
        let expected_len = size * 2;

        if data.len() != expected_len {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "FFT 3D data length {} doesn't match expected {} ({}x{}x{}x2)",
                    data.len(),
                    expected_len,
                    self.nx,
                    self.ny,
                    self.nz
                ),
            });
        }

        let buffer_bytes = (expected_len * std::mem::size_of::<f64>()) as u64;
        let dev = &self.device;

        let buf_a = dev
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("FFT3D buf_a"),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            });

        let buf_b = dev.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FFT3D buf_b"),
            size: buffer_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = dev
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("FFT3D Encoder"),
            });

        let axes = [
            AxisConfig {
                degree: self.nz,
                element_stride: 1,
                dim1_stride: self.ny * self.nz,
                dim2_stride: self.nz,
                dim1_count: self.nx,
                dim2_count: self.ny,
            },
            AxisConfig {
                degree: self.ny,
                element_stride: self.nz,
                dim1_stride: self.ny * self.nz,
                dim2_stride: 1,
                dim1_count: self.nx,
                dim2_count: self.nz,
            },
            AxisConfig {
                degree: self.nx,
                element_stride: self.ny * self.nz,
                dim1_stride: self.nz,
                dim2_stride: 1,
                dim1_count: self.ny,
                dim2_count: self.nz,
            },
        ];

        for axis in &axes {
            self.encode_axis(&mut encoder, &buf_a, &buf_b, buffer_bytes, axis, inverse);
        }

        dev.submit_and_poll(std::iter::once(encoder.finish()));

        dev.read_f64_buffer(&buf_a, expected_len)
    }

    fn encode_axis(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        buf_a: &wgpu::Buffer,
        buf_b: &wgpu::Buffer,
        buffer_bytes: u64,
        axis: &AxisConfig,
        inverse: bool,
    ) {
        let dev = &self.device;
        let n = axis.degree as u32;
        let log_n = (n as f32).log2() as u32;
        let num_pencils = (axis.dim1_count * axis.dim2_count) as u32;
        let inv_flag = u32::from(inverse);

        let (tw_re, tw_im) = &self.twiddles[&n];

        // Bit-reverse pass: buf_a → buf_b
        let br_params = BatchedFftParams {
            degree: n,
            stage: 0,
            inverse: inv_flag,
            element_stride: axis.element_stride as u32,
            dim1_stride: axis.dim1_stride as u32,
            dim2_stride: axis.dim2_stride as u32,
            dim1_count: axis.dim1_count as u32,
            dim2_count: axis.dim2_count as u32,
        };
        let br_params_buf = dev
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("FFT3D BR Params"),
                contents: bytemuck::bytes_of(&br_params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let br_bg = dev.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FFT3D BR BG"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buf_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: tw_re.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: tw_im.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: br_params_buf.as_entire_binding(),
                },
            ],
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("FFT3D Bit Reverse"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline_bit_reverse);
            pass.set_bind_group(0, &br_bg, &[]);
            let total_invocations = num_pencils * n;
            pass.dispatch_workgroups(total_invocations.div_ceil(WORKGROUP_SIZE), 1, 1);
        }

        // Copy buf_b → buf_a (bit-reversed data into working buffer)
        encoder.copy_buffer_to_buffer(buf_b, 0, buf_a, 0, buffer_bytes);

        // Butterfly stages
        for stage in 0..log_n {
            let s_params = BatchedFftParams {
                degree: n,
                stage,
                inverse: inv_flag,
                element_stride: axis.element_stride as u32,
                dim1_stride: axis.dim1_stride as u32,
                dim2_stride: axis.dim2_stride as u32,
                dim1_count: axis.dim1_count as u32,
                dim2_count: axis.dim2_count as u32,
            };
            let s_params_buf = dev
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("FFT3D Stage Params"),
                    contents: bytemuck::bytes_of(&s_params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

            let s_bg = dev.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("FFT3D Stage BG"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buf_a.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buf_b.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: tw_re.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: tw_im.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: s_params_buf.as_entire_binding(),
                    },
                ],
            });

            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("FFT3D Butterfly"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipeline_butterfly);
                pass.set_bind_group(0, &s_bg, &[]);
                let total = num_pencils * (n / 2);
                pass.dispatch_workgroups(total.div_ceil(WORKGROUP_SIZE), 1, 1);
            }

            // Ping-pong: copy output to working for next stage
            if stage < log_n - 1 {
                encoder.copy_buffer_to_buffer(buf_b, 0, buf_a, 0, buffer_bytes);
            }
        }

        // After last butterfly, result is in buf_b. Copy back to buf_a for next axis.
        encoder.copy_buffer_to_buffer(buf_b, 0, buf_a, 0, buffer_bytes);
    }

    pub fn dims(&self) -> (usize, usize, usize) {
        (self.nx, self.ny, self.nz)
    }
}

fn bgl_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fft_3d_f64_roundtrip() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        let n = 4;
        let size = n * n * n;

        let mut data = vec![0.0f64; size * 2];
        data[0] = 1.0;

        let fft = Fft3DF64::new(device.clone(), n, n, n).unwrap();

        let freq = fft.forward(&data).await.unwrap();
        assert_eq!(freq.len(), size * 2);

        for i in 0..size {
            let re = freq[i * 2];
            let im = freq[i * 2 + 1];
            let mag = (re * re + im * im).sqrt();
            assert!(
                (mag - 1.0).abs() < 1e-10,
                "Expected magnitude 1.0, got {}",
                mag
            );
        }

        let back = fft.inverse(&freq).await.unwrap();

        let norm = (size as f64).recip();
        let back_norm: Vec<f64> = back.iter().map(|x| x * norm).collect();

        assert!((back_norm[0] - 1.0).abs() < 1e-10);
        for i in 1..size {
            assert!((back_norm[i * 2]).abs() < 1e-10);
            assert!((back_norm[i * 2 + 1]).abs() < 1e-10);
        }
    }

    #[tokio::test]
    async fn test_fft_3d_f64_sinusoidal_roundtrip() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        let n = 8;
        let size = n * n * n;
        let pi = std::f64::consts::PI;

        let mut data = vec![0.0f64; size * 2];
        for ix in 0..n {
            for iy in 0..n {
                for iz in 0..n {
                    let idx = (ix * n * n + iy * n + iz) * 2;
                    data[idx] = (2.0 * pi * ix as f64 / n as f64).sin()
                        + 0.5 * (2.0 * pi * iz as f64 / n as f64).cos();
                }
            }
        }
        let original = data.clone();

        let fft = Fft3DF64::new(device.clone(), n, n, n).unwrap();
        let freq = fft.forward(&data).await.unwrap();
        let back = fft.inverse(&freq).await.unwrap();

        let norm = (size as f64).recip();
        for i in 0..size {
            let re = back[i * 2] * norm;
            let im = back[i * 2 + 1] * norm;
            assert!(
                (re - original[i * 2]).abs() < 1e-8,
                "Voxel {} real: expected {:.15e}, got {re:.15e}",
                i,
                original[i * 2]
            );
            assert!(
                im.abs() < 1e-8,
                "Voxel {} imag: expected ~0, got {im:.15e}",
                i
            );
        }
    }

    #[tokio::test]
    async fn test_fft_3d_f64_non_cubic() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        let (nx, ny, nz) = (4, 8, 16);
        let size = nx * ny * nz;
        let mut data = vec![0.0f64; size * 2];
        data[0] = 1.0;

        let fft = Fft3DF64::new(device.clone(), nx, ny, nz).unwrap();
        let freq = fft.forward(&data).await.unwrap();
        let back = fft.inverse(&freq).await.unwrap();

        let norm = (size as f64).recip();
        assert!((back[0] * norm - 1.0).abs() < 1e-10);
        for i in 1..size {
            assert!((back[i * 2] * norm).abs() < 1e-10);
        }
    }
}
