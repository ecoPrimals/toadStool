//! Pure Rust GPU executor using wgpu
//! 
//! Modern, idiomatic Rust with WebGPU standard.
//! Zero FFI, zero unsafe code in our implementation!

use anyhow::{Context, Result};
use wgpu::util::DeviceExt;

/// Binary operations for elementwise operations
#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add = 0,
    Sub = 1,
    Mul = 2,
    Div = 3,
}

/// Reduction operations
#[derive(Debug, Clone, Copy)]
pub enum ReduceOp {
    Sum = 0,
    Max = 1,
    Min = 2,
    Mean = 3,
}

/// Map operations
#[derive(Debug, Clone, Copy)]
pub enum MapOp {
    Square = 0,
    Sqrt = 1,
    Abs = 2,
    Negate = 3,
    Reciprocal = 4,
}

/// Pure Rust GPU executor using wgpu (WebGPU)
/// 
/// No FFI, no unsafe code - just modern idiomatic Rust!
pub struct WgpuExecutor {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_info: wgpu::AdapterInfo,
}

impl WgpuExecutor {
    /// Create a new wgpu executor
    /// 
    /// This is pure Rust - no FFI, no unsafe!
    pub async fn new() -> Result<Self> {
        // Create instance (pure Rust)
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        // Request adapter (pure Rust)
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .context("Failed to find GPU adapter")?;
        
        let adapter_info = adapter.get_info();
        
        // Request device (pure Rust)
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("ToadStool GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .context("Failed to create GPU device")?;
        
        Ok(Self {
            device,
            queue,
            adapter_info,
        })
    }
    
    /// Get GPU information
    pub fn gpu_info(&self) -> String {
        format!(
            "{} {} ({})",
            self.adapter_info.vendor,
            self.adapter_info.name,
            self.adapter_info.backend.to_str()
        )
    }
    
    /// Execute ReLU activation: output = max(0, input)
    /// 
    /// Pure Rust implementation - no FFI!
    pub async fn execute_relu(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = input.len();
        
        // Load shader (pure Rust, compile-time checked!)
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ReLU Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/relu.wgsl").into()),
        });
        
