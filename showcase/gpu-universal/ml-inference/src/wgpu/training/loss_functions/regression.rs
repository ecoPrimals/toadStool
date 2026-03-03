// SPDX-License-Identifier: AGPL-3.0-or-later
//! Regression losses: MSE, MAE, Huber

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::super::super::{
    executor::WgpuExecutor,
    types::{HuberLossConfig, LossReduction, RegressionLossConfig},
};

impl WgpuExecutor {
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

        let shader_source = include_str!("../../../shaders/mse_loss.wgsl");

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
            std::mem::size_of_val(predictions) as u64,
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

        let shader_source = include_str!("../../../shaders/mae_loss.wgsl");

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
            std::mem::size_of_val(predictions) as u64,
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

        let shader_source = include_str!("../../../shaders/huber_loss.wgsl");

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
            std::mem::size_of_val(predictions) as u64,
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
}
