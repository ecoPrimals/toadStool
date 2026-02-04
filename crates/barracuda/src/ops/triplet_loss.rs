//! Triplet Loss - GPU-accelerated metric learning loss
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (new shader!)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready for metric learning)
//!
//! ## Algorithm
//!
//! ```text
//! For each triplet (anchor, positive, negative):
//!
//! d_pos = distance(anchor, positive)   // Should be small
//! d_neg = distance(anchor, negative)   // Should be large
//!
//! loss = max(0, d_pos - d_neg + margin)
//! ```
//!
//! **Goal**: Learn embeddings where similar items are close, dissimilar items are far
//!
//! **Implementation**: Single-pass GPU distance computation
//!
//! **Key Properties**:
//! - Pulls positives closer to anchors
//! - Pushes negatives farther from anchors
//! - Margin ensures minimum separation
//! - No explicit classification needed
//!
//! **Used By**: Face recognition, person re-ID, similarity search, metric learning
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let anchors = Tensor::randn(vec![32, 128]).await?;   // [batch, embedding_dim]
//! let positives = Tensor::randn(vec![32, 128]).await?; // Same class as anchors
//! let negatives = Tensor::randn(vec![32, 128]).await?; // Different class
//!
//! let loss = anchors.triplet_loss(&positives, &negatives, 0.2)?;
//! ```

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// Triplet loss parameters for WGSL shader
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TripletParams {
    batch_size: u32,
    embedding_dim: u32,
    margin: f32,
    distance_type: u32,
}

/// Distance metric for triplet loss
#[derive(Copy, Clone, Debug)]
pub enum DistanceMetric {
    /// L2 Euclidean distance (default)
    L2,
    /// Cosine distance (1 - cosine similarity)
    Cosine,
}

/// Triplet Loss operation
///
/// **Deep Debt**: Uses new WGSL shader for metric learning
pub struct TripletLoss {
    anchors: Tensor,
    positives: Tensor,
    negatives: Tensor,
    margin: f32,
    distance_metric: DistanceMetric,
}

impl TripletLoss {
    /// Create new Triplet loss operation
    ///
    /// **Deep Debt**: Validates all inputs for shape compatibility
    pub fn new(
        anchors: Tensor,
        positives: Tensor,
        negatives: Tensor,
        margin: f32,
        distance_metric: DistanceMetric,
    ) -> Result<Self> {
        // Validate shapes match
        if anchors.shape() != positives.shape() {
            return Err(BarracudaError::shape_mismatch(
                anchors.shape().to_vec(),
                positives.shape().to_vec(),
            ));
        }
        if anchors.shape() != negatives.shape() {
            return Err(BarracudaError::shape_mismatch(
                anchors.shape().to_vec(),
                negatives.shape().to_vec(),
            ));
        }

        // Validate shape is 2D [batch, embedding_dim]
        if anchors.shape().len() != 2 {
            return Err(BarracudaError::invalid_op(
                "TripletLoss",
                format!(
                    "Expected 2D tensors [batch, embedding_dim], got shape {:?}",
                    anchors.shape()
                ),
            ));
        }

        // Validate margin
        if margin < 0.0 {
            return Err(BarracudaError::invalid_op(
                "TripletLoss",
                format!("margin must be non-negative, got {}", margin),
            ));
        }

        Ok(Self {
            anchors,
            positives,
            negatives,
            margin,
            distance_metric,
        })
    }

