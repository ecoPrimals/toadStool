//! Optimizers for Training
//!
//! 6 gradient-based optimizers (SGD, Adam, RMSprop, Adagrad, NAdam, Adadelta).
//! All operations run on GPU for efficient parameter updates during training.

use anyhow::{Context, Result};
use wgpu::util::DeviceExt;

use super::super::{executor::WgpuExecutor, types::*};
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
