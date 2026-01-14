//! Normalization operations
//!
//! Softmax, LayerNorm, BatchNorm, GroupNorm, etc.
//! Complex multi-pass GPU operations for neural network normalization.

use anyhow::{Context, Result};
use wgpu::util::DeviceExt;

use super::{executor::WgpuExecutor, types::*};

impl WgpuExecutor {
    /// Execute softmax: stable softmax activation (full GPU multi-pass)
    ///
    /// Implementation: Three-pass GPU pipeline for numerical stability
    /// Pass 1: Find max (GPU reduction)
    /// Pass 2: Compute exp(x - max) and sum (GPU)
    /// Pass 3: Normalize (divide by sum, GPU)
    ///
    /// Deep Debt: No hardcoded sizes, all runtime-configured.
    pub async fn execute_softmax(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = input.len();
        let workgroups = self.calculate_workgroups(size, 256).max(1);

        let shader_source = include_str!("../shaders/softmax.wgsl");

        // Create buffers
        let input_buffer = self.create_input_buffer(input, "Softmax Input");
        let output_buffer = self.create_output_buffer(size, "Softmax Output");
        let staging_buffer = self.create_staging_buffer(size, "Softmax Staging");

        // Intermediate buffers for multi-pass algorithm
        let max_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Max Values"),
            size: (workgroups as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let sum_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sum Values"),
            size: (workgroups as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct SoftmaxParams {
            size: u32,
            _padding: [u32; 3], // Align to 16 bytes
        }

        let params = SoftmaxParams {
            size: size as u32,
            _padding: [0; 3],
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Softmax Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        // Create bind group layout (5 bindings for multi-pass)
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Softmax Layout"),
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
            label: Some("Softmax Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: max_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: sum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create pipelines for each pass
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Softmax Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Softmax Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        // Three pipelines for three passes
        let find_max_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Softmax Find Max"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "find_max",
                });

        let exp_sum_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Softmax Exp Sum"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "exp_and_sum",
                });

        let normalize_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Softmax Normalize"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "normalize",
                });

        // Execute three-pass algorithm
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Softmax Encoder"),
            });

        {
            // Pass 1: Find max
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Softmax Find Max Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&find_max_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        {
            // Pass 2: Compute exp and sum
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Softmax Exp Sum Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&exp_sum_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        {
            // Pass 3: Normalize
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Softmax Normalize Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&normalize_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }

    /// Execute LayerNorm: Full GPU multi-pass normalization
    ///
    /// Algorithm: Welford's online algorithm for stable statistics
    /// Formula: output = (input - mean) / sqrt(variance + epsilon) * gamma + beta
    ///
    /// Deep Debt: Full GPU execution, no CPU fallbacks.
    pub async fn execute_layernorm(&self, input: &[f32], config: NormConfig) -> Result<Vec<f32>> {
        let size = input.len();
        anyhow::ensure!(size > 0, "LayerNorm: input cannot be empty");

        let workgroups = self.calculate_workgroups(size, 256).max(1);
        let shader_source = include_str!("../shaders/layernorm.wgsl");

        // Create buffers
        let input_buffer = self.create_input_buffer(input, "LayerNorm Input");

        // Gamma (scale) - default to all 1s if not provided (Deep Debt: configurable!)
        let gamma = config.gamma.unwrap_or_else(|| vec![1.0; size]);
        anyhow::ensure!(
            gamma.len() == size,
            "LayerNorm: gamma size must match input size"
        );
        let gamma_buffer = self.create_input_buffer(&gamma, "LayerNorm Gamma");

        // Beta (shift) - default to all 0s if not provided (Deep Debt: configurable!)
        let beta = config.beta.unwrap_or_else(|| vec![0.0; size]);
        anyhow::ensure!(
            beta.len() == size,
            "LayerNorm: beta size must match input size"
        );
        let beta_buffer = self.create_input_buffer(&beta, "LayerNorm Beta");

        let output_buffer = self.create_output_buffer(size, "LayerNorm Output");
        let staging_buffer = self.create_staging_buffer(size, "LayerNorm Staging");

        // Stats buffer for multi-pass algorithm
        let stats_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerNorm Stats"),
            size: ((workgroups * 2 + 2) * std::mem::size_of::<f32>() as u32) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct LayerNormParams {
            size: u32,
            epsilon: f32,
            _padding: [u32; 2],
        }

        let params = LayerNormParams {
            size: size as u32,
            epsilon: config.epsilon,
            _padding: [0; 2],
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("LayerNorm Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        // Complex bind group for multi-pass algorithm
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("LayerNorm Layout"),
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
            label: Some("LayerNorm Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gamma_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: beta_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: stats_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create pipelines for multi-pass algorithm
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("LayerNorm Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("LayerNorm Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        // Three passes: compute stats, finalize stats, normalize
        let compute_stats =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("LayerNorm Compute Stats"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "compute_stats",
                });

        let finalize_stats =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("LayerNorm Finalize Stats"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "finalize_stats",
                });

        let normalize =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("LayerNorm Normalize"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "normalize",
                });

        // Execute three-pass algorithm
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LayerNorm Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("LayerNorm Compute Stats"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&compute_stats);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("LayerNorm Finalize Stats"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&finalize_stats);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1); // Single workgroup for final reduction
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("LayerNorm Normalize"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&normalize);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }

    /// Execute BatchNorm: Batch Normalization for CNN training
    ///
    /// Normalizes across batch dimension using running statistics.
    /// Common in CNNs for accelerating training convergence.
    ///
    /// Deep Debt: Inference mode with pre-computed statistics (runtime configurable).
    pub async fn execute_batchnorm(
        &self,
        input: &[f32],
        batch_size: usize,
        channels: usize,
        spatial_size: usize,
        config: BatchNormConfig,
    ) -> Result<Vec<f32>> {
        let total_size = batch_size * channels * spatial_size;

        anyhow::ensure!(
            input.len() == total_size,
            "BatchNorm: input size must equal batch_size * channels * spatial_size"
        );
        anyhow::ensure!(
            config.gamma.len() == channels,
            "BatchNorm: gamma size must equal channels"
        );
        anyhow::ensure!(
            config.beta.len() == channels,
            "BatchNorm: beta size must equal channels"
        );
        anyhow::ensure!(
            config.running_mean.len() == channels,
            "BatchNorm: running_mean size must equal channels"
        );
        anyhow::ensure!(
            config.running_var.len() == channels,
            "BatchNorm: running_var size must equal channels"
        );

        let shader_source = include_str!("../shaders/batchnorm.wgsl");

        // Create input buffers
        let input_buffer = self.create_input_buffer(input, "BatchNorm Input");
        let gamma_buffer = self.create_input_buffer(&config.gamma, "BatchNorm Gamma");
        let beta_buffer = self.create_input_buffer(&config.beta, "BatchNorm Beta");
        let mean_buffer = self.create_input_buffer(&config.running_mean, "BatchNorm Mean");
        let var_buffer = self.create_input_buffer(&config.running_var, "BatchNorm Var");
        let output_buffer = self.create_output_buffer(total_size, "BatchNorm Output");
        let staging_buffer = self.create_staging_buffer(total_size, "BatchNorm Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct BatchNormParams {
            batch_size: u32,
            channels: u32,
            spatial_size: u32,
            epsilon: f32,
            training: u32,
            _padding: [u32; 3],
        }

        let params = BatchNormParams {
            batch_size: batch_size as u32,
            channels: channels as u32,
            spatial_size: spatial_size as u32,
            epsilon: config.epsilon,
            training: 0, // Inference mode (Deep Debt: configurable!)
            _padding: [0; 3],
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("BatchNorm Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        // Complex bind group with 7 bindings
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("BatchNorm Layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
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
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BatchNorm Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gamma_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: beta_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: mean_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: var_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline =
            self.create_simple_pipeline(shader_source, "BatchNorm", &bind_group_layout);
        let workgroups = self.calculate_workgroups(total_size, 256);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "BatchNorm");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (total_size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, total_size).await
    }

    /// Execute GroupNorm: Group Normalization
    ///
    /// Divides channels into groups and normalizes within each group.
    /// More stable than BatchNorm for small batch sizes.
    ///
    /// Deep Debt: Group count and parameters determined at runtime.
    pub async fn execute_groupnorm(
        &self,
        input: &[f32],
        batch_size: usize,
        channels: usize,
        spatial_size: usize,
        config: GroupNormConfig,
    ) -> Result<Vec<f32>> {
        let total_size = batch_size * channels * spatial_size;

        anyhow::ensure!(
            input.len() == total_size,
            "GroupNorm: input size must equal batch_size * channels * spatial_size"
        );
        anyhow::ensure!(
            channels % config.num_groups == 0,
            "GroupNorm: channels must be divisible by num_groups"
        );
        anyhow::ensure!(
            config.gamma.len() == channels,
            "GroupNorm: gamma size must equal channels"
        );
        anyhow::ensure!(
            config.beta.len() == channels,
            "GroupNorm: beta size must equal channels"
        );

        let channels_per_group = channels / config.num_groups;
        let total_groups = batch_size * config.num_groups;

        let shader_source = include_str!("../shaders/groupnorm.wgsl");

        // Create buffers
        let input_buffer = self.create_input_buffer(input, "GroupNorm Input");
        let gamma_buffer = self.create_input_buffer(&config.gamma, "GroupNorm Gamma");
        let beta_buffer = self.create_input_buffer(&config.beta, "GroupNorm Beta");
        let output_buffer = self.create_output_buffer(total_size, "GroupNorm Output");
        let staging_buffer = self.create_staging_buffer(total_size, "GroupNorm Staging");

        // Statistics buffer: 2 values (mean, variance) per group
        let stats_size = total_groups * 2;
        let stats_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GroupNorm Stats"),
            size: (stats_size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct GroupNormParams {
            batch_size: u32,
            channels: u32,
            spatial_size: u32,
            num_groups: u32,
            channels_per_group: u32,
            epsilon: f32,
            _padding: [u32; 2],
        }

        let params = GroupNormParams {
            batch_size: batch_size as u32,
            channels: channels as u32,
            spatial_size: spatial_size as u32,
            num_groups: config.num_groups as u32,
            channels_per_group: channels_per_group as u32,
            epsilon: config.epsilon,
            _padding: [0; 2],
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("GroupNorm Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        // Complex bind group with 6 bindings
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("GroupNorm Layout"),
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
            label: Some("GroupNorm Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gamma_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: beta_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: stats_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline =
            self.create_simple_pipeline(shader_source, "GroupNorm", &bind_group_layout);
        let workgroups = self.calculate_workgroups(total_size, 256);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "GroupNorm");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (total_size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, total_size).await
    }

    /// Execute Instance Normalization
    ///
    /// Normalizes each instance (batch sample) independently across spatial dimensions.
    /// Computes mean and variance over (height, width) for each (batch, channel) pair.
    ///
    /// Used in: Style transfer, GANs, real-time image generation.
    /// Benefits: No batch dependency, works well for style/texture tasks.
    ///
    /// Deep Debt: All dimensions determined at runtime, learnable parameters.
    pub async fn execute_instance_norm(
        &self,
        input: &[f32],
        batch: usize,
        channels: usize,
        spatial_size: usize, // height * width
        config: InstanceNormConfig,
    ) -> Result<Vec<f32>> {
        let total_size = batch * channels * spatial_size;
        anyhow::ensure!(
            input.len() == total_size,
            "InstanceNorm: input size must match batch * channels * spatial_size"
        );
        anyhow::ensure!(
            config.gamma.len() == channels,
            "InstanceNorm: gamma size must match channels"
        );
        anyhow::ensure!(
            config.beta.len() == channels,
            "InstanceNorm: beta size must match channels"
        );

        let shader_source = include_str!("../shaders/instancenorm.wgsl");

        let input_buffer = self.create_input_buffer(input, "InstanceNorm Input");
        let gamma_buffer = self.create_input_buffer(&config.gamma, "InstanceNorm Gamma");
        let beta_buffer = self.create_input_buffer(&config.beta, "InstanceNorm Beta");
        let output_buffer = self.create_output_buffer(total_size, "InstanceNorm Output");
        let staging_buffer = self.create_staging_buffer(total_size, "InstanceNorm Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct InstanceNormParams {
            batch: u32,
            channels: u32,
            spatial_size: u32,
            epsilon: f32,
        }

        let params = InstanceNormParams {
            batch: batch as u32,
            channels: channels as u32,
            spatial_size: spatial_size as u32,
            epsilon: config.epsilon,
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("InstanceNorm Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("InstanceNorm Layout"),
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
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("InstanceNorm Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gamma_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: beta_buffer.as_entire_binding(),
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

        let pipeline =
            self.create_simple_pipeline(shader_source, "InstanceNorm", &bind_group_layout);

        let num_instances = batch * channels;
        let workgroups = self.calculate_workgroups(num_instances, 256);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "InstanceNorm");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (total_size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, total_size).await
    }

    /// Execute RMS Normalization
    ///
    /// Simpler alternative to LayerNorm used in modern transformers.
    /// RMSNorm(x) = x / sqrt(mean(x²) + epsilon) * gamma
    ///
    /// No mean subtraction, only RMS scaling - faster and simpler than LayerNorm.
    /// Used in: LLaMA, GPT-NeoX, T5, modern large language models.
    ///
    /// Deep Debt: Runtime dimensions, learnable scale parameters.
    pub async fn execute_rms_norm(
        &self,
        input: &[f32],
        batch_size: usize,
        feature_size: usize,
        config: RmsNormConfig,
    ) -> Result<Vec<f32>> {
        let total_size = batch_size * feature_size;
        anyhow::ensure!(
            input.len() == total_size,
            "RMSNorm: input size must match batch_size * feature_size"
        );
        anyhow::ensure!(
            config.gamma.len() == feature_size,
            "RMSNorm: gamma size must match feature_size"
        );

        let shader_source = include_str!("../shaders/rmsnorm.wgsl");

        let input_buffer = self.create_input_buffer(input, "RMSNorm Input");
        let gamma_buffer = self.create_input_buffer(&config.gamma, "RMSNorm Gamma");
        let output_buffer = self.create_output_buffer(total_size, "RMSNorm Output");
        let staging_buffer = self.create_staging_buffer(total_size, "RMSNorm Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct RmsNormParams {
            batch_size: u32,
            feature_size: u32,
            epsilon: f32,
            _padding: u32,
        }

        let params = RmsNormParams {
            batch_size: batch_size as u32,
            feature_size: feature_size as u32,
            epsilon: config.epsilon,
            _padding: 0,
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("RMSNorm Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("RMSNorm Layout"),
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
            label: Some("RMSNorm Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gamma_buffer.as_entire_binding(),
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

        let pipeline = self.create_simple_pipeline(shader_source, "RMSNorm", &bind_group_layout);
        let workgroups = self.calculate_workgroups(batch_size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "RMSNorm");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (total_size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, total_size).await
    }
}