    /// WGSL shader source
    fn shader() -> &'static str {
        include_str!("../shaders/triplet_loss.wgsl")
    }

    /// Execute Triplet loss (GPU distance computation)
    ///
    /// **Deep Debt**: Efficient single-pass distance computation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.anchors.device();

        let batch_size = self.anchors.shape()[0];
        let embedding_dim = self.anchors.shape()[1];

        // Create parameters
        let params = TripletParams {
            batch_size: batch_size as u32,
            embedding_dim: embedding_dim as u32,
            margin: self.margin,
            distance_type: match self.distance_metric {
                DistanceMetric::L2 => 0,
                DistanceMetric::Cosine => 1,
            },
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Triplet Loss Params"),
            size: std::mem::size_of::<TripletParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device
            .queue
            .write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        // Output buffer (one loss value per sample)
        let output_buffer = device.create_buffer_f32(batch_size)?;

        // Compile shader
        let shader = device.compile_shader(Self::shader(), Some("Triplet Loss"));

        // Create bind group layout
        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Triplet Loss BGL"),
                entries: &[
                    // Anchors
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
                    // Positives
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
                    // Negatives
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
                    // Output
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
                    // Params
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

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Triplet Loss BG"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.anchors.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.positives.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.negatives.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Triplet Loss Pipeline Layout"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Triplet Loss Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Triplet Loss Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Triplet Loss Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // One workgroup dispatch
            let workgroups = (batch_size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));
        device.device.poll(wgpu::Maintain::Wait);

        // Return output tensor [batch_size]
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
    /// Triplet loss for metric learning
    ///
    /// **Deep Debt**: Essential for similarity learning and face recognition
    ///
    /// # Arguments
    /// - `positives`: Similar embeddings [same shape as anchors]
    /// - `negatives`: Dissimilar embeddings [same shape as anchors]
    /// - `margin`: Minimum separation between positive and negative (typically 0.2-1.0)
    ///
    /// # Returns
    /// - Loss tensor [batch_size] (one value per triplet)
    ///
    /// # Example
    /// ```rust,ignore
    /// // L2 distance (default)
    /// let loss = anchors.triplet_loss(&positives, &negatives, 0.2)?;
    ///
    /// // Cosine distance
    /// let loss = anchors.triplet_loss_cosine(&positives, &negatives, 0.1)?;
    /// ```
    ///
    /// # Note
    /// - Embeddings should be [batch, embedding_dim]
    /// - Margin controls how far negatives should be from positives
    /// - Larger margin = stricter separation requirement
    pub fn triplet_loss(self, positives: &Self, negatives: &Self, margin: f32) -> Result<Self> {
        TripletLoss::new(
            self,
            positives.clone(),
            negatives.clone(),
            margin,
            DistanceMetric::L2,
        )?
        .execute()
    }

    /// Triplet loss with cosine distance metric
    ///
    /// **Deep Debt**: Useful when embeddings are normalized
    pub fn triplet_loss_cosine(
        self,
        positives: &Self,
        negatives: &Self,
        margin: f32,
    ) -> Result<Self> {
        TripletLoss::new(
            self,
            positives.clone(),
            negatives.clone(),
            margin,
            DistanceMetric::Cosine,
        )?
        .execute()
    }
}

