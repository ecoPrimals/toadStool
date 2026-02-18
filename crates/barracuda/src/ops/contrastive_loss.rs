//! Contrastive Loss - GPU-accelerated NT-Xent for self-supervised learning
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (new shader!)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready for SimCLR, MoCo)
//!
//! ## Algorithm
//!
//! ```text
//! For batch of positive pairs (z_i, z_i+batch):
//! 1. Compute cosine similarity matrix
//! 2. For each sample i:
//!    - Numerator: exp(sim(i, positive_i) / temperature)
//!    - Denominator: sum(exp(sim(i, j) / temperature)) for all j != i
//! 3. Loss = -log(numerator / denominator)
//! ```
//!
//! **Parameters**:
//! - `temperature`: Controls distribution sharpness (typically 0.1-0.5)
//!
//! **Key Properties**:
//! - Pulls positive pairs together
//! - Pushes negative pairs apart
//! - Self-supervised (no labels needed)
//! - Standard in SimCLR, MoCo
//!
//! **Used By**: SimCLR, MoCo, CLIP, self-supervised learning
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! // Embeddings: [batch*2, embed_dim] - positive pairs concatenated
//! let embeddings = Tensor::randn(vec![16, 128]).await?;  // 8 pairs, 128-dim
//!
//! let loss = embeddings.contrastive_loss(0.5)?;  // temperature=0.5
//! ```

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ContrastiveLossParams {
    batch_size: u32,
    embed_dim: u32,
    temperature: f32,
    _padding: u32,
}

pub struct ContrastiveLoss {
    embeddings: Tensor,
    temperature: f32,
}

impl ContrastiveLoss {
    pub fn new(embeddings: Tensor, temperature: f32) -> Result<Self> {
        // Validate 2D input
        if embeddings.shape().len() != 2 {
            return Err(BarracudaError::invalid_op(
                "ContrastiveLoss",
                format!(
                    "embeddings must be 2D [batch*2, embed_dim], got shape {:?}",
                    embeddings.shape()
                ),
            ));
        }

        let batch_total = embeddings.shape()[0];
        if !batch_total.is_multiple_of(2) {
            return Err(BarracudaError::invalid_op(
                "ContrastiveLoss",
                format!(
                    "first dimension must be even (batch*2), got {}",
                    batch_total
                ),
            ));
        }

        // Validate temperature
        if temperature <= 0.0 {
            return Err(BarracudaError::invalid_op(
                "ContrastiveLoss",
                format!("temperature must be positive, got {}", temperature),
            ));
        }

        Ok(Self {
            embeddings,
            temperature,
        })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/loss/contrastive_loss.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.embeddings.device();
        let batch_total = self.embeddings.shape()[0];
        let embed_dim = self.embeddings.shape()[1];
        let batch_size = batch_total / 2;

        let params = ContrastiveLossParams {
            batch_size: batch_size as u32,
            embed_dim: embed_dim as u32,
            temperature: self.temperature,
            _padding: 0,
        };

        // Output: [batch_size] losses
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("contrastive_loss_output"),
            size: (batch_size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("contrastive_loss_params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("contrastive_loss_shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("contrastive_loss_bind_group_layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
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
                    label: Some("contrastive_loss_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("contrastive_loss_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("contrastive_loss_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.embeddings.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("contrastive_loss_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("contrastive_loss_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device.as_ref());
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Reduction);
            let workgroups = (batch_size as u32).div_ceil(optimal_wg_size);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch_size],
            device.clone(),
        ))
    }
}

// ═══════════════════════════════════════════════════════════════
// TENSOR API INTEGRATION
// ═══════════════════════════════════════════════════════════════

impl Tensor {
    /// Contrastive loss (NT-Xent) for self-supervised learning
    ///
    /// **Deep Debt**: Essential for SimCLR, MoCo, CLIP-style training
    ///
    /// # Arguments
    /// - `self`: Embeddings [batch*2, embed_dim] - positive pairs concatenated
    /// - `temperature`: Controls distribution sharpness (typically 0.1-0.5)
    ///
    /// # Returns
    /// - Loss tensor [batch_size] - per-sample losses
    ///
    /// # Example
    /// ```rust,ignore
    /// // 8 positive pairs, 128-dimensional embeddings
    /// let embeddings = Tensor::randn(vec![16, 128]).await?;
    ///
    /// // SimCLR-style: temperature=0.5
    /// let loss = embeddings.contrastive_loss(0.5)?;
    ///
    /// // MoCo-style: temperature=0.07
    /// let loss = embeddings.contrastive_loss(0.07)?;
    /// ```
    ///
    /// # Note
    /// - Input format: First `batch_size` samples paired with second `batch_size` samples
    /// - Lower temperature: Sharper distribution (more aggressive)
    /// - Higher temperature: Smoother distribution (more permissive)
    /// - Standard values: SimCLR=0.5, MoCo=0.07
    pub fn contrastive_loss(self, temperature: f32) -> Result<Self> {
        ContrastiveLoss::new(self, temperature)?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_contrastive_loss_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // 4 positive pairs (8 samples), 16-dim embeddings
        let data: Vec<f32> = (0..8 * 16).map(|i| ((i % 100) as f32) / 100.0).collect();
        let embeddings = Tensor::from_vec_on(data, vec![8, 16], device.clone())
            .await
            .unwrap();

        let loss = embeddings.contrastive_loss(0.5).unwrap();

        assert_eq!(loss.shape(), &[4]); // batch_size=4

        let data = loss.to_vec().unwrap();
        assert!(data.iter().all(|&x: &f32| x.is_finite()));
        assert!(data.iter().all(|&x: &f32| x >= 0.0)); // Loss should be non-negative
    }

    #[tokio::test]
    async fn test_contrastive_loss_similar_pairs() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Create similar positive pairs (should have relatively low loss)
        let data: Vec<f32> = (0..8 * 16)
            .map(|i| {
                let row = i / 16;
                let col = i % 16;
                // First 4 rows similar to last 4 rows
                ((row % 4) * 16 + col) as f32 / 64.0
            })
            .collect();

        let embeddings = Tensor::from_vec_on(data, vec![8, 16], device.clone())
            .await
            .unwrap();

        let loss = embeddings.contrastive_loss(0.5).unwrap();

        let data = loss.to_vec().unwrap();
        assert!(data.iter().all(|&x: &f32| x.is_finite()));
    }

    #[tokio::test]
    async fn test_contrastive_loss_temperature_effect() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let data: Vec<f32> = (0..6 * 32).map(|i| ((i % 100) as f32) / 100.0).collect();
        let embeddings = Tensor::from_vec_on(data, vec![6, 32], device.clone())
            .await
            .unwrap();

        // Lower temperature should sharpen distribution
        let loss_low_temp = embeddings.clone().contrastive_loss(0.1).unwrap();
        let loss_high_temp = embeddings.contrastive_loss(1.0).unwrap();

        let data_low = loss_low_temp.to_vec().unwrap();
        let data_high = loss_high_temp.to_vec().unwrap();

        assert!(data_low.iter().all(|&x: &f32| x.is_finite()));
        assert!(data_high.iter().all(|&x: &f32| x.is_finite()));
    }

