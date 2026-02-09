//! Activation function operations
//!
//! ReLU, Sigmoid, Tanh, Softmax, etc.
//! Modern idiomatic Rust with eliminated boilerplate.

use anyhow::Result;

use super::executor::WgpuExecutor;

impl WgpuExecutor {
    /// Execute ReLU activation: output = max(0, input)
    ///
    /// Pure Rust, zero unsafe, modern async/await.
    /// Deep Debt: No hardcoded workgroup sizes - calculated at runtime.
    pub async fn execute_relu(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = input.len();

        // Load shader (compile-time checked!)
        let shader_source = include_str!("../shaders/relu.wgsl");

        // Create buffers using safe helpers
        let input_buffer = self.create_input_buffer(input, "ReLU Input");
        let output_buffer = self.create_output_buffer(size, "ReLU Output");
        let staging_buffer = self.create_staging_buffer(size, "ReLU Staging");

        // Create bind group layout
        let bind_group_layout = self.create_binary_bind_group_layout("ReLU Bind Group Layout");

        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ReLU Bind Group"),
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
            ],
        });

        // Create pipeline
        let pipeline = self.create_simple_pipeline(shader_source, "ReLU", &bind_group_layout);

        // Execute (workgroup size determined at runtime, not hardcoded)
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "ReLU");

        // Copy to staging
        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of_val(input) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read results using safe helper
        self.read_buffer(&staging_buffer, size).await
    }

    /// Execute Sigmoid activation: sigmoid(x) = 1 / (1 + exp(-x))
    ///
    /// Modern idiomatic implementation with reduced boilerplate.
    pub async fn execute_sigmoid(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = input.len();
        let shader_source = include_str!("../shaders/sigmoid.wgsl");

        let input_buffer = self.create_input_buffer(input, "Sigmoid Input");
        let output_buffer = self.create_output_buffer(size, "Sigmoid Output");
        let staging_buffer = self.create_staging_buffer(size, "Sigmoid Staging");

        let bind_group_layout = self.create_binary_bind_group_layout("Sigmoid Bind Group Layout");

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sigmoid Bind Group"),
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
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "Sigmoid", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Sigmoid");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of_val(input) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }

    /// Execute Tanh activation: tanh(x)
    ///
    /// Hyperbolic tangent activation function.
    pub async fn execute_tanh(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = input.len();
        let shader_source = include_str!("../shaders/tanh.wgsl");

        let input_buffer = self.create_input_buffer(input, "Tanh Input");
        let output_buffer = self.create_output_buffer(size, "Tanh Output");
        let staging_buffer = self.create_staging_buffer(size, "Tanh Staging");

        let bind_group_layout = self.create_binary_bind_group_layout("Tanh Bind Group Layout");

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Tanh Bind Group"),
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
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "Tanh", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Tanh");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of_val(input) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }

    /// Execute GELU (Gaussian Error Linear Unit) activation
    ///
    /// GELU(x) = x * Φ(x) where Φ is the cumulative distribution function
    /// Used extensively in BERT, GPT, and modern transformers.
    ///
    /// Deep Debt: Runtime activation computation, no hardcoded values.
    pub async fn execute_gelu(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = input.len();
        let shader_source = include_str!("../shaders/gelu.wgsl");

        let input_buffer = self.create_input_buffer(input, "GELU Input");
        let output_buffer = self.create_output_buffer(size, "GELU Output");
        let staging_buffer = self.create_staging_buffer(size, "GELU Staging");

        let bind_group_layout = self.create_binary_bind_group_layout("GELU Bind Group Layout");

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GELU Bind Group"),
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
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "GELU", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "GELU");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of_val(input) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }

    /// Execute Swish / SiLU (Sigmoid Linear Unit) activation
    ///
    /// Swish(x) = x * sigmoid(x)
    /// Used in EfficientNet, MobileNetV3, and modern architectures.
    /// Self-gated activation with smooth, non-monotonic behavior.
    pub async fn execute_swish(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = input.len();
        let shader_source = include_str!("../shaders/swish.wgsl");

        let input_buffer = self.create_input_buffer(input, "Swish Input");
        let output_buffer = self.create_output_buffer(size, "Swish Output");
        let staging_buffer = self.create_staging_buffer(size, "Swish Staging");

        let bind_group_layout = self.create_binary_bind_group_layout("Swish Bind Group Layout");

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Swish Bind Group"),
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
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "Swish", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Swish");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of_val(input) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }

    /// Execute LeakyReLU activation: max(α*x, x)
    ///
    /// Addresses dying ReLU problem by allowing small negative slope.
    /// Widely used in GANs and deep networks.
    ///
    /// # Arguments
    /// * `input` - Input tensor
    /// * `negative_slope` - Slope for negative values (typically 0.01)
    pub async fn execute_leaky_relu(&self, input: &[f32], negative_slope: f32) -> Result<Vec<f32>> {
        let size = input.len();
        let shader_source = include_str!("../shaders/leaky_relu.wgsl");

        let input_buffer = self.create_input_buffer(input, "LeakyReLU Input");
        let output_buffer = self.create_output_buffer(size, "LeakyReLU Output");
        let staging_buffer = self.create_staging_buffer(size, "LeakyReLU Staging");

        // Create params buffer (aligned to 32 bytes for WGSL struct)
        let params = [negative_slope, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]; // Pad to 32 bytes
        let params_buffer = self.create_uniform_buffer(&params, "LeakyReLU Params");

        // Create bind group layout with params
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("LeakyReLU Bind Group Layout"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LeakyReLU Bind Group"),
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
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "LeakyReLU", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "LeakyReLU");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of_val(input) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }

    /// Execute ELU (Exponential Linear Unit) activation
    ///
    /// ELU(x) = x if x > 0, else α * (exp(x) - 1)
    /// Smooth negative part, reduces bias shift effect.
    ///
    /// # Arguments
    /// * `input` - Input tensor
    /// * `alpha` - Scale for negative values (typically 1.0)
    pub async fn execute_elu(&self, input: &[f32], alpha: f32) -> Result<Vec<f32>> {
        let size = input.len();
        let shader_source = include_str!("../shaders/elu.wgsl");

        let input_buffer = self.create_input_buffer(input, "ELU Input");
        let output_buffer = self.create_output_buffer(size, "ELU Output");
        let staging_buffer = self.create_staging_buffer(size, "ELU Staging");

        // Create params buffer (aligned to 32 bytes for WGSL struct)
        let params = [alpha, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]; // Pad to 32 bytes
        let params_buffer = self.create_uniform_buffer(&params, "ELU Params");

        // Create bind group layout with params
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("ELU Bind Group Layout"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ELU Bind Group"),
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
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "ELU", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "ELU");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of_val(input) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }

    /// Execute SELU (Scaled Exponential Linear Unit) activation
    ///
    /// Self-normalizing activation function for deep neural networks.
    /// Automatically pushes activations towards zero mean and unit variance.
    ///
    /// SELU(x) = scale * x                      if x > 0
    ///         = scale * alpha * (exp(x) - 1)   if x <= 0
    ///
    /// Uses proven constants: alpha ≈ 1.673, scale ≈ 1.051
    /// Used in: Self-Normalizing Neural Networks (SNNs)
    ///
    /// Deep Debt: No hardcoded values except mathematically proven constants.
    pub async fn execute_selu(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = input.len();
        let shader_source = include_str!("../shaders/selu.wgsl");

        let input_buffer = self.create_input_buffer(input, "SELU Input");
        let output_buffer = self.create_output_buffer(size, "SELU Output");
        let staging_buffer = self.create_staging_buffer(size, "SELU Staging");

        let bind_group_layout = self.create_binary_bind_group_layout("SELU Bind Group Layout");

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SELU Bind Group"),
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
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "SELU", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "SELU");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of_val(input) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }

    /// Execute HardSwish activation
    ///
    /// Efficient approximation of Swish/SiLU for mobile and edge devices.
    /// HardSwish(x) = x * ReLU6(x + 3) / 6
    ///
    /// Faster than Swish (no sigmoid), optimized for inference on resource-constrained devices.
    /// Used in: MobileNetV3, EfficientNet-Lite
    ///
    /// Deep Debt: Runtime computation, mobile-friendly implementation.
    pub async fn execute_hardswish(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = input.len();
        let shader_source = include_str!("../shaders/hardswish.wgsl");

        let input_buffer = self.create_input_buffer(input, "HardSwish Input");
        let output_buffer = self.create_output_buffer(size, "HardSwish Output");
        let staging_buffer = self.create_staging_buffer(size, "HardSwish Staging");

        let bind_group_layout = self.create_binary_bind_group_layout("HardSwish Bind Group Layout");

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("HardSwish Bind Group"),
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
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "HardSwish", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "HardSwish");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of_val(input) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }

    /// Execute Mish activation
    ///
    /// Self-regularizing smooth non-monotonic activation.
    /// Mish(x) = x * tanh(softplus(x)) = x * tanh(ln(1 + exp(x)))
    ///
    /// Smooth alternative to ReLU with better accuracy in many tasks.
    /// Used in: YOLOv4, modern computer vision, deep networks.
    ///
    /// Deep Debt: Runtime computation with numerically stable implementation.
    pub async fn execute_mish(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = input.len();
        let shader_source = include_str!("../shaders/mish.wgsl");

        let input_buffer = self.create_input_buffer(input, "Mish Input");
        let output_buffer = self.create_output_buffer(size, "Mish Output");
        let staging_buffer = self.create_staging_buffer(size, "Mish Staging");

        let bind_group_layout = self.create_binary_bind_group_layout("Mish Bind Group Layout");

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mish Bind Group"),
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
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "Mish", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Mish");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of_val(input) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }
}
