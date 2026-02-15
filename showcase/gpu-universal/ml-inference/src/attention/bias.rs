//! Attention Bias

use anyhow::Result;
use std::sync::Arc;

/// Attention Bias
///
/// Adds learned or positional biases to attention scores.
/// Supports various bias types:
/// - Positional bias (learned position embeddings)
/// - ALiBi (Attention with Linear Biases)
/// - Relative position bias
/// - Custom bias patterns
///
/// ## Usage
///
/// ```text
/// scores_biased = scores + bias[i, j]
/// attention_weights = softmax(scores_biased)
/// ```
pub struct AttentionBias {
    #[allow(dead_code)]
    device: Arc<wgpu::Device>,
    #[allow(dead_code)]
    queue: Arc<wgpu::Queue>,
    _pipeline: wgpu::ComputePipeline,
    _bind_group_layout: wgpu::BindGroupLayout,
}

impl AttentionBias {
    /// Create new Attention Bias operation
    pub async fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Attention Bias Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../attention_bias.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Attention Bias Bind Group Layout"),
            entries: &[
                // Scores buffer (input)
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
                // Bias buffer (input)
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
                // Output buffer
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
                // Config buffer
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Attention Bias Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Attention Bias Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "attention_bias",
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self {
            device,
            queue,
            _pipeline: pipeline,
            _bind_group_layout: bind_group_layout,
        })
    }

    /// Apply attention bias
    ///
    /// # Arguments
    ///
    /// * `scores` - Attention scores [batch, seq_len, seq_len]
    /// * `bias` - Bias values [seq_len, seq_len] or [batch, seq_len, seq_len]
    /// * `batch` - Batch size
    /// * `seq_len` - Sequence length
    ///
    /// # Returns
    ///
    /// Biased scores [batch, seq_len, seq_len]
    pub async fn apply(
        &self,
        scores: &[f32],
        bias: &[f32],
        batch: u32,
        seq_len: u32,
    ) -> Result<Vec<f32>> {
        let expected_size = (batch * seq_len * seq_len) as usize;
        anyhow::ensure!(
            scores.len() == expected_size,
            "Scores size mismatch: expected {}, got {}",
            expected_size,
            scores.len()
        );

        // Simple CPU implementation for now
        let mut output = scores.to_vec();

        for b in 0..batch {
            for i in 0..seq_len {
                for j in 0..seq_len {
                    let idx = ((b * seq_len + i) * seq_len + j) as usize;
                    let bias_idx = if bias.len() == (seq_len * seq_len) as usize {
                        // Shared bias across batch
                        (i * seq_len + j) as usize
                    } else {
                        // Per-batch bias
                        idx
                    };
                    output[idx] += bias[bias_idx];
                }
            }
        }

        Ok(output)
    }

    /// Generate ALiBi (Attention with Linear Biases)
    ///
    /// ALiBi adds a linear bias based on distance: bias`i,j` = -slope * |i - j|
    ///
    /// Reference: "Train Short, Test Long: Attention with Linear Biases"
    /// (Press et al., 2021)
    pub fn generate_alibi(num_heads: u32, seq_len: u32) -> Vec<f32> {
        let mut biases = vec![0.0f32; (num_heads * seq_len * seq_len) as usize];

        for h in 0..num_heads {
            // Compute slope for this head
            let slope = 2.0f32.powf(-((h + 1) as f32 / num_heads as f32 * 8.0));

            for i in 0..seq_len {
                for j in 0..seq_len {
                    let distance = j.abs_diff(i);
                    let bias = -slope * distance as f32;
                    let idx = ((h * seq_len + i) * seq_len + j) as usize;
                    biases[idx] = bias;
                }
            }
        }

        biases
    }
}

#[cfg(test)]
mod causal_and_bias_tests {
    use super::*;
    use crate::attention::CausalMask;

    #[test]
    fn test_causal_mask_generation() {
        let batch = 1;
        let seq_len = 4;
        let mask = CausalMask::generate(batch, seq_len);

        assert_eq!(mask.len(), (batch * seq_len * seq_len) as usize);

        // Check causal pattern
        for i in 0..seq_len {
            for j in 0..seq_len {
                let idx = (i * seq_len + j) as usize;
                if j <= i {
                    assert_eq!(mask[idx], 1.0, "Position ({}, {}) should be 1.0", i, j);
                } else {
                    assert_eq!(mask[idx], 0.0, "Position ({}, {}) should be 0.0", i, j);
                }
            }
        }
    }

    #[test]
    fn test_alibi_generation() {
        let num_heads = 8;
        let seq_len = 16;
        let biases = AttentionBias::generate_alibi(num_heads, seq_len);

        assert_eq!(biases.len(), (num_heads * seq_len * seq_len) as usize);

        // Check that biases are negative and distance-based
        for h in 0..num_heads {
            for i in 0..seq_len {
                // Bias at (i, i) should be 0 (same position)
                let idx_self = ((h * seq_len + i) * seq_len + i) as usize;
                assert_eq!(biases[idx_self], 0.0);

                // Bias should decrease with distance
                if i + 1 < seq_len {
                    let idx_next = ((h * seq_len + i) * seq_len + (i + 1)) as usize;
                    assert!(biases[idx_next] < 0.0, "ALiBi bias should be negative");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_attention_bias_creation() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        if let Some(adapter) = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
        {
            if let Ok((device, queue)) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("Test Device"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                        memory_hints: Default::default(),
                    },
                    None,
                )
                .await
            {
                let result = AttentionBias::new(Arc::new(device), Arc::new(queue)).await;
                assert!(result.is_ok(), "Failed to create attention bias operation");
            }
        }
    }
}