        // Create buffers (pure Rust, no unsafe!)
        let input_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input Buffer"),
            contents: bytemuck::cast_slice(input),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        // Create bind group layout
        let bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ReLU Bind Group Layout"),
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
            ],
        });
        
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
        
        // Create pipeline layout
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ReLU Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create compute pipeline
        let compute_pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ReLU Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        // Create staging buffer for reading results
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Execute (pure Rust!)
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ReLU Encoder"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ReLU Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            // Dispatch workgroups (workgroup_size = 256)
            let workgroups = (size as u32 + 255) / 256;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        
        // Copy to staging buffer
        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );
        
        self.queue.submit(Some(encoder.finish()));
        
        // Read results (pure Rust, async)
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        
        self.device.poll(wgpu::Maintain::Wait);
        receiver.receive().await.context("Failed to map buffer")??;
        
        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        
        drop(data);
        staging_buffer.unmap();
        
        Ok(result)
    }
    
    /// Execute matrix multiplication: C = A * B
    /// 
    /// A: (M, K), B: (K, N), C: (M, N)
    /// Pure Rust, no FFI!
    pub async fn execute_matmul(
        &self,
        a: &[f32],
        b: &[f32],
        m: u32,
        k: u32,
        n: u32,
    ) -> Result<Vec<f32>> {
        // Load shader
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MatMul Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/matmul.wgsl").into()),
        });
        
        // Create buffers
        let a_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Matrix A"),
            contents: bytemuck::cast_slice(a),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let b_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Matrix B"),
            contents: bytemuck::cast_slice(b),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let c_size = (m * n) as usize * std::mem::size_of::<f32>();
        let c_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Matrix C"),
            size: c_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        // Create params buffer
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct MatmulParams {
            m: u32,
            k: u32,
            n: u32,
            _padding: u32,
        }
        
        let params = MatmulParams {
            m,
            k,
            n,
            _padding: 0,
        };
        
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        // Create bind group layout
        let bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MatMul Layout"),
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
        
        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MatMul Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: a_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: b_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: c_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Create pipeline
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MatMul Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("MatMul Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        // Create staging buffer
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: c_size as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Execute
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((n + 15) / 16, (m + 15) / 16, 1);
        }
        encoder.copy_buffer_to_buffer(&c_buffer, 0, &staging_buffer, 0, c_size as u64);
        self.queue.submit(Some(encoder.finish()));
        
        // Read results
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        
        self.device.poll(wgpu::Maintain::Wait);
        receiver.receive().await.context("Failed to map buffer")??;
        
        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        
        drop(data);
        staging_buffer.unmap();
        
        Ok(result)
    }
    
    /// Execute vector addition: C = A * alpha + B
    /// 
    /// CUDA equivalent: `cublas::axpy`
    /// Use cases: Gradient updates, residual connections
    pub async fn execute_vector_add(
        &self,
        a: &[f32],
        b: &[f32],
        alpha: f32,
    ) -> Result<Vec<f32>> {
        let size = a.len();
        anyhow::ensure!(b.len() == size, "Vector sizes must match");
        
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VectorAdd Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/vectoradd.wgsl").into()),
        });
        
        let a_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vector A"),
            contents: bytemuck::cast_slice(a),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let b_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vector B"),
            contents: bytemuck::cast_slice(b),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct VectorAddParams {
            size: u32,
            alpha: f32,
        }
        
        let params = VectorAddParams {
            size: size as u32,
            alpha,
        };
        
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("VectorAdd Layout"),
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
            label: Some("VectorAdd Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: a_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: b_buffer.as_entire_binding(),
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
        
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("VectorAdd Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("VectorAdd Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((size as u32 + 255) / 256, 1, 1);
        }
        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );
        self.queue.submit(Some(encoder.finish()));
        
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        
        self.device.poll(wgpu::Maintain::Wait);
        receiver.receive().await.context("Failed to map buffer")??;
        
        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        
        drop(data);
        staging_buffer.unmap();
        
        Ok(result)
    }
    
    /// Execute elementwise binary operation: C = A op B
    /// 
    /// CUDA equivalent: `thrust::transform` (binary)
    /// Use cases: Residual connections, loss computation
    pub async fn execute_elementwise_binary(
        &self,
        a: &[f32],
        b: &[f32],
        operation: BinaryOp,
    ) -> Result<Vec<f32>> {
        let size = a.len();
        anyhow::ensure!(b.len() == size, "Vector sizes must match");
        
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ElementwiseBinary Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/elementwise_binary.wgsl").into()),
        });
        
        let a_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input A"),
            contents: bytemuck::cast_slice(a),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let b_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input B"),
            contents: bytemuck::cast_slice(b),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct BinaryParams {
            size: u32,
            operation: u32,
        }
        
        let params = BinaryParams {
            size: size as u32,
            operation: operation as u32,
        };
        
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Binary Layout"),
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
            label: Some("Binary Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: a_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: b_buffer.as_entire_binding(),
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
        
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Binary Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Binary Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((size as u32 + 255) / 256, 1, 1);
        }
        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );
        self.queue.submit(Some(encoder.finish()));
        
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        
        self.device.poll(wgpu::Maintain::Wait);
        receiver.receive().await.context("Failed to map buffer")??;
        
        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        
        drop(data);
        staging_buffer.unmap();
        
        Ok(result)
    }
    
    /// Execute reduce operation: compute sum/max/min/mean
    /// 
    /// CUDA equivalent: `thrust::reduce`, `cub::DeviceReduce`
    /// Use cases: Loss computation, gradient accumulation
    pub async fn execute_reduce(
        &self,
        input: &[f32],
        operation: ReduceOp,
    ) -> Result<f32> {
        let size = input.len();
        
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Reduce Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/reduce.wgsl").into()),
        });
        
        let input_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input"),
            contents: bytemuck::cast_slice(input),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        // Calculate number of workgroups
        let workgroups = ((size as u32 + 255) / 256).max(1);
        
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Partial Results"),
            size: (workgroups as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct ReduceParams {
            size: u32,
            operation: u32,
        }
        
        let params = ReduceParams {
            size: size as u32,
            operation: operation as u32,
        };
        
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Reduce Layout"),
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
            label: Some("Reduce Bind Group"),
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
        
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Reduce Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Reduce Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: (workgroups as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (workgroups as usize * std::mem::size_of::<f32>()) as u64,
        );
        self.queue.submit(Some(encoder.finish()));
        
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        
        self.device.poll(wgpu::Maintain::Wait);
        receiver.receive().await.context("Failed to map buffer")??;
        
        let data = buffer_slice.get_mapped_range();
        let partial_results: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        
        drop(data);
        staging_buffer.unmap();
        
        // Final reduction on CPU (small array)
        let final_result = match operation {
            ReduceOp::Sum | ReduceOp::Mean => partial_results.iter().sum::<f32>(),
            ReduceOp::Max => partial_results.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
            ReduceOp::Min => partial_results.iter().cloned().fold(f32::INFINITY, f32::min),
        };
        
        let result = if matches!(operation, ReduceOp::Mean) {
            final_result / size as f32
        } else {
            final_result
        };
        
        Ok(result)
    }
    
    /// Execute dot product: compute A · B
    /// 
    /// CUDA equivalent: `cublas::dot`
    /// Use cases: Similarity, attention scores
    pub async fn execute_dot_product(
        &self,
        a: &[f32],
        b: &[f32],
    ) -> Result<f32> {
        let size = a.len();
        anyhow::ensure!(b.len() == size, "Vector sizes must match");
        
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("DotProduct Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/dotproduct.wgsl").into()),
        });
        
        let a_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vector A"),
            contents: bytemuck::cast_slice(a),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let b_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vector B"),
            contents: bytemuck::cast_slice(b),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let workgroups = ((size as u32 + 255) / 256).max(1);
        
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Partial Sums"),
            size: (workgroups as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct DotProductParams {
            size: u32,
        }
        
        let params = DotProductParams {
            size: size as u32,
        };
        
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("DotProduct Layout"),
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
            label: Some("DotProduct Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: a_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: b_buffer.as_entire_binding(),
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
        
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("DotProduct Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("DotProduct Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: (workgroups as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (workgroups as usize * std::mem::size_of::<f32>()) as u64,
        );
        self.queue.submit(Some(encoder.finish()));
        
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        
        self.device.poll(wgpu::Maintain::Wait);
        receiver.receive().await.context("Failed to map buffer")??;
        
        let data = buffer_slice.get_mapped_range();
        let partial_sums: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        
        drop(data);
        staging_buffer.unmap();
        
        // Final sum on CPU
        let result = partial_sums.iter().sum::<f32>();
        
        Ok(result)
    }
    
    /// Execute transpose: (rows, cols) -> (cols, rows)
    /// 
    /// CUDA equivalent: `cublas::geam`
    /// Use cases: Matrix operations, layout transforms, attention
    pub async fn execute_transpose(
        &self,
        input: &[f32],
        rows: u32,
        cols: u32,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            input.len() == (rows * cols) as usize,
            "Input size must match rows * cols"
        );
        
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Transpose Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/transpose.wgsl").into()),
        });
        
        let input_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input"),
            contents: bytemuck::cast_slice(input),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let output_size = (rows * cols) as usize * std::mem::size_of::<f32>();
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: output_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct TransposeParams {
            rows: u32,
            cols: u32,
        }
        
        let params = TransposeParams { rows, cols };
        
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Transpose Layout"),
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
            label: Some("Transpose Bind Group"),
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
        
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Transpose Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Transpose Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: output_size as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // 16x16 workgroup size, dispatch enough workgroups to cover matrix
            pass.dispatch_workgroups((cols + 15) / 16, (rows + 15) / 16, 1);
        }
        encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, output_size as u64);
        self.queue.submit(Some(encoder.finish()));
        
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        
        self.device.poll(wgpu::Maintain::Wait);
        receiver.receive().await.context("Failed to map buffer")??;
        
        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        
        drop(data);
        staging_buffer.unmap();
        
        Ok(result)
    }
    
    /// Execute map operation: apply function to each element
    /// 
    /// CUDA equivalent: `thrust::transform`
    /// Use cases: Element-wise transforms, preprocessing
    pub async fn execute_map(
        &self,
        input: &[f32],
        operation: MapOp,
    ) -> Result<Vec<f32>> {
        let size = input.len();
        
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Map Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/map.wgsl").into()),
        });
        
        let input_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input"),
            contents: bytemuck::cast_slice(input),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct MapParams {
            size: u32,
            operation: u32,
        }
        
        let params = MapParams {
            size: size as u32,
            operation: operation as u32,
        };
        
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let bind_group_layout = self.create_simple_bind_group_layout(3);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Map Bind Group"),
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
        
        self.execute_simple_compute(&shader, &bind_group_layout, &bind_group, size, &output_buffer).await
    }
    
    /// Execute sigmoid activation: sigmoid(x) = 1 / (1 + exp(-x))
    /// 
    /// CUDA equivalent: `cudnn::Activation(SIGMOID)`
    /// Use cases: Binary classification, gate activations
    pub async fn execute_sigmoid(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = input.len();
        
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sigmoid Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sigmoid.wgsl").into()),
        });
        
        let input_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input"),
            contents: bytemuck::cast_slice(input),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct SigmoidParams {
            size: u32,
        }
        
        let params = SigmoidParams { size: size as u32 };
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let bind_group_layout = self.create_simple_bind_group_layout(3);
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        
        self.execute_simple_compute(&shader, &bind_group_layout, &bind_group, size, &output_buffer).await
    }
    
    /// Execute tanh activation: tanh(x)
    /// 
    /// CUDA equivalent: `cudnn::Activation(TANH)`
    /// Use cases: Activation function, output normalization
    pub async fn execute_tanh(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = input.len();
        
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Tanh Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/tanh.wgsl").into()),
        });
        
        let input_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input"),
            contents: bytemuck::cast_slice(input),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct TanhParams {
            size: u32,
        }
        
        let params = TanhParams { size: size as u32 };
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let bind_group_layout = self.create_simple_bind_group_layout(3);
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        
        self.execute_simple_compute(&shader, &bind_group_layout, &bind_group, size, &output_buffer).await
    }
    
    /// Execute gather: indirect read with indices
    /// 
    /// CUDA equivalent: `thrust::gather`
    /// Use cases: Embedding lookup, sparse access, graph neural networks
    pub async fn execute_gather(
        &self,
        source: &[f32],
        indices: &[u32],
    ) -> Result<Vec<f32>> {
        let num_elements = indices.len();
        let source_size = source.len();
        
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Gather Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/gather.wgsl").into()),
        });
        
        let source_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Source"),
            contents: bytemuck::cast_slice(source),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let indices_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Indices"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: (num_elements * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct GatherParams {
            num_elements: u32,
            source_size: u32,
        }
        
        let params = GatherParams {
            num_elements: num_elements as u32,
            source_size: source_size as u32,
        };
        
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Gather Layout"),
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
            label: Some("Gather Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: source_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: indices_buffer.as_entire_binding(),
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
        
        self.execute_simple_compute(&shader, &bind_group_layout, &bind_group, num_elements, &output_buffer).await
    }
    
    /// Execute dropout: regularization with random masking
    /// 
    /// CUDA equivalent: `cudnn::Dropout`
    /// Use cases: Regularization, preventing overfitting
    pub async fn execute_dropout(
        &self,
        input: &[f32],
        dropout_prob: f32,
        training: bool,
        seed: Option<u64>,
    ) -> Result<Vec<f32>> {
        let size = input.len();
        
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Dropout Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/dropout.wgsl").into()),
        });
        
        let input_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input"),
            contents: bytemuck::cast_slice(input),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let mask_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mask"),
            size: (size * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct DropoutParams {
            size: u32,
            dropout_prob: f32,
            training: u32,
            seed: u32,
        }
        
        let params = DropoutParams {
            size: size as u32,
            dropout_prob,
            training: if training { 1 } else { 0 },
            seed: seed.unwrap_or(12345) as u32,
        };
        
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Dropout Layout"),
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
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Dropout Bind Group"),
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
                    resource: mask_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        
        self.execute_simple_compute(&shader, &bind_group_layout, &bind_group, size, &output_buffer).await
    }
    
    // Helper methods to reduce boilerplate
    
    fn create_simple_bind_group_layout(&self, num_bindings: u32) -> wgpu::BindGroupLayout {
        let mut entries = Vec::new();
        
        for i in 0..num_bindings {
            let ty = if i < num_bindings - 1 {
                // First N-1 bindings are storage buffers
                wgpu::BindingType::Buffer {
                    ty: if i == 0 {
                        wgpu::BufferBindingType::Storage { read_only: true }
                    } else {
                        wgpu::BufferBindingType::Storage { read_only: false }
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }
            } else {
                // Last binding is uniform (params)
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }
            };
            
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: i,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty,
                count: None,
            });
        }
        
        self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Simple Layout"),
            entries: &entries,
        })
    }
    
    async fn execute_simple_compute(
        &self,
        shader: &wgpu::ShaderModule,
        bind_group_layout: &wgpu::BindGroupLayout,
        bind_group: &wgpu::BindGroup,
        size: usize,
        output_buffer: &wgpu::Buffer,
    ) -> Result<Vec<f32>> {
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Simple Pipeline Layout"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Simple Pipeline"),
            layout: Some(&pipeline_layout),
            module: shader,
            entry_point: "main",
        });
        
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups((size as u32 + 255) / 256, 1, 1);
        }
        encoder.copy_buffer_to_buffer(
            output_buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );
        self.queue.submit(Some(encoder.finish()));
        
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        
        self.device.poll(wgpu::Maintain::Wait);
        receiver.receive().await.context("Failed to map buffer")??;
        
        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        
        drop(data);
        staging_buffer.unmap();
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_wgpu_relu() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let output = executor.execute_relu(&input).await.unwrap();
        
        let expected = vec![0.0, 0.0, 0.0, 1.0, 2.0];
        for (out, exp) in output.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-5);
        }
    }
    
    #[tokio::test]
    async fn test_wgpu_matmul() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        // 2x3 * 3x2 = 2x2
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        
        let c = executor.execute_matmul(&a, &b, 2, 3, 2).await.unwrap();
        
        // Expected: [[22, 28], [49, 64]]
        let expected = vec![22.0, 28.0, 49.0, 64.0];
        for (out, exp) in c.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-3);
        }
    }
    
    #[tokio::test]
    async fn test_vector_add() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let alpha = 2.0;
        
        let result = executor.execute_vector_add(&a, &b, alpha).await.unwrap();
        
        // Expected: a * alpha + b = [2, 4, 6, 8, 10] + [10, 20, 30, 40, 50] = [12, 24, 36, 48, 60]
        let expected = vec![12.0, 24.0, 36.0, 48.0, 60.0];
        for (out, exp) in result.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-5);
        }
    }
    
    #[tokio::test]
    async fn test_elementwise_add() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        
        let result = executor.execute_elementwise_binary(&a, &b, BinaryOp::Add).await.unwrap();
        
        let expected = vec![11.0, 22.0, 33.0, 44.0, 55.0];
        for (out, exp) in result.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-5);
        }
    }
    
    #[tokio::test]
    async fn test_elementwise_mul() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![2.0, 3.0, 4.0, 5.0, 6.0];
        
        let result = executor.execute_elementwise_binary(&a, &b, BinaryOp::Mul).await.unwrap();
        
        let expected = vec![2.0, 6.0, 12.0, 20.0, 30.0];
        for (out, exp) in result.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-5);
        }
    }
    
    #[tokio::test]
    async fn test_reduce_sum() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = executor.execute_reduce(&input, ReduceOp::Sum).await.unwrap();
        
        let expected = 15.0;
        assert!((result - expected).abs() < 1e-5);
    }
    
    #[tokio::test]
    async fn test_reduce_max() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        let input = vec![1.0, 5.0, 3.0, 2.0, 4.0];
        let result = executor.execute_reduce(&input, ReduceOp::Max).await.unwrap();
        
        let expected = 5.0;
        assert!((result - expected).abs() < 1e-5);
    }
    
    #[tokio::test]
    async fn test_reduce_mean() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = executor.execute_reduce(&input, ReduceOp::Mean).await.unwrap();
        
        let expected = 3.0;
        assert!((result - expected).abs() < 1e-5);
    }
    
    #[tokio::test]
    async fn test_dot_product() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![2.0, 3.0, 4.0, 5.0, 6.0];
        
        let result = executor.execute_dot_product(&a, &b).await.unwrap();
        
        // 1*2 + 2*3 + 3*4 + 4*5 + 5*6 = 2 + 6 + 12 + 20 + 30 = 70
        let expected = 70.0;
        assert!((result - expected).abs() < 1e-5);
    }
    
    #[tokio::test]
    async fn test_transpose() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        // 2x3 matrix
        let input = vec![
            1.0, 2.0, 3.0,
            4.0, 5.0, 6.0,
        ];
        
        let result = executor.execute_transpose(&input, 2, 3).await.unwrap();
        
        // Expected: 3x2 matrix
        let expected = vec![
            1.0, 4.0,
            2.0, 5.0,
            3.0, 6.0,
        ];
        
        for (out, exp) in result.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-5);
        }
    }
    
    #[tokio::test]
    async fn test_map_square() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = executor.execute_map(&input, MapOp::Square).await.unwrap();
        
        let expected = vec![1.0, 4.0, 9.0, 16.0, 25.0];
        for (out, exp) in result.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-5);
        }
    }
    
    #[tokio::test]
    async fn test_sigmoid() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let result = executor.execute_sigmoid(&input).await.unwrap();
        
        // Expected: sigmoid(x) = 1 / (1 + exp(-x))
        let expected = vec![0.1192, 0.2689, 0.5, 0.7311, 0.8808];
        for (out, exp) in result.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-3);
        }
    }
    
    #[tokio::test]
    async fn test_tanh() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let result = executor.execute_tanh(&input).await.unwrap();
        
        // Expected: tanh values
        let expected = vec![-0.9640, -0.7616, 0.0, 0.7616, 0.9640];
        for (out, exp) in result.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-3);
        }
    }
    
    #[tokio::test]
    async fn test_gather() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        let source = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let indices = vec![0, 2, 4, 1, 3];
        
        let result = executor.execute_gather(&source, &indices).await.unwrap();
        
        let expected = vec![10.0, 30.0, 50.0, 20.0, 40.0];
        for (out, exp) in result.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-5);
        }
    }
    
    #[tokio::test]
    async fn test_dropout_inference() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = executor.execute_dropout(&input, 0.5, false, None).await.unwrap();
        
        // In inference mode, dropout should not modify input
        for (out, inp) in result.iter().zip(input.iter()) {
            assert!((out - inp).abs() < 1e-5);
        }
    }
    
    #[tokio::test]
    async fn test_dropout_training() {
        let executor = WgpuExecutor::new().await.unwrap();
        
        let input = vec![1.0; 1000];  // Large input for statistical testing
        let result = executor.execute_dropout(&input, 0.5, true, Some(42)).await.unwrap();
        
        // In training mode with 0.5 dropout, roughly half should be zeroed
        let zero_count = result.iter().filter(|&&x| x == 0.0).count();
        let non_zero_count = result.iter().filter(|&&x| x != 0.0).count();
        
        // Should be roughly 50/50 (allow 40-60% range for randomness)
        assert!(zero_count > 400 && zero_count < 600, "Zero count: {}", zero_count);
        assert!(non_zero_count > 400 && non_zero_count < 600);
        
        // Non-zero values should be scaled by 1/(1-p) = 2.0
        let avg_non_zero = result.iter().filter(|&&x| x != 0.0).sum::<f32>() / non_zero_count as f32;
        assert!((avg_non_zero - 2.0).abs() < 0.2, "Avg non-zero: {}", avg_non_zero);
    }
}

