//! MelScale - Mel filterbank for audio feature extraction
//!
//! Converts linear frequency scale to mel scale.
//! Used in speech recognition (MFCC, mel spectrograms).
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0)
}

/// MelScale operation
pub struct MelScale {
    spectrogram: Tensor,
    n_frames: usize,
    n_freqs: usize,
    n_mels: usize,
    sample_rate: f32,
    f_min: f32,
    f_max: f32,
}

impl MelScale {
    /// Create a new mel scale operation
    pub fn new(
        spectrogram: Tensor,
        n_frames: usize,
        n_freqs: usize,
        n_mels: usize,
        sample_rate: f32,
        f_min: f32,
        f_max: f32,
    ) -> Result<Self> {
        let spec_size: usize = spectrogram.shape().iter().product();
        if spec_size != n_frames * n_freqs {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Spectrogram size ({}) must equal n_frames * n_freqs ({})",
                    spec_size,
                    n_frames * n_freqs
                ),
            });
        }

        Ok(Self {
            spectrogram,
            n_frames,
            n_freqs,
            n_mels,
            sample_rate,
            f_min,
            f_max,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/audio/mel_scale.wgsl")
    }

    /// Build mel filterbank (CPU-side preprocessing)
    fn build_filterbank(&self) -> Vec<f32> {
        let mel_min = hz_to_mel(self.f_min);
        let mel_max = hz_to_mel(self.f_max);
        let mel_points: Vec<f32> = (0..=self.n_mels + 1)
            .map(|i| mel_to_hz(mel_min + (mel_max - mel_min) * i as f32 / (self.n_mels + 1) as f32))
            .collect();

        let freq_bin = self.sample_rate / (2.0 * (self.n_freqs - 1) as f32);

        // Build filterbank
        let mut filterbank = vec![vec![0.0f32; self.n_freqs]; self.n_mels];
        for m in 0..self.n_mels {
            for f in 0..self.n_freqs {
                let freq = f as f32 * freq_bin;

                if freq >= mel_points[m] && freq <= mel_points[m + 1] {
                    filterbank[m][f] = (freq - mel_points[m]) / (mel_points[m + 1] - mel_points[m]);
                } else if freq >= mel_points[m + 1] && freq <= mel_points[m + 2] {
                    filterbank[m][f] =
                        (mel_points[m + 2] - freq) / (mel_points[m + 2] - mel_points[m + 1]);
                }
            }
        }

        // Flatten filterbank
        filterbank.into_iter().flatten().collect()
    }

    /// Execute the mel scale operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.spectrogram.device();

        // Build filterbank (CPU-side preprocessing)
        let filterbank_data = self.build_filterbank();

        // Access input buffer directly (zero-copy)
        let spectrogram_buffer = self.spectrogram.buffer();

        // Create filterbank buffer
        let filterbank_buffer =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Mel Filterbank"),
                    contents: bytemuck::cast_slice(&filterbank_data),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        // Create output buffer
        let output_size = self.n_frames * self.n_mels;
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            n_frames: u32,
            n_freqs: u32,
            n_mels: u32,
            sample_rate: f32,
            f_min: f32,
            f_max: f32,
        }

        let params = Params {
            n_frames: self.n_frames as u32,
            n_freqs: self.n_freqs as u32,
            n_mels: self.n_mels as u32,
            sample_rate: self.sample_rate,
            f_min: self.f_min,
            f_max: self.f_max,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("MelScale Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Compile shader
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("MelScale Shader"));

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("MelScale Bind Group Layout"),
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

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MelScale Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: spectrogram_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: filterbank_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("MelScale Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("MelScale Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
                });

        // Execute compute shader
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MelScale Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("MelScale Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let size = self.n_frames * self.n_mels;
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
            let workgroups = (size as u32).div_ceil(optimal_wg_size);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Output shape: [n_frames, n_mels]
        let output_shape = vec![self.n_frames, self.n_mels];

        // Return tensor without reading back (zero-copy)
        Ok(Tensor::from_buffer(
            output_buffer,
            output_shape,
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_mel_scale_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let spectrogram = Tensor::from_vec_on(vec![1.0; 100 * 257], vec![100, 257], device.clone())
            .await
            .unwrap();

        let mel_spec = MelScale::new(spectrogram, 100, 257, 80, 16000.0, 0.0, 8000.0)
            .unwrap()
            .execute()
            .unwrap();
        assert_eq!(mel_spec.shape(), &[100, 80]);
    }
}
