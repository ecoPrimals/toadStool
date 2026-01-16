//! Scaled Dot-Product Attention

use anyhow::{Context, Result};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Scaled Dot-Product Attention
///
/// The fundamental attention mechanism: `Attention(Q, K, V) = softmax(Q·K^T / √d_k)·V`
///
/// ## Parameters
///
/// - `Q` (Query): [batch, seq_len, d_k]
/// - `K` (Key): [batch, seq_len, d_k]
/// - `V` (Value): [batch, seq_len, d_v]
/// - `mask` (optional): [batch, seq_len, seq_len] - attention mask
///
/// ## Returns
///
/// - Output: [batch, seq_len, d_v]
/// - Attention weights: [batch, seq_len, seq_len]
///
/// ## Performance
///
/// - Complexity: O(seq_len²·d_k)
/// - Memory: O(seq_len²) for attention matrix
/// - Optimized with tiling for long sequences
pub struct ScaledDotProductAttention {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl ScaledDotProductAttention {
    /// Create new Scaled Dot-Product Attention operation
    ///
    /// # Errors
    ///
    /// Returns error if shader compilation fails
    pub async fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Scaled Dot-Product Attention Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../attention_scaled_dot_product.wgsl").into(),
            ),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Attention Bind Group Layout"),
            entries: &[
                // Query buffer
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
                // Key buffer
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
                // Value buffer
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
                // Mask buffer (optional)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
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
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Attention weights output (optional)
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Config buffer (batch, seq_len, d_k, d_v)
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
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
            label: Some("Attention Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Scaled Dot-Product Attention Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "scaled_dot_product_attention",
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
        })
    }

    /// Execute attention computation
    ///
    /// # Arguments
    ///
    /// * `query` - Query matrix [batch, seq_len, d_k]
    /// * `key` - Key matrix [batch, seq_len, d_k]
    /// * `value` - Value matrix [batch, seq_len, d_v]
    /// * `mask` - Optional attention mask [batch, seq_len, seq_len]
    /// * `batch` - Batch size
    /// * `seq_len` - Sequence length
    /// * `d_k` - Key/query dimension
    /// * `d_v` - Value dimension
    ///
    /// # Returns
    ///
    /// Tuple of (output, attention_weights):
    /// - output: [batch, seq_len, d_v]
    /// - attention_weights: [batch, seq_len, seq_len]
    ///
    /// # Errors
    ///
    /// Returns error if GPU execution fails
    pub async fn execute(
        &self,
        query: &[f32],
        key: &[f32],
        value: &[f32],
        mask: Option<&[f32]>,
        batch: u32,
        seq_len: u32,
        d_k: u32,
        d_v: u32,
    ) -> Result<(Vec<f32>, Vec<f32>)> {
        // Validate dimensions
        let expected_qk_size = (batch * seq_len * d_k) as usize;
        let expected_v_size = (batch * seq_len * d_v) as usize;

        anyhow::ensure!(
            query.len() == expected_qk_size,
            "Query size mismatch: expected {}, got {}",
            expected_qk_size,
            query.len()
        );
        anyhow::ensure!(
            key.len() == expected_qk_size,
            "Key size mismatch: expected {}, got {}",
            expected_qk_size,
            key.len()
        );
        anyhow::ensure!(
            value.len() == expected_v_size,
            "Value size mismatch: expected {}, got {}",
            expected_v_size,
            value.len()
        );

        // Create GPU buffers
        let query_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Query Buffer"),
                contents: bytemuck::cast_slice(query),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let key_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Key Buffer"),
                contents: bytemuck::cast_slice(key),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let value_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Value Buffer"),
                contents: bytemuck::cast_slice(value),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Mask buffer (use dummy if not provided)
        let mask_data: Vec<f32> = if let Some(m) = mask {
            m.to_vec()
        } else {
            vec![1.0f32; (batch * seq_len * seq_len) as usize]
        };
        let mask_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mask Buffer"),
                contents: bytemuck::cast_slice(&mask_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Output buffers
        let output_size = (batch * seq_len * d_v) as u64 * 4; // f32 = 4 bytes
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let attention_weights_size = (batch * seq_len * seq_len) as u64 * 4;
        let attention_weights_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Attention Weights Buffer"),
            size: attention_weights_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Config buffer
        let config_data = [batch, seq_len, d_k, d_v];
        let config_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Config Buffer"),
                contents: bytemuck::cast_slice(&config_data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Attention Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: query_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: key_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: value_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: mask_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: attention_weights_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: config_buffer.as_entire_binding(),
                },
            ],
        });

        // Execute compute pass
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Attention Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Attention Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch: one workgroup per (batch, sequence_position)
            let workgroup_size = 256;
            let num_workgroups = ((batch * seq_len) + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        // Create staging buffers for readback
        let output_staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Staging Buffer"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let weights_staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Weights Staging Buffer"),
            size: attention_weights_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(&output_buffer, 0, &output_staging, 0, output_size);
        encoder.copy_buffer_to_buffer(
            &attention_weights_buffer,
            0,
            &weights_staging,
            0,
            attention_weights_size,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read back results
        let output_slice = output_staging.slice(..);
        let weights_slice = weights_staging.slice(..);

        let (output_sender, output_receiver) =
            futures_intrusive::channel::shared::oneshot_channel();
        let (weights_sender, weights_receiver) =
            futures_intrusive::channel::shared::oneshot_channel();

        output_slice.map_async(wgpu::MapMode::Read, move |result| {
            output_sender.send(result).ok();
        });
        weights_slice.map_async(wgpu::MapMode::Read, move |result| {
            weights_sender.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);

        output_receiver
            .receive()
            .await
            .context("Failed to map output buffer")?
            .context("Output buffer mapping failed")?;
        weights_receiver
            .receive()
            .await
            .context("Failed to map weights buffer")?
            .context("Weights buffer mapping failed")?;

        let output_data = output_slice.get_mapped_range();
        let weights_data = weights_slice.get_mapped_range();

        let output = bytemuck::cast_slice(&output_data).to_vec();
        let attention_weights = bytemuck::cast_slice(&weights_data).to_vec();

        Ok((output, attention_weights))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_device() -> Result<(Arc<wgpu::Device>, Arc<wgpu::Queue>)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .context("No GPU adapter found")?;

        let (device, queue) = adapter
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
            .context("Failed to create device")?;

        Ok((Arc::new(device), Arc::new(queue)))
    }

    #[tokio::test]
    async fn test_scaled_dot_product_attention_creation() {
        if let Ok((device, queue)) = create_test_device().await {
            let result = ScaledDotProductAttention::new(device, queue).await;
            assert!(result.is_ok(), "Failed to create attention operation");
        }
    }

    #[tokio::test]
    async fn test_attention_small_input() {
        if let Ok((device, queue)) = create_test_device().await {
            if let Ok(attention) = ScaledDotProductAttention::new(device, queue).await {
                // Small test: batch=1, seq_len=4, d_k=d_v=8
                let batch = 1;
                let seq_len = 4;
                let d_k = 8;
                let d_v = 8;

                let query = vec![1.0f32; (batch * seq_len * d_k) as usize];
                let key = vec![1.0f32; (batch * seq_len * d_k) as usize];
                let value = vec![1.0f32; (batch * seq_len * d_v) as usize];

                let result = attention
                    .execute(&query, &key, &value, None, batch, seq_len, d_k, d_v)
                    .await;
                assert!(result.is_ok(), "Attention execution failed");

                if let Ok((output, weights)) = result {
                    assert_eq!(output.len(), (batch * seq_len * d_v) as usize);
                    assert_eq!(weights.len(), (batch * seq_len * seq_len) as usize);
                }
            }
        }
    }
}