    #[tokio::test]
    async fn test_contrastive_loss_validation() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Test odd batch size (should fail)
        let embeddings = Tensor::from_vec_on(vec![0.5; 7 * 16], vec![7, 16], device.clone())
            .await
            .unwrap();
        assert!(embeddings.contrastive_loss(0.5).is_err());

        // Test negative temperature (should fail)
        let embeddings = Tensor::from_vec_on(vec![0.5; 8 * 16], vec![8, 16], device.clone())
            .await
            .unwrap();
        assert!(embeddings.clone().contrastive_loss(-0.5).is_err());

        // Test zero temperature (should fail)
        assert!(embeddings.contrastive_loss(0.0).is_err());

        // Test 1D input (should fail)
        let embeddings = Tensor::from_vec_on(vec![0.5; 16], vec![16], device.clone())
            .await
            .unwrap();
        assert!(embeddings.contrastive_loss(0.5).is_err());
    }

    #[tokio::test]
    async fn test_contrastive_loss_large_batch() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Large batch: 32 pairs (64 samples), 128-dim
        let data: Vec<f32> = (0..64 * 128).map(|i| ((i % 100) as f32) / 100.0).collect();
        let embeddings = Tensor::from_vec_on(data, vec![64, 128], device.clone())
            .await
            .unwrap();

        let loss = embeddings.contrastive_loss(0.07).unwrap(); // MoCo-style temperature

        assert_eq!(loss.shape(), &[32]);

        let data = loss.to_vec().unwrap();
        assert!(data.iter().all(|&x: &f32| x.is_finite()));
        assert!(data.iter().all(|&x: &f32| x >= 0.0));
    }
}
