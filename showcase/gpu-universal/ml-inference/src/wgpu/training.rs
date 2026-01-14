//! Training operations
//!
//! Loss functions (CrossEntropy) and optimizers (Adam) for neural network training.
//! Full GPU execution for efficient backpropagation and parameter updates.

use anyhow::{Context, Result};
use wgpu::util::DeviceExt;

use super::{executor::WgpuExecutor, types::*};

impl WgpuExecutor {
    /// Execute CrossEntropy Loss: Multi-class classification loss
    ///
    /// Computes negative log-likelihood between predictions and targets.
    /// Supports mean, sum, or per-sample reduction modes.
    ///
    /// Deep Debt: Reduction mode determined at runtime (configurable!).
    pub async fn execute_cross_entropy(
        &self,
        predictions: &[f32],
        targets: &[f32],
        batch_size: usize,
        num_classes: usize,
        config: CrossEntropyConfig,
    ) -> Result<Vec<f32>> {
        let expected_size = batch_size * num_classes;
        anyhow::ensure!(
            predictions.len() == expected_size,
            "CrossEntropy: predictions size must equal batch_size * num_classes"
        );
        anyhow::ensure!(
            targets.len() == expected_size,
            "CrossEntropy: targets size must equal batch_size * num_classes"
        );

        let shader_source = include_str!("../shaders/cross_entropy.wgsl");

        let predictions_buffer = self.create_input_buffer(predictions, "CrossEntropy Predictions");
        let targets_buffer = self.create_input_buffer(targets, "CrossEntropy Targets");

        // Output buffer: per-sample losses
        let output_size = batch_size;
        let losses_buffer = self.create_output_buffer(output_size, "CrossEntropy Losses");
        let staging_buffer = self.create_staging_buffer(output_size, "CrossEntropy Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct CrossEntropyParams {
            batch_size: u32,
            num_classes: u32,
            epsilon: f32,
            reduction: u32, // 0=none, 1=mean, 2=sum
        }

        let reduction_mode = match config.reduction {
            LossReduction::None => 0,
            LossReduction::Mean => 1,
            LossReduction::Sum => 2,
        };

        let params = CrossEntropyParams {
            batch_size: batch_size as u32,
            num_classes: num_classes as u32,
            epsilon: config.epsilon,
            reduction: reduction_mode,
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("CrossEntropy Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let bind_group_layout = self.create_binary_bind_group_layout("CrossEntropy");

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CrossEntropy Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: predictions_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: targets_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: losses_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create shader and pipeline
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("CrossEntropy Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("CrossEntropy Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("CrossEntropy Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "compute_loss",
                });

        let workgroups = self.calculate_workgroups(batch_size, 256);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "CrossEntropy");

        encoder.copy_buffer_to_buffer(
            &losses_buffer,
            0,
            &staging_buffer,
            0,
            (output_size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read per-sample losses
        let losses = self.read_buffer(&staging_buffer, output_size).await?;

        // Apply reduction (Deep Debt: determined at runtime!)
        let result = match config.reduction {
            LossReduction::None => losses,
            LossReduction::Mean => {
                let sum: f32 = losses.iter().sum();
                vec![sum / batch_size as f32]
            }
            LossReduction::Sum => {
                let sum: f32 = losses.iter().sum();
                vec![sum]
            }
        };

        Ok(result)
    }

    /// Execute Adam Optimizer Step: Adaptive moment estimation
    ///
    /// Updates parameters using Adam optimizer with momentum and RMSprop.
    /// All buffers (params, m, v) are updated in-place on GPU then copied back.
    ///
    /// Deep Debt: Learning rate, betas, and weight decay all runtime configurable.
    pub async fn execute_adam_step(
        &self,
        gradients: &[f32],
        params: &mut Vec<f32>,
        m: &mut Vec<f32>,
        v: &mut Vec<f32>,
        step: usize,
        config: AdamConfig,
    ) -> Result<()> {
        let num_params = params.len();

        anyhow::ensure!(
            gradients.len() == num_params,
            "Adam: gradients size must equal params size"
        );
        anyhow::ensure!(m.len() == num_params, "Adam: m buffer size must equal params size");
        anyhow::ensure!(v.len() == num_params, "Adam: v buffer size must equal params size");
        anyhow::ensure!(step > 0, "Adam: step must be >= 1");

        let shader_source = include_str!("../shaders/adam.wgsl");

        // Create buffers
        let gradients_buffer = self.create_input_buffer(gradients, "Adam Gradients");
        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Adam Params"),
                    contents: bytemuck::cast_slice(params.as_slice()),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                });
        let m_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Adam M"),
                    contents: bytemuck::cast_slice(m.as_slice()),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                });
        let v_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Adam V"),
                    contents: bytemuck::cast_slice(v.as_slice()),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct AdamParams {
            num_params: u32,
            learning_rate: f32,
            beta1: f32,
            beta2: f32,
            epsilon: f32,
            weight_decay: f32,
            step: u32,
            _padding: u32,
        }

