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

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("CrossEntropy Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("CrossEntropy Layout"),
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

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("CrossEntropy Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("CrossEntropy Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "compute_loss",
                compilation_options: Default::default(),
                cache: None,
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
        anyhow::ensure!(
            m.len() == num_params,
            "Adam: m buffer size must equal params size"
        );
        anyhow::ensure!(
            v.len() == num_params,
            "Adam: v buffer size must equal params size"
        );
        anyhow::ensure!(step > 0, "Adam: step must be >= 1");

        let shader_source = include_str!("../shaders/adam.wgsl");

        // Create buffers
        let gradients_buffer = self.create_input_buffer(gradients, "Adam Gradients");
        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Adam Params"),
                contents: bytemuck::cast_slice(params.as_slice()),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });
        let m_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Adam M"),
                contents: bytemuck::cast_slice(m.as_slice()),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });
        let v_buffer = self
            .device
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

    /// Execute SGD (Stochastic Gradient Descent) optimizer step
    ///
    /// Fundamental optimization algorithm with optional momentum.
    /// Updates weights in-place based on gradients.
    ///
    /// # Arguments
    /// * `weights` - Current weights (mutable, updated in-place)
    /// * `gradients` - Computed gradients
    /// * `velocity` - Momentum buffer (mutable, updated if momentum > 0)
    /// * `config` - SGD configuration (learning rate, momentum, etc.)
    ///
    /// Deep Debt: All hyperparameters configured at runtime.
    pub async fn execute_sgd(
        &self,
        weights: &mut [f32],
        gradients: &[f32],
        velocity: &mut [f32],
        config: SgdConfig,
    ) -> Result<()> {
        let num_params = weights.len();
        anyhow::ensure!(
            gradients.len() == num_params,
            "SGD: gradients length must match weights length"
        );
        anyhow::ensure!(
            velocity.len() == num_params,
            "SGD: velocity length must match weights length"
        );

        let shader_source = include_str!("../shaders/sgd.wgsl");

        // Create buffers
        let weights_buffer = self.create_input_buffer(weights, "SGD Weights");
        let gradients_buffer = self.create_input_buffer(gradients, "SGD Gradients");
        let velocity_in_buffer = self.create_input_buffer(velocity, "SGD Velocity In");
        let weights_out_buffer = self.create_output_buffer(num_params, "SGD Weights Out");
        let velocity_out_buffer = self.create_output_buffer(num_params, "SGD Velocity Out");

        // Staging buffers for readback
        let weights_staging = self.create_staging_buffer(num_params, "SGD Weights Staging");
        let velocity_staging = self.create_staging_buffer(num_params, "SGD Velocity Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct SgdParams {
            learning_rate: f32,
            momentum: f32,
            weight_decay: f32,
            dampening: f32,
        }

        let params = SgdParams {
            learning_rate: config.learning_rate,
            momentum: config.momentum,
            weight_decay: config.weight_decay,
            dampening: config.dampening,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SGD Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("SGD Layout"),
                    entries: &[
                        // Weights (read-only)
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
                        // Gradients (read-only)
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
                        // Velocity in (read-only)
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
                        // Weights out (read-write)
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
                        // Velocity out (read-write)
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
                        // Params (uniform)
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
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
            label: Some("SGD Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: weights_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gradients_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: velocity_in_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: weights_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: velocity_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create pipeline
        let pipeline = self.create_simple_pipeline(shader_source, "SGD", &bind_group_layout);

        // Execute
        let workgroups = self.calculate_workgroups(num_params, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "SGD");

        // Copy outputs to staging
        encoder.copy_buffer_to_buffer(
            &weights_out_buffer,
            0,
            &weights_staging,
            0,
            (num_params * std::mem::size_of::<f32>()) as u64,
        );
        encoder.copy_buffer_to_buffer(
            &velocity_out_buffer,
            0,
            &velocity_staging,
            0,
            (num_params * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read back updated weights
        let weights_slice = weights_staging.slice(..);
        let (sender1, receiver1) = futures_intrusive::channel::shared::oneshot_channel();
        weights_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender1.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver1
            .receive()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive weights buffer mapping"))?
            .context("Failed to map weights buffer")?;

        let weights_data = weights_slice.get_mapped_range();
        weights.copy_from_slice(bytemuck::cast_slice(&weights_data));
        drop(weights_data);
        weights_staging.unmap();

        // Read back updated velocity
        let velocity_slice = velocity_staging.slice(..);
        let (sender2, receiver2) = futures_intrusive::channel::shared::oneshot_channel();
        velocity_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender2.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver2
            .receive()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive velocity buffer mapping"))?
            .context("Failed to map velocity buffer")?;

        let velocity_data = velocity_slice.get_mapped_range();
        velocity.copy_from_slice(bytemuck::cast_slice(&velocity_data));
        drop(velocity_data);
        velocity_staging.unmap();

        Ok(())
    }

    /// Execute MSE (Mean Squared Error) loss
    ///
    /// Fundamental regression loss function.
    /// Computes (predictions - targets)² with configurable reduction.
    ///
    /// Deep Debt: Reduction mode determined at runtime.
    pub async fn execute_mse_loss(
        &self,
        predictions: &[f32],
        targets: &[f32],
        config: RegressionLossConfig,
    ) -> Result<f32> {
        let size = predictions.len();
        anyhow::ensure!(
            targets.len() == size,
            "MSE: targets length must match predictions length"
        );

        let shader_source = include_str!("../shaders/mse_loss.wgsl");

        let predictions_buffer = self.create_input_buffer(predictions, "MSE Predictions");
        let targets_buffer = self.create_input_buffer(targets, "MSE Targets");
        let output_buffer = self.create_output_buffer(size, "MSE Output");
        let staging_buffer = self.create_staging_buffer(size, "MSE Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct MseParams {
            reduction_mode: u32, // 0=mean, 1=sum, 2=none
            size: u32,
            _padding: [u32; 2],
        }

        let reduction_mode = match config.reduction {
            LossReduction::Mean => 0,
            LossReduction::Sum => 1,
            LossReduction::None => 2,
        };

        let params = MseParams {
            reduction_mode,
            size: size as u32,
            _padding: [0; 2],
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("MSE Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("MSE Layout"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MSE Bind Group"),
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
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "MSE", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "MSE");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read back results
        let squared_errors = self.read_buffer(&staging_buffer, size).await?;

        // Apply final reduction on CPU (efficient for small final step)
        let loss = match config.reduction {
            LossReduction::Mean => squared_errors.iter().sum::<f32>() / size as f32,
            LossReduction::Sum => squared_errors.iter().sum::<f32>(),
            LossReduction::None => {
                // Return first element (shader writes per-element)
                return Ok(squared_errors[0]);
            }
        };

        Ok(loss)
    }

    /// Execute MAE (Mean Absolute Error) loss / L1 Loss
    ///
    /// Regression loss more robust to outliers than MSE.
    /// Computes |predictions - targets| with configurable reduction.
    ///
    /// Deep Debt: Reduction mode determined at runtime.
    pub async fn execute_mae_loss(
        &self,
        predictions: &[f32],
        targets: &[f32],
        config: RegressionLossConfig,
    ) -> Result<f32> {
        let size = predictions.len();
        anyhow::ensure!(
            targets.len() == size,
            "MAE: targets length must match predictions length"
        );

        let shader_source = include_str!("../shaders/mae_loss.wgsl");

        let predictions_buffer = self.create_input_buffer(predictions, "MAE Predictions");
        let targets_buffer = self.create_input_buffer(targets, "MAE Targets");
        let output_buffer = self.create_output_buffer(size, "MAE Output");
        let staging_buffer = self.create_staging_buffer(size, "MAE Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct MaeParams {
            reduction_mode: u32, // 0=mean, 1=sum, 2=none
            size: u32,
            _padding: [u32; 2],
        }

        let reduction_mode = match config.reduction {
            LossReduction::Mean => 0,
            LossReduction::Sum => 1,
            LossReduction::None => 2,
        };

        let params = MaeParams {
            reduction_mode,
            size: size as u32,
            _padding: [0; 2],
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("MAE Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout (same as MSE)
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("MAE Layout"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MAE Bind Group"),
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
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "MAE", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "MAE");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read back results
        let absolute_errors = self.read_buffer(&staging_buffer, size).await?;

        // Apply final reduction on CPU
        let loss = match config.reduction {
            LossReduction::Mean => absolute_errors.iter().sum::<f32>() / size as f32,
            LossReduction::Sum => absolute_errors.iter().sum::<f32>(),
            LossReduction::None => {
                // Return first element
                return Ok(absolute_errors[0]);
            }
        };

        Ok(loss)
    }

    /// Execute RMSprop (Root Mean Square Propagation) optimizer step
    ///
    /// Adaptive learning rate optimizer that addresses AdaGrad's diminishing learning rates.
    /// Maintains moving average of squared gradients for per-parameter adaptive rates.
    ///
    /// # Arguments
    /// * `weights` - Current weights (mutable, updated in-place)
    /// * `gradients` - Computed gradients
    /// * `square_avg` - Running average of squared gradients (mutable)
    /// * `config` - RMSprop configuration
    ///
    /// Deep Debt: All hyperparameters configured at runtime.
    pub async fn execute_rmsprop(
        &self,
        weights: &mut [f32],
        gradients: &[f32],
        square_avg: &mut [f32],
        config: RmspropConfig,
    ) -> Result<()> {
        let num_params = weights.len();
        anyhow::ensure!(
            gradients.len() == num_params,
            "RMSprop: gradients length must match weights length"
        );
        anyhow::ensure!(
            square_avg.len() == num_params,
            "RMSprop: square_avg length must match weights length"
        );

        let shader_source = include_str!("../shaders/rmsprop.wgsl");

        // Create buffers
        let weights_buffer = self.create_input_buffer(weights, "RMSprop Weights");
        let gradients_buffer = self.create_input_buffer(gradients, "RMSprop Gradients");
        let sq_avg_in_buffer = self.create_input_buffer(square_avg, "RMSprop SqAvg In");
        let weights_out_buffer = self.create_output_buffer(num_params, "RMSprop Weights Out");
        let sq_avg_out_buffer = self.create_output_buffer(num_params, "RMSprop SqAvg Out");

        // Staging buffers for readback
        let weights_staging = self.create_staging_buffer(num_params, "RMSprop Weights Staging");
        let sq_avg_staging = self.create_staging_buffer(num_params, "RMSprop SqAvg Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct RmspropParams {
            learning_rate: f32,
            alpha: f32,
            epsilon: f32,
            weight_decay: f32,
        }

        let params = RmspropParams {
            learning_rate: config.learning_rate,
            alpha: config.alpha,
            epsilon: config.epsilon,
            weight_decay: config.weight_decay,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("RMSprop Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("RMSprop Layout"),
                    entries: &[
                        // Weights (read-only)
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
                        // Gradients (read-only)
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
                        // Square average in (read-only)
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
                        // Weights out (read-write)
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
                        // Square average out (read-write)
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
                        // Params (uniform)
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
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
            label: Some("RMSprop Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: weights_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gradients_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: sq_avg_in_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: weights_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: sq_avg_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create pipeline
        let pipeline = self.create_simple_pipeline(shader_source, "RMSprop", &bind_group_layout);

        // Execute
        let workgroups = self.calculate_workgroups(num_params, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "RMSprop");

        // Copy outputs to staging
        encoder.copy_buffer_to_buffer(
            &weights_out_buffer,
            0,
            &weights_staging,
            0,
            (num_params * std::mem::size_of::<f32>()) as u64,
        );
        encoder.copy_buffer_to_buffer(
            &sq_avg_out_buffer,
            0,
            &sq_avg_staging,
            0,
            (num_params * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read back updated weights
        let weights_slice = weights_staging.slice(..);
        let (sender1, receiver1) = futures_intrusive::channel::shared::oneshot_channel();
        weights_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender1.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver1
            .receive()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive weights buffer mapping"))?
            .context("Failed to map weights buffer")?;

        let weights_data = weights_slice.get_mapped_range();
        weights.copy_from_slice(bytemuck::cast_slice(&weights_data));
        drop(weights_data);
        weights_staging.unmap();

        // Read back updated square average
        let sq_avg_slice = sq_avg_staging.slice(..);
        let (sender2, receiver2) = futures_intrusive::channel::shared::oneshot_channel();
        sq_avg_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender2.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver2
            .receive()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive square_avg buffer mapping"))?
            .context("Failed to map square_avg buffer")?;

        let sq_avg_data = sq_avg_slice.get_mapped_range();
        square_avg.copy_from_slice(bytemuck::cast_slice(&sq_avg_data));
        drop(sq_avg_data);
        sq_avg_staging.unmap();

        Ok(())
    }

    /// Execute Huber Loss (Smooth L1 Loss)
    ///
    /// Robust regression loss less sensitive to outliers than MSE.
    /// Combines quadratic loss (for small errors) with linear loss (for large errors).
    ///
    /// Used in: Robust regression, DQN reinforcement learning
    ///
    /// Deep Debt: Delta threshold and reduction mode configured at runtime.
    pub async fn execute_huber_loss(
        &self,
        predictions: &[f32],
        targets: &[f32],
        config: HuberLossConfig,
    ) -> Result<f32> {
        let size = predictions.len();
        anyhow::ensure!(
            targets.len() == size,
            "Huber: targets length must match predictions length"
        );

        let shader_source = include_str!("../shaders/huber_loss.wgsl");

        let predictions_buffer = self.create_input_buffer(predictions, "Huber Predictions");
        let targets_buffer = self.create_input_buffer(targets, "Huber Targets");
        let output_buffer = self.create_output_buffer(size, "Huber Output");
        let staging_buffer = self.create_staging_buffer(size, "Huber Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct HuberParams {
            delta: f32,
            reduction_mode: u32,
            size: u32,
            _padding: u32,
        }

        let reduction_mode = match config.reduction {
            LossReduction::Mean => 0,
            LossReduction::Sum => 1,
            LossReduction::None => 2,
        };

        let params = HuberParams {
            delta: config.delta,
            reduction_mode,
            size: size as u32,
            _padding: 0,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Huber Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Huber Layout"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Huber Bind Group"),
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
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "Huber", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Huber");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        let losses = self.read_buffer(&staging_buffer, size).await?;

        let loss = match config.reduction {
            LossReduction::Mean => losses.iter().sum::<f32>() / size as f32,
            LossReduction::Sum => losses.iter().sum::<f32>(),
            LossReduction::None => return Ok(losses[0]),
        };

        Ok(loss)
    }

    /// Execute BCE (Binary Cross Entropy) Loss
    ///
    /// Binary classification loss function.
    /// BCE(p, t) = -[t * log(p) + (1 - t) * log(1 - p)]
    ///
    /// Used in: Binary classification, multi-label classification, GANs
    ///
    /// Deep Debt: Epsilon and reduction mode configured at runtime.
    pub async fn execute_bce_loss(
        &self,
        predictions: &[f32],
        targets: &[f32],
        config: BceLossConfig,
    ) -> Result<f32> {
        let size = predictions.len();
        anyhow::ensure!(
            targets.len() == size,
            "BCE: targets length must match predictions length"
        );

        let shader_source = include_str!("../shaders/bce_loss.wgsl");

        let predictions_buffer = self.create_input_buffer(predictions, "BCE Predictions");
        let targets_buffer = self.create_input_buffer(targets, "BCE Targets");
        let output_buffer = self.create_output_buffer(size, "BCE Output");
        let staging_buffer = self.create_staging_buffer(size, "BCE Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct BceParams {
            epsilon: f32,
            reduction_mode: u32,
            size: u32,
            _padding: u32,
        }

        let reduction_mode = match config.reduction {
            LossReduction::Mean => 0,
            LossReduction::Sum => 1,
            LossReduction::None => 2,
        };

        let params = BceParams {
            epsilon: config.epsilon,
            reduction_mode,
            size: size as u32,
            _padding: 0,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("BCE Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("BCE Layout"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BCE Bind Group"),
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
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "BCE", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "BCE");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        let losses = self.read_buffer(&staging_buffer, size).await?;

        let loss = match config.reduction {
            LossReduction::Mean => losses.iter().sum::<f32>() / size as f32,
            LossReduction::Sum => losses.iter().sum::<f32>(),
            LossReduction::None => return Ok(losses[0]),
        };

        Ok(loss)
    }

    /// Execute AdaGrad (Adaptive Gradient) optimizer step
    ///
    /// Adapts learning rate for each parameter based on historical gradients.
    /// Particularly effective for sparse features and NLP tasks.
    ///
    /// # Arguments
    /// * `weights` - Current weights (mutable, updated in-place)
    /// * `gradients` - Computed gradients
    /// * `accumulated` - Sum of squared gradients (mutable)
    /// * `config` - AdaGrad configuration
    ///
    /// Deep Debt: All hyperparameters configured at runtime.
    pub async fn execute_adagrad(
        &self,
        weights: &mut [f32],
        gradients: &[f32],
        accumulated: &mut [f32],
        config: AdagradConfig,
    ) -> Result<()> {
        let num_params = weights.len();
        anyhow::ensure!(
            gradients.len() == num_params,
            "AdaGrad: gradients length must match weights length"
        );
        anyhow::ensure!(
            accumulated.len() == num_params,
            "AdaGrad: accumulated length must match weights length"
        );

        let shader_source = include_str!("../shaders/adagrad.wgsl");

        let weights_buffer = self.create_input_buffer(weights, "AdaGrad Weights");
        let gradients_buffer = self.create_input_buffer(gradients, "AdaGrad Gradients");
        let acc_in_buffer = self.create_input_buffer(accumulated, "AdaGrad Acc In");
        let weights_out_buffer = self.create_output_buffer(num_params, "AdaGrad Weights Out");
        let acc_out_buffer = self.create_output_buffer(num_params, "AdaGrad Acc Out");

        let weights_staging = self.create_staging_buffer(num_params, "AdaGrad Weights Staging");
        let acc_staging = self.create_staging_buffer(num_params, "AdaGrad Acc Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct AdagradParams {
            learning_rate: f32,
            epsilon: f32,
            weight_decay: f32,
            _padding: u32,
        }

        let params = AdagradParams {
            learning_rate: config.learning_rate,
            epsilon: config.epsilon,
            weight_decay: config.weight_decay,
            _padding: 0,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("AdaGrad Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("AdaGrad Layout"),
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
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
            label: Some("AdaGrad Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: weights_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gradients_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: acc_in_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: weights_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: acc_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "AdaGrad", &bind_group_layout);
        let workgroups = self.calculate_workgroups(num_params, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "AdaGrad");

        encoder.copy_buffer_to_buffer(
            &weights_out_buffer,
            0,
            &weights_staging,
            0,
            (num_params * std::mem::size_of::<f32>()) as u64,
        );
        encoder.copy_buffer_to_buffer(
            &acc_out_buffer,
            0,
            &acc_staging,
            0,
            (num_params * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read back updated weights
        let weights_slice = weights_staging.slice(..);
        let (sender1, receiver1) = futures_intrusive::channel::shared::oneshot_channel();
        weights_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender1.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver1
            .receive()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive weights buffer mapping"))?
            .context("Failed to map weights buffer")?;

        let weights_data = weights_slice.get_mapped_range();
        weights.copy_from_slice(bytemuck::cast_slice(&weights_data));
        drop(weights_data);
        weights_staging.unmap();

        // Read back updated accumulated
        let acc_slice = acc_staging.slice(..);
        let (sender2, receiver2) = futures_intrusive::channel::shared::oneshot_channel();
        acc_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender2.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver2
            .receive()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive accumulated buffer mapping"))?
            .context("Failed to map accumulated buffer")?;

        let acc_data = acc_slice.get_mapped_range();
        accumulated.copy_from_slice(bytemuck::cast_slice(&acc_data));
        drop(acc_data);
        acc_staging.unmap();

        Ok(())
    }

    /// Execute NAdam (Nesterov-accelerated Adam) optimizer step
    ///
    /// Combines Adam with Nesterov momentum for faster convergence.
    /// Often outperforms Adam on complex optimization landscapes.
    ///
    /// # Arguments
    /// * `weights` - Current weights (mutable, updated in-place)
    /// * `gradients` - Computed gradients
    /// * `m` - First moment estimate (mutable)
    /// * `v` - Second moment estimate (mutable)
    /// * `step` - Current step number (for bias correction)
    /// * `config` - NAdam configuration
    ///
    /// Deep Debt: All hyperparameters configured at runtime.
    pub async fn execute_nadam(
        &self,
        weights: &mut [f32],
        gradients: &[f32],
        m: &mut [f32],
        v: &mut [f32],
        step: usize,
        config: NadamConfig,
    ) -> Result<()> {
        let num_params = weights.len();
        anyhow::ensure!(
            gradients.len() == num_params,
            "NAdam: gradients length must match weights length"
        );
        anyhow::ensure!(
            m.len() == num_params && v.len() == num_params,
            "NAdam: m and v lengths must match weights length"
        );

        let shader_source = include_str!("../shaders/nadam.wgsl");

        let weights_buffer = self.create_input_buffer(weights, "NAdam Weights");
        let gradients_buffer = self.create_input_buffer(gradients, "NAdam Gradients");
        let m_buffer = self.create_input_buffer(m, "NAdam M");
        let v_buffer = self.create_input_buffer(v, "NAdam V");
        let weights_out_buffer = self.create_output_buffer(num_params, "NAdam Weights Out");
        let m_out_buffer = self.create_output_buffer(num_params, "NAdam M Out");
        let v_out_buffer = self.create_output_buffer(num_params, "NAdam V Out");

        let weights_staging = self.create_staging_buffer(num_params, "NAdam Weights Staging");
        let m_staging = self.create_staging_buffer(num_params, "NAdam M Staging");
        let v_staging = self.create_staging_buffer(num_params, "NAdam V Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct NadamParams {
            learning_rate: f32,
            beta1: f32,
            beta2: f32,
            epsilon: f32,
            weight_decay: f32,
            step: u32,
            _padding: [u32; 2],
        }

        let params = NadamParams {
            learning_rate: config.learning_rate,
            beta1: config.beta1,
            beta2: config.beta2,
            epsilon: config.epsilon,
            weight_decay: config.weight_decay,
            step: step as u32,
            _padding: [0; 2],
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("NAdam Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("NAdam Layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 7,
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
            label: Some("NAdam Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: weights_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gradients_buffer.as_entire_binding(),
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
                    resource: weights_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: m_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: v_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "NAdam", &bind_group_layout);
        let workgroups = self.calculate_workgroups(num_params, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "NAdam");

        encoder.copy_buffer_to_buffer(
            &weights_out_buffer,
            0,
            &weights_staging,
            0,
            (num_params * std::mem::size_of::<f32>()) as u64,
        );
        encoder.copy_buffer_to_buffer(
            &m_out_buffer,
            0,
            &m_staging,
            0,
            (num_params * std::mem::size_of::<f32>()) as u64,
        );
        encoder.copy_buffer_to_buffer(
            &v_out_buffer,
            0,
            &v_staging,
            0,
            (num_params * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read back weights, m, and v (similar pattern as Adam)
        let weights_slice = weights_staging.slice(..);
        let (sender1, receiver1) = futures_intrusive::channel::shared::oneshot_channel();
        weights_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender1.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver1
            .receive()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive weights buffer mapping"))?
            .context("Failed to map weights buffer")?;

        let weights_data = weights_slice.get_mapped_range();
        weights.copy_from_slice(bytemuck::cast_slice(&weights_data));
        drop(weights_data);
        weights_staging.unmap();

        // Read m
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

        // Read v
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

    /// Execute AdaDelta optimizer step
    ///
    /// Extension of AdaGrad that reduces monotonically decreasing learning rate.
    /// NO learning rate hyperparameter needed - automatically adapted!
    ///
    /// # Arguments
    /// * `weights` - Current weights (mutable, updated in-place)
    /// * `gradients` - Computed gradients
    /// * `acc_grad` - Accumulated gradient squared (mutable)
    /// * `acc_delta` - Accumulated delta squared (mutable)
    /// * `config` - AdaDelta configuration
    ///
    /// Deep Debt: Self-adapting, no learning rate tuning required.
    pub async fn execute_adadelta(
        &self,
        weights: &mut [f32],
        gradients: &[f32],
        acc_grad: &mut [f32],
        acc_delta: &mut [f32],
        config: AdadeltaConfig,
    ) -> Result<()> {
        let num_params = weights.len();
        anyhow::ensure!(
            gradients.len() == num_params,
            "AdaDelta: gradients length must match weights length"
        );
        anyhow::ensure!(
            acc_grad.len() == num_params && acc_delta.len() == num_params,
            "AdaDelta: acc_grad and acc_delta lengths must match weights length"
        );

        let shader_source = include_str!("../shaders/adadelta.wgsl");

        let weights_buffer = self.create_input_buffer(weights, "AdaDelta Weights");
        let gradients_buffer = self.create_input_buffer(gradients, "AdaDelta Gradients");
        let acc_grad_in_buffer = self.create_input_buffer(acc_grad, "AdaDelta AccGrad In");
        let acc_delta_in_buffer = self.create_input_buffer(acc_delta, "AdaDelta AccDelta In");
        let weights_out_buffer = self.create_output_buffer(num_params, "AdaDelta Weights Out");
        let acc_grad_out_buffer = self.create_output_buffer(num_params, "AdaDelta AccGrad Out");
        let acc_delta_out_buffer = self.create_output_buffer(num_params, "AdaDelta AccDelta Out");

        let weights_staging = self.create_staging_buffer(num_params, "AdaDelta Weights Staging");
        let acc_grad_staging = self.create_staging_buffer(num_params, "AdaDelta AccGrad Staging");
        let acc_delta_staging = self.create_staging_buffer(num_params, "AdaDelta AccDelta Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct AdadeltaParams {
            rho: f32,
            epsilon: f32,
            weight_decay: f32,
            _padding: u32,
        }

        let params = AdadeltaParams {
            rho: config.rho,
            epsilon: config.epsilon,
            weight_decay: config.weight_decay,
            _padding: 0,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("AdaDelta Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("AdaDelta Layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 7,
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
            label: Some("AdaDelta Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: weights_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gradients_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: acc_grad_in_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: acc_delta_in_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: weights_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: acc_grad_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: acc_delta_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "AdaDelta", &bind_group_layout);
        let workgroups = self.calculate_workgroups(num_params, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "AdaDelta");

        encoder.copy_buffer_to_buffer(
            &weights_out_buffer,
            0,
            &weights_staging,
            0,
            (num_params * std::mem::size_of::<f32>()) as u64,
        );
        encoder.copy_buffer_to_buffer(
            &acc_grad_out_buffer,
            0,
            &acc_grad_staging,
            0,
            (num_params * std::mem::size_of::<f32>()) as u64,
        );
        encoder.copy_buffer_to_buffer(
            &acc_delta_out_buffer,
            0,
            &acc_delta_staging,
            0,
            (num_params * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read back weights
        let weights_slice = weights_staging.slice(..);
        let (sender1, receiver1) = futures_intrusive::channel::shared::oneshot_channel();
        weights_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender1.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver1
            .receive()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive weights buffer mapping"))?
            .context("Failed to map weights buffer")?;

        let weights_data = weights_slice.get_mapped_range();
        weights.copy_from_slice(bytemuck::cast_slice(&weights_data));
        drop(weights_data);
        weights_staging.unmap();

        // Read acc_grad
        let acc_grad_slice = acc_grad_staging.slice(..);
        let (sender2, receiver2) = futures_intrusive::channel::shared::oneshot_channel();
        acc_grad_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender2.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver2
            .receive()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive acc_grad buffer mapping"))?
            .context("Failed to map acc_grad buffer")?;

        let acc_grad_data = acc_grad_slice.get_mapped_range();
        acc_grad.copy_from_slice(bytemuck::cast_slice(&acc_grad_data));
        drop(acc_grad_data);
        acc_grad_staging.unmap();

        // Read acc_delta
        let acc_delta_slice = acc_delta_staging.slice(..);
        let (sender3, receiver3) = futures_intrusive::channel::shared::oneshot_channel();
        acc_delta_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender3.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver3
            .receive()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive acc_delta buffer mapping"))?
            .context("Failed to map acc_delta buffer")?;

        let acc_delta_data = acc_delta_slice.get_mapped_range();
        acc_delta.copy_from_slice(bytemuck::cast_slice(&acc_delta_data));
        drop(acc_delta_data);
        acc_delta_staging.unmap();

        Ok(())
    }

    /// Execute Focal Loss
    ///
    /// Addresses class imbalance by down-weighting easy examples.
    /// FocalLoss = -alpha * (1 - p_t)^gamma * log(p_t)
    ///
    /// Used in: RetinaNet, object detection with severe class imbalance.
    /// Benefits: Focuses training on hard examples, improves rare class detection.
    ///
    /// Deep Debt: Alpha and gamma configured at runtime.
    pub async fn execute_focal_loss(
        &self,
        predictions: &[f32],
        targets: &[f32],
        config: FocalLossConfig,
    ) -> Result<f32> {
        let size = predictions.len();
        anyhow::ensure!(
            targets.len() == size,
            "Focal: targets length must match predictions length"
        );

        let shader_source = include_str!("../shaders/focal_loss.wgsl");

        let predictions_buffer = self.create_input_buffer(predictions, "Focal Predictions");
        let targets_buffer = self.create_input_buffer(targets, "Focal Targets");
        let output_buffer = self.create_output_buffer(size, "Focal Output");
        let staging_buffer = self.create_staging_buffer(size, "Focal Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct FocalParams {
            alpha: f32,          // offset 0, 4 bytes
            gamma: f32,          // offset 4, 4 bytes
            epsilon: f32,        // offset 8, 4 bytes
            reduction_mode: u32, // offset 12, 4 bytes
            size: u32,           // offset 16, 4 bytes
            _pad0: [u32; 3],     // offset 20, 12 bytes - padding to align vec3
            _pad1: [u32; 3],     // offset 32, 12 bytes - vec3<u32>
            _pad2: [u32; 4],     // offset 48, 16 bytes - vec4<u32>
            _pad3: [u32; 4],     // offset 64, 16 bytes - vec4<u32>
            _pad4: [u32; 4],     // offset 80, 16 bytes - vec4<u32>
            _pad5: u32,          // offset 92, 4 bytes - final padding to 96 (multiple of 16)
                                 // Total: 96 bytes (matches WGSL struct alignment requirement)
        }

        let reduction_mode = match config.reduction {
            LossReduction::Mean => 0,
            LossReduction::Sum => 1,
            LossReduction::None => 2,
        };

        let params = FocalParams {
            alpha: config.alpha,
            gamma: config.gamma,
            epsilon: config.epsilon,
            reduction_mode,
            size: size as u32,
            _pad0: [0; 3], // Explicit padding to match WGSL vec3 alignment
            _pad1: [0; 3],
            _pad2: [0; 4],
            _pad3: [0; 4],
            _pad4: [0; 4],
            _pad5: 0, // Final padding to reach 96 bytes
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Focal Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Focal Layout"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Focal Bind Group"),
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
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "Focal", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Focal");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        let losses = self.read_buffer(&staging_buffer, size).await?;

        let loss = match config.reduction {
            LossReduction::Mean => losses.iter().sum::<f32>() / size as f32,
            LossReduction::Sum => losses.iter().sum::<f32>(),
            LossReduction::None => return Ok(losses[0]),
        };

        Ok(loss)
    }

    /// Execute Dice Loss (F1 Loss)
    ///
    /// Measures overlap between predicted and target segmentation masks.
    /// DiceLoss = 1 - (2 * |X ∩ Y|) / (|X| + |Y|)
    ///
    /// Used in: Medical image segmentation, semantic segmentation.
    /// Benefits: Handles class imbalance, directly optimizes IoU-like metric.
    ///
    /// Deep Debt: Smooth factor configured at runtime.
    pub async fn execute_dice_loss(
        &self,
        predictions: &[f32],
        targets: &[f32],
        batch_size: usize,
        elements_per_sample: usize,
        config: DiceLossConfig,
    ) -> Result<f32> {
        let total_size = batch_size * elements_per_sample;
        anyhow::ensure!(
            predictions.len() == total_size,
            "Dice: predictions size must match batch_size * elements_per_sample"
        );
        anyhow::ensure!(
            targets.len() == total_size,
            "Dice: targets size must match batch_size * elements_per_sample"
        );

        let shader_source = include_str!("../shaders/dice_loss.wgsl");

        let predictions_buffer = self.create_input_buffer(predictions, "Dice Predictions");
        let targets_buffer = self.create_input_buffer(targets, "Dice Targets");
        let output_buffer = self.create_output_buffer(batch_size, "Dice Output");
        let staging_buffer = self.create_staging_buffer(batch_size, "Dice Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct DiceParams {
            smooth: f32,
            reduction_mode: u32,
            batch_size: u32,
            elements_per_sample: u32,
        }

        let reduction_mode = match config.reduction {
            LossReduction::Mean => 0,
            LossReduction::Sum => 1,
            LossReduction::None => 2,
        };

        let params = DiceParams {
            smooth: config.smooth,
            reduction_mode,
            batch_size: batch_size as u32,
            elements_per_sample: elements_per_sample as u32,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Dice Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Dice Layout"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Dice Bind Group"),
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
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "Dice", &bind_group_layout);
        let workgroups = batch_size as u32;
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Dice");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (batch_size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        let losses = self.read_buffer(&staging_buffer, batch_size).await?;

        let loss = match config.reduction {
            LossReduction::Mean => losses.iter().sum::<f32>() / batch_size as f32,
            LossReduction::Sum => losses.iter().sum::<f32>(),
            LossReduction::None => return Ok(losses[0]),
        };

        Ok(loss)
    }
}
