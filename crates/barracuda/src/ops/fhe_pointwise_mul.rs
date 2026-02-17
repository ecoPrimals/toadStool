use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// Point-wise multiplication of two polynomials in NTT domain
///
/// Given two polynomials A and B in NTT domain (already transformed),
/// computes C = A ⊙ B (element-wise product).
///
/// This is the core operation in fast polynomial multiplication:
///   poly_mul(a, b) = INTT(pointwise_mul(NTT(a), NTT(b)))
///
/// Complexity: O(N) - much faster than O(N²) convolution!
///
/// # Example
/// ```ignore
/// use barracuda::ops::{FheNtt, FhePointwiseMul, FheIntt};
///
/// // Fast polynomial multiplication
/// let ntt_a = FheNtt::new(poly_a, degree, modulus, root)?;
/// let ntt_b = FheNtt::new(poly_b, degree, modulus, root)?;
///
/// let a_ntt = ntt_a.execute().await?;
/// let b_ntt = ntt_b.execute().await?;
///
/// let pointwise_mul = FhePointwiseMul::new(a_ntt, b_ntt, degree, modulus)?;
/// let c_ntt = pointwise_mul.execute().await?;
///
/// let intt = FheIntt::new(c_ntt, degree, modulus, inv_root)?;
/// let c = intt.execute().await?;  // c = a * b (polynomial multiplication!)
/// ```
#[allow(dead_code)]
pub struct FhePointwiseMul {
    input_a: Tensor,
    input_b: Tensor,
    degree: u32,
    modulus: u64,
    barrett_mu: u64,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

/// Parameters passed to GPU shader
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct PointwiseMulParams {
    degree: u32,
    modulus_low: u32,
    modulus_high: u32,
    barrett_mu_low: u32,
    barrett_mu_high: u32,
    _padding: [u32; 3], // Align to 16 bytes
}

impl FhePointwiseMul {
    /// Create a new point-wise multiplication operation
    ///
    /// # Arguments
    /// * `input_a` - First polynomial in NTT domain (N × 2 u32 values)
    /// * `input_b` - Second polynomial in NTT domain (N × 2 u32 values)
    /// * `degree` - Polynomial degree (N), must be power of 2
    /// * `modulus` - FHE modulus q (64-bit prime)
    pub fn new(input_a: Tensor, input_b: Tensor, degree: u32, modulus: u64) -> Result<Self> {
        // Validate inputs
        if !degree.is_power_of_two() {
            return Err(BarracudaError::Device(format!(
                "Degree must be power of 2, got {}",
                degree
            )));
        }

        if !(4..=65536).contains(&degree) {
            return Err(BarracudaError::Device(format!(
                "Degree must be in range [4, 65536], got {}",
                degree
            )));
        }

        let expected_len = (degree * 2) as usize; // 2 u32 per coefficient
        if input_a.len() != expected_len {
            return Err(BarracudaError::Device(format!(
                "Input A has wrong length: expected {}, got {}",
                expected_len,
                input_a.len()
            )));
        }
        if input_b.len() != expected_len {
            return Err(BarracudaError::Device(format!(
                "Input B has wrong length: expected {}, got {}",
                expected_len,
                input_b.len()
            )));
        }

        // Ensure both tensors are on same device
        if !std::ptr::eq(input_a.device().as_ref(), input_b.device().as_ref()) {
            return Err(BarracudaError::Device(
                "input_a and input_b must be on the same device".to_string(),
            ));
        }

        // Compute Barrett reduction constant: μ = ⌊2^128 / q⌋
        // For 64-bit approximation: μ ≈ u64::MAX / q
        let barrett_mu = if modulus > 0 { u64::MAX / modulus } else { 0 };

        // Get device from input tensors
        let device = input_a.device();

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("FHE Point-wise Multiply Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("fhe_pointwise_mul.wgsl").into()),
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("FHE Point-wise Multiply Bind Group Layout"),
                    entries: &[
                        // Input A
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
                        // Input B
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
                        // Output
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
                        // Parameters
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("FHE Point-wise Multiply Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("FHE Point-wise Multiply Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
            });

        Ok(Self {
            input_a,
            input_b,
            degree,
            modulus,
            barrett_mu,
            pipeline,
            bind_group_layout,
        })
    }

    /// Execute point-wise multiplication on GPU
    ///
    /// Returns: C = A ⊙ B (element-wise product in NTT domain)
    pub fn execute(&self) -> Result<Tensor> {
        let device = self.input_a.device();

        // Create output buffer
        let result_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Point-wise Multiply Output"),
            size: (self.degree as u64 * 2 * std::mem::size_of::<u32>() as u64),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create parameter buffer
        let params = PointwiseMulParams {
            degree: self.degree,
            modulus_low: (self.modulus & 0xFFFFFFFF) as u32,
            modulus_high: (self.modulus >> 32) as u32,
            barrett_mu_low: (self.barrett_mu & 0xFFFFFFFF) as u32,
            barrett_mu_high: (self.barrett_mu >> 32) as u32,
            _padding: [0; 3],
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Point-wise Multiply Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Point-wise Multiply Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input_a.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.input_b.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: result_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create command encoder
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Point-wise Multiply Command Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Point-wise Multiply Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch: one thread per coefficient
            let workgroup_size = 256;
            let num_workgroups = self.degree.div_ceil(workgroup_size);
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        // Submit command buffer
        device.queue.submit(Some(encoder.finish()));

        // Return tensor (data stays on GPU)
        Ok(Tensor::from_buffer(
            result_buffer,
            vec![self.degree as usize * 2],
            device.clone(),
        ))
    }
}

// Tests disabled - requires integration testing framework
// Will be tested via fhe_fast_poly_mul integration tests
