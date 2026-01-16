//! Loss Functions for Training
//!
//! 7 loss functions for various ML tasks (classification, regression, segmentation).
//! All operations run on GPU with configurable reduction modes (none, mean, sum).

use anyhow::{Context, Result};
use wgpu::util::DeviceExt;

use super::super::{executor::WgpuExecutor, types::*};
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
        let squared_errors: Vec<f32> = self.read_buffer(&staging_buffer, size).await?;

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
        let absolute_errors: Vec<f32> = self.read_buffer(&staging_buffer, size).await?;

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

        let losses: Vec<f32> = self.read_buffer(&staging_buffer, size).await?;

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

        let losses: Vec<f32> = self.read_buffer(&staging_buffer, size).await?;

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

        let losses: Vec<f32> = self.read_buffer(&staging_buffer, size).await?;

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

        let losses: Vec<f32> = self.read_buffer(&staging_buffer, batch_size).await?;

        let loss = match config.reduction {
            LossReduction::Mean => losses.iter().sum::<f32>() / batch_size as f32,
            LossReduction::Sum => losses.iter().sum::<f32>(),
            LossReduction::None => return Ok(losses[0]),
        };

        Ok(loss)
    }
}
