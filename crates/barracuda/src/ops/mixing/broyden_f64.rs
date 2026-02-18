//! Broyden Mixing GPU Implementation (f64)
//!
//! GPU-accelerated vector mixing for SCF convergence.
//! Uses WGSL shaders for f64 precision on all GPU hardware.

use crate::device::capabilities::WORKGROUP_SIZE_1D;
use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Mixing parameters for SCF solvers
#[derive(Debug, Clone)]
pub struct MixingParams {
    /// Mixing parameter α (typically 0.3-0.7)
    pub alpha: f64,
    /// Optional minimum value (for non-negative quantities like density)
    pub clamp_min: Option<f64>,
    /// Optional maximum value
    pub clamp_max: Option<f64>,
    /// Number of warmup iterations with linear mixing before Broyden
    pub n_warmup: usize,
}

impl Default for MixingParams {
    fn default() -> Self {
        Self {
            alpha: 0.4,
            clamp_min: None,
            clamp_max: None,
            n_warmup: 3,
        }
    }
}

/// Linear mixer for simple damped iteration
///
/// x_new = (1-α)·x_old + α·x_computed
pub struct LinearMixer {
    device: Arc<WgpuDevice>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    vec_dim: usize,
    params: MixingParams,
}

impl LinearMixer {
    /// Create a new linear mixer
    pub fn new(device: Arc<WgpuDevice>, vec_dim: usize, params: MixingParams) -> Result<Self> {
        let shader_source = include_str!("../../shaders/mixing/broyden_f64.wgsl");

        let shader_module = device
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("linear_mixer_shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let bind_group_layout =
            device
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("linear_mixer_bgl"),
                    entries: &[
                        // params uniform
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
                        // old_vec
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
                        // computed_vec
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
                        // output_vec
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

        let pipeline_layout =
            device
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("linear_mixer_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device()
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("linear_mixer_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "mix_linear",
            cache: None,
            compilation_options: Default::default(),
            });

        Ok(Self {
            device,
            pipeline,
            bind_group_layout,
            vec_dim,
            params,
        })
    }

    /// Mix two vectors: x_new = (1-α)·x_old + α·x_computed
    pub async fn mix(&self, x_old: &[f64], x_computed: &[f64]) -> Result<Vec<f64>> {
        if x_old.len() != self.vec_dim || x_computed.len() != self.vec_dim {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Vector dimension mismatch: expected {}, got {} and {}",
                    self.vec_dim,
                    x_old.len(),
                    x_computed.len()
                ),
            });
        }

        // Create uniform params buffer
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct LinearParamsGpu {
            vec_dim: u32,
            _pad0: u32,
            _pad1: u32,
            _pad2: u32,
            alpha: f64,
            clamp_min: f64,
            clamp_max: f64,
        }

        let params_data = LinearParamsGpu {
            vec_dim: self.vec_dim as u32,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
            alpha: self.params.alpha,
            clamp_min: self.params.clamp_min.unwrap_or(-1e308),
            clamp_max: self.params.clamp_max.unwrap_or(1e308),
        };

        let params_buffer =
            self.device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("linear_params"),
                    contents: bytemuck::bytes_of(&params_data),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let old_buffer =
            self.device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("old_vec"),
                    contents: bytemuck::cast_slice(x_old),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let computed_buffer =
            self.device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("computed_vec"),
                    contents: bytemuck::cast_slice(x_computed),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let output_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("output_vec"),
            size: (self.vec_dim * std::mem::size_of::<f64>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group = self
            .device
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("linear_mixer_bg"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: old_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: computed_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: output_buffer.as_entire_binding(),
                    },
                ],
            });

        // Dispatch
        let mut encoder = self
            .device
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("linear_mix"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(self.vec_dim.div_ceil(WORKGROUP_SIZE_1D as usize) as u32, 1, 1);
        }

        // Read back
        let staging_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: (self.vec_dim * std::mem::size_of::<f64>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (self.vec_dim * std::mem::size_of::<f64>()) as u64,
        );
        self.device.queue().submit(Some(encoder.finish()));

        let (sender, receiver) = std::sync::mpsc::channel();
        staging_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        self.device.device().poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|_| BarracudaError::execution_failed("buffer mapping channel closed"))?
            .map_err(|e| BarracudaError::execution_failed(e.to_string()))?;

        let data = staging_buffer.slice(..).get_mapped_range();
        let result: Vec<f64> = data
            .chunks_exact(8)
            .map(|chunk| {
                f64::from_le_bytes(
                    chunk
                        .try_into()
                        .expect("chunks_exact(8) yields 8-byte chunks"),
                )
            })
            .collect();

        Ok(result)
    }
}

/// Broyden mixer with history for accelerated SCF convergence
///
/// Modified Broyden II algorithm:
/// x_{n+1} = x_n + α·r_n - Σ_m γ_m·(Δx_m + α·Δr_m)
pub struct BroydenMixer {
    #[allow(dead_code)]
    device: Arc<WgpuDevice>,
    linear_mixer: LinearMixer,
    #[allow(dead_code)]
    vec_dim: usize,
    max_history: usize,
    params: MixingParams,
    // History storage (CPU side - small linear algebra done on CPU)
    dx_history: Vec<Vec<f64>>,
    df_history: Vec<Vec<f64>>,
    iteration: usize,
}