// ═══════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_triplet_loss_gpu_basic() {
        let device = get_test_device().await;

        let batch = 32;
        let embedding_dim = 128;

        // Create triplets
        let anchors = Tensor::from_vec_on(
            vec![1.0; batch * embedding_dim],
            vec![batch, embedding_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let positives = Tensor::from_vec_on(
            vec![1.1; batch * embedding_dim], // Close to anchors
            vec![batch, embedding_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let negatives = Tensor::from_vec_on(
            vec![5.0; batch * embedding_dim], // Far from anchors
            vec![batch, embedding_dim],
            device,
        )
        .await
        .unwrap();

        let loss = anchors.triplet_loss(&positives, &negatives, 0.2).unwrap();

        assert_eq!(loss.shape(), &[batch]);
        let data = loss.to_vec().unwrap();

        // Loss should be low (negatives are far enough)
        assert!(data.iter().all(|&x| x >= 0.0 && x.is_finite()));
    }

    #[tokio::test]
    async fn test_triplet_loss_gpu_hard_negative() {
        let device = get_test_device().await;

        let batch = 16;
        let embedding_dim = 64;

        // All embeddings very similar (hard negative case)
        let anchors = Tensor::from_vec_on(
            vec![1.0; batch * embedding_dim],
            vec![batch, embedding_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let positives = Tensor::from_vec_on(
            vec![1.02; batch * embedding_dim], // Very close
            vec![batch, embedding_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let negatives = Tensor::from_vec_on(
            vec![1.03; batch * embedding_dim], // Also very close (hard negative!)
            vec![batch, embedding_dim],
            device,
        )
        .await
        .unwrap();

        let loss = anchors.triplet_loss(&positives, &negatives, 0.2).unwrap();
        let data = loss.to_vec().unwrap();

        // Loss should be positive (negatives not far enough from positives)
        assert!(data.iter().all(|&x| x > 0.0));
    }

    #[tokio::test]
    async fn test_triplet_loss_gpu_easy_negative() {
        let device = get_test_device().await;

        let batch = 8;
        let embedding_dim = 32;

        let anchors = Tensor::from_vec_on(
            vec![0.0; batch * embedding_dim],
            vec![batch, embedding_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let positives = Tensor::from_vec_on(
            vec![0.1; batch * embedding_dim],
            vec![batch, embedding_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let negatives = Tensor::from_vec_on(
            vec![10.0; batch * embedding_dim], // Very far (easy negative)
            vec![batch, embedding_dim],
            device,
        )
        .await
        .unwrap();

        let loss = anchors.triplet_loss(&positives, &negatives, 0.2).unwrap();
        let data = loss.to_vec().unwrap();

        // Loss should be zero or near-zero (negatives far enough)
        assert!(data.iter().all(|&x| x < 0.1));
    }

    #[tokio::test]
    async fn test_triplet_loss_gpu_cosine_distance() {
        let device = get_test_device().await;

        let batch = 16;
        let embedding_dim = 128;

        let anchors = Tensor::from_vec_on(
            vec![1.0; batch * embedding_dim],
            vec![batch, embedding_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let positives = Tensor::from_vec_on(
            vec![0.9; batch * embedding_dim],
            vec![batch, embedding_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let negatives = Tensor::from_vec_on(
            vec![-1.0; batch * embedding_dim], // Opposite direction
            vec![batch, embedding_dim],
            device,
        )
        .await
        .unwrap();

        let loss = anchors
            .triplet_loss_cosine(&positives, &negatives, 0.1)
            .unwrap();

        assert_eq!(loss.shape(), &[batch]);
        let data = loss.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_triplet_loss_gpu_margin_effect() {
        let device = get_test_device().await;

        let batch = 8;
        let embedding_dim = 32;

        let anchors = Tensor::from_vec_on(
            vec![1.0; batch * embedding_dim],
            vec![batch, embedding_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let positives = Tensor::from_vec_on(
            vec![1.2; batch * embedding_dim],
            vec![batch, embedding_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let negatives = Tensor::from_vec_on(
            vec![2.0; batch * embedding_dim],
            vec![batch, embedding_dim],
            device,
        )
        .await
        .unwrap();

        // Small margin
        let loss_small = anchors
            .clone()
            .triplet_loss(&positives, &negatives, 0.1)
            .unwrap();

        // Large margin
        let loss_large = anchors.triplet_loss(&positives, &negatives, 1.0).unwrap();

        let data_small = loss_small.to_vec().unwrap();
        let data_large = loss_large.to_vec().unwrap();

        // Larger margin should result in higher loss
        assert!(data_large[0] >= data_small[0]);
    }

    #[tokio::test]
    async fn test_triplet_loss_gpu_validation() {
        let device = get_test_device().await;

        let anchors = Tensor::from_vec_on(vec![1.0; 100], vec![10, 10], device.clone())
            .await
            .unwrap();

        let positives = Tensor::from_vec_on(vec![1.0; 50], vec![10, 5], device.clone())
            .await
            .unwrap();

        let negatives = Tensor::from_vec_on(vec![1.0; 100], vec![10, 10], device)
            .await
            .unwrap();

        // Shape mismatch should error
        assert!(anchors.triplet_loss(&positives, &negatives, 0.2).is_err());
    }
}