        let adam_params = AdamParams {
            num_params: num_params as u32,
            learning_rate: config.learning_rate,
            beta1: config.beta1,
            beta2: config.beta2,
            epsilon: config.epsilon,
            weight_decay: config.weight_decay,
            step: step as u32,
            _padding: 0,
        };

        let params_uniform_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Adam Params Uniform"),
                    contents: bytemuck::bytes_of(&adam_params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        // Complex bind group with 5 bindings
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Adam Layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Adam Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: gradients_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: m_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: v_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "Adam", &bind_group_layout);

        // Staging buffers to read back updated values
        let params_staging = self.create_staging_buffer(num_params, "Adam Params Staging");
        let m_staging = self.create_staging_buffer(num_params, "Adam M Staging");
        let v_staging = self.create_staging_buffer(num_params, "Adam V Staging");

        // Execute Adam update
        let workgroups = self.calculate_workgroups(num_params, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Adam");

        // Copy results back to staging buffers
        let size_bytes = (num_params * std::mem::size_of::<f32>()) as u64;
        encoder.copy_buffer_to_buffer(&params_buffer, 0, &params_staging, 0, size_bytes);
        encoder.copy_buffer_to_buffer(&m_buffer, 0, &m_staging, 0, size_bytes);
        encoder.copy_buffer_to_buffer(&v_buffer, 0, &v_staging, 0, size_bytes);

        self.queue.submit(Some(encoder.finish()));

        // Read back updated parameters
        let params_slice = params_staging.slice(..);
        let (sender1, receiver1) = futures_intrusive::channel::shared::oneshot_channel();
        params_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender1.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver1
            .receive()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive params buffer mapping"))?
            .context("Failed to map params buffer")?;

        let params_data = params_slice.get_mapped_range();
        params.copy_from_slice(bytemuck::cast_slice(&params_data));
        drop(params_data);
        params_staging.unmap();

        // Read back updated m
        let m_slice = m_staging.slice(..);
        let (sender2, receiver2) = futures_intrusive::channel::shared::oneshot_channel();
        m_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender2.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver2
            .receive()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive m buffer mapping"))?
            .context("Failed to map m buffer")?;

        let m_data = m_slice.get_mapped_range();
        m.copy_from_slice(bytemuck::cast_slice(&m_data));
        drop(m_data);
        m_staging.unmap();

        // Read back updated v
        let v_slice = v_staging.slice(..);
        let (sender3, receiver3) = futures_intrusive::channel::shared::oneshot_channel();
        v_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender3.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver3
            .receive()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive v buffer mapping"))?
            .context("Failed to map v buffer")?;

        let v_data = v_slice.get_mapped_range();
        v.copy_from_slice(bytemuck::cast_slice(&v_data));
        drop(v_data);
        v_staging.unmap();

        Ok(())
    }
}