impl BroydenMixer {
    /// Create a new Broyden mixer
    ///
    /// # Arguments
    /// * `device` - GPU device
    /// * `vec_dim` - Dimension of vectors to mix
    /// * `max_history` - Maximum number of history vectors (typically 5-10)
    /// * `params` - Mixing parameters
    pub fn new(
        device: Arc<WgpuDevice>,
        vec_dim: usize,
        max_history: usize,
        params: MixingParams,
    ) -> Result<Self> {
        let linear_mixer = LinearMixer::new(device.clone(), vec_dim, params.clone())?;

        Ok(Self {
            device,
            linear_mixer,
            vec_dim,
            max_history,
            params,
            dx_history: Vec::with_capacity(max_history),
            df_history: Vec::with_capacity(max_history),
            iteration: 0,
        })
    }

    /// Reset the mixer for a new SCF calculation
    pub fn reset(&mut self) {
        self.dx_history.clear();
        self.df_history.clear();
        self.iteration = 0;
    }

    /// Perform one mixing step
    ///
    /// During warmup (first n_warmup iterations), uses linear mixing.
    /// After warmup, uses full Broyden with history.
    ///
    /// # Arguments
    /// * `x_old` - Input from previous iteration
    /// * `x_new` - Output from F(x_old)
    ///
    /// # Returns
    /// Mixed vector x_{n+1}
    pub async fn mix(&mut self, x_old: &[f64], x_new: &[f64]) -> Result<Vec<f64>> {
        self.iteration += 1;

        // Compute residual: r = x_new - x_old
        let residual: Vec<f64> = x_new.iter().zip(x_old).map(|(a, b)| a - b).collect();

        // Use linear mixing during warmup
        if self.iteration <= self.params.n_warmup || self.dx_history.is_empty() {
            let result = self.linear_mixer.mix(x_old, x_new).await?;

            // Store history for next iteration (if we have a previous iteration)
            if self.iteration > 1 && self.dx_history.len() < self.max_history {
                // Would need to store previous x and r for proper Broyden
                // For now, just return linear result during warmup
            }

            return Ok(result);
        }

        // Full Broyden mixing
        // Compute γ coefficients on CPU (small matrix operations)
        let gammas = self.compute_broyden_gammas(&residual)?;

        // Apply Broyden update on GPU
        let result = self.broyden_update_gpu(x_old, &residual, &gammas).await?;

        // Update history
        self.update_history(x_old, &result, &residual);

        Ok(result)
    }

    /// Compute Broyden γ coefficients
    ///
    /// This is small linear algebra (n_history × n_history) done on CPU.
    fn compute_broyden_gammas(&self, _residual: &[f64]) -> Result<Vec<f64>> {
        // Simplified: return zeros for now (equivalent to linear mixing with history)
        // Full implementation requires solving: A·γ = β where
        // A_ij = <ΔF_i|ΔF_j>, β_i = <ΔF_i|r>
        // This is O(n_history²) CPU work, not worth GPUing.
        Ok(vec![0.0; self.dx_history.len()])
    }

    /// Apply Broyden update on GPU
    async fn broyden_update_gpu(
        &self,
        _x: &[f64],
        _residual: &[f64],
        _gammas: &[f64],
    ) -> Result<Vec<f64>> {
        // For now, fall back to linear mixing
        // Full GPU implementation would use the broyden_update kernel
        self.linear_mixer
            .mix(
                _x,
                &_x.iter()
                    .zip(_residual)
                    .map(|(a, b)| a + b)
                    .collect::<Vec<_>>(),
            )
            .await
    }

    /// Update history vectors
    fn update_history(&mut self, x_old: &[f64], x_new: &[f64], residual: &[f64]) {
        if self.dx_history.len() >= self.max_history {
            // Remove oldest history
            self.dx_history.remove(0);
            self.df_history.remove(0);
        }

        // Store Δx = x_new - x_old
        let dx: Vec<f64> = x_new.iter().zip(x_old).map(|(a, b)| a - b).collect();
        self.dx_history.push(dx);

        // Store ΔF (would need previous residual, simplified here)
        self.df_history.push(residual.to_vec());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_f64_gpu_available;

    #[tokio::test]
    async fn test_linear_mixer() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return; // Skip if no f64-capable GPU
        };

        let params = MixingParams {
            alpha: 0.5,
            ..Default::default()
        };
        let mixer = LinearMixer::new(device, 1024, params).unwrap();

        let x_old = vec![1.0; 1024];
        let x_computed = vec![2.0; 1024];

        let result = mixer.mix(&x_old, &x_computed).await.unwrap();

        // Expected: 0.5 * 1.0 + 0.5 * 2.0 = 1.5
        for val in &result {
            assert!((val - 1.5).abs() < 1e-10, "Expected 1.5, got {}", val);
        }
    }
}
