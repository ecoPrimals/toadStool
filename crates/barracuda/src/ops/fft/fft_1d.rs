//! 1D Fast Fourier Transform Operation
//!
//! **Evolution**: Adapted from FheNtt (80% Rust structure reuse!)
//! **Performance**: ~10x faster than NTT (native float vs U64 emulation)
//! **CRITICAL**: Unblocks PPPM, structure factors, all wave physics

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// 1D Complex FFT operation
///
/// Transforms complex signal from time/spatial domain to frequency domain.
pub struct Fft1D {
    input: Tensor,
    degree: u32,
    twiddle_factors: Vec<f32>, // Precomputed exp(-2πik/N) as complex pairs
    pipeline_butterfly: wgpu::ComputePipeline,
    pipeline_bit_reverse: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl Fft1D {
    /// Create a new 1D FFT operation
    ///
    /// ## Parameters
    ///
    /// - `input`: Complex tensor (shape [..., N, 2] where last dim is (real, imag))
    /// - `degree`: FFT size N (must be power of 2)
    ///
    /// ## Constraints
    ///
    /// - N must be a power of 2 (for Cooley-Tukey radix-2 FFT)
    /// - Input must have last dimension = 2 (complex representation)
    pub fn new(input: Tensor, degree: u32) -> Result<Self> {
        // Validate input
        let shape = input.shape();
        if shape.last() != Some(&2) {
            return Err(BarracudaError::Device(
                "FFT input must have last dimension = 2 (complex)".to_string(),
            ));
        }

        // Validate degree is power of 2
        if degree & (degree - 1) != 0 {
            return Err(BarracudaError::Device(format!(
                "FFT degree {} must be power of 2",
                degree
            )));
        }

        let device = input.device();

        // ================================================================
        // PRECOMPUTE TWIDDLE FACTORS
        // ================================================================
        // twiddle[k] = exp(-2πik/N) for k = 0 to N-1
        // These are the roots of unity on the complex unit circle

        let mut twiddle_factors = Vec::with_capacity((degree * 2) as usize);
        let pi = std::f32::consts::PI;

        for k in 0..degree {
            let angle = -2.0 * pi * (k as f32) / (degree as f32);
            let real = angle.cos(); // exp(iθ) = cos(θ) + i·sin(θ)
            let imag = angle.sin();
            twiddle_factors.push(real);
            twiddle_factors.push(imag);
        }

        // ================================================================
        // LOAD SHADERS
        // ================================================================

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("FFT 1D Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("fft_1d.wgsl").into()),
            });

        // ================================================================
        // CREATE BIND GROUP LAYOUT
        // ================================================================

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("FFT 1D Bind Group Layout"),
                    entries: &[
                        // Binding 0: Input buffer (complex signal)
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
                        // Binding 1: Output buffer (complex spectrum)
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
                        // Binding 2: Twiddle factors
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
                        // Binding 3: Params (degree, stage, etc.)
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

        // ================================================================
        // CREATE PIPELINES
        // ================================================================

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("FFT 1D Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        // Butterfly pipeline (main FFT kernel)
        let pipeline_butterfly =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("FFT 1D Butterfly Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
                });

        // Bit-reversal pipeline (preprocessing)
        let pipeline_bit_reverse =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("FFT 1D Bit Reverse Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "bit_reverse",
                cache: None,
                compilation_options: Default::default(),
                });

        Ok(Self {
            input,
            degree,
            twiddle_factors,
            pipeline_butterfly,
            pipeline_bit_reverse,
            bind_group_layout,
        })
    }

    /// Execute FFT transformation
    ///
    /// Returns a new tensor containing the frequency-domain representation.
    ///
    /// ## Algorithm
    ///
    /// 1. Bit-reversal permutation (preprocessing)
    /// 2. log₂(N) butterfly stages (Cooley-Tukey FFT)
    /// 3. Each stage processes N/2 butterflies in parallel
    ///
    /// ## Complexity
    ///
    /// - Time: O(N log N)
    /// - Space: O(N) temporary buffer
    /// - GPU parallelism: N/2 threads per stage
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();

        // Buffer size: degree * 2 f32s (for complex numbers)
        let buffer_size = self.degree as u64 * 2 * std::mem::size_of::<f32>() as u64;

        // Create output buffer
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FFT Output Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create intermediate buffer (for ping-pong between stages)
        let intermediate_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FFT Intermediate Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Upload twiddle factors to GPU
        let twiddle_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("FFT Twiddle Factors"),
                contents: bytemuck::cast_slice(&self.twiddle_factors),
                usage: wgpu::BufferUsages::STORAGE,
            });

        // Params struct
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct FftParams {
            degree: u32,
            stage: u32,
            inverse: u32,
            _padding: u32,
        }

        let base_params = FftParams {
            degree: self.degree,
            stage: 0,
            inverse: 0, // Forward FFT
            _padding: 0,
        };

        // ============================================================
        // Pass 1: Bit-reversal permutation
        // ============================================================

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("FFT Command Encoder"),
            });

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("FFT Params (Bit Reverse)"),
                contents: bytemuck::bytes_of(&base_params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FFT Bit Reverse Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: intermediate_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: twiddle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("FFT Bit Reverse Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline_bit_reverse);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch: one thread per coefficient
            let workgroup_size = 256u32;
            let num_workgroups = self.degree.div_ceil(workgroup_size);
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        // Submit bit-reversal pass
        device.queue.submit(std::iter::once(encoder.finish()));

        // ============================================================
        // Pass 2-N: Butterfly stages (log₂(N) stages)
        // ============================================================

        let num_stages = (self.degree as f32).log2() as u32;
        let mut current_input = &intermediate_buffer;
        let mut current_output = &output_buffer;

        // Submit each stage separately to ensure sequential execution
        for stage in 0..num_stages {
            let mut stage_encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some(&format!("FFT Stage {} Encoder", stage)),
                    });

            let stage_params = FftParams {
                stage,
                ..base_params
            };

            let stage_params_buffer =
                device
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("FFT Params (Stage {})", stage)),
                        contents: bytemuck::bytes_of(&stage_params),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

            let stage_bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("FFT Butterfly Bind Group (Stage {})", stage)),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: current_input.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: current_output.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: twiddle_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: stage_params_buffer.as_entire_binding(),
                    },
                ],
            });

            {
                let mut compute_pass =
                    stage_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some(&format!("FFT Butterfly Pass (Stage {})", stage)),
                        timestamp_writes: None,
                    });

                compute_pass.set_pipeline(&self.pipeline_butterfly);
                compute_pass.set_bind_group(0, &stage_bind_group, &[]);

                // Dispatch: one thread per butterfly (N/2 butterflies per stage)
                let num_butterflies = self.degree / 2;
                let workgroup_size = 256u32;
                let num_workgroups = num_butterflies.div_ceil(workgroup_size);
                compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
            }

            // Submit THIS stage before moving to next
            device.queue.submit(std::iter::once(stage_encoder.finish()));

            // Ping-pong buffers for next stage
            std::mem::swap(&mut current_input, &mut current_output);
        }

        // After all swaps, determine which buffer has the final result
        let final_buffer = if num_stages.is_multiple_of(2) {
            intermediate_buffer
        } else {
            output_buffer
        };

        // Create result tensor
        Ok(Tensor::from_buffer(
            final_buffer,
            self.input.shape().to_vec(), // Preserve input shape!
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fft_1d_simple() {
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return;
        };

        // Simple 4-point FFT test
        // Input: [1+0i, 0+0i, 0+0i, 0+0i]
        let data = vec![
            1.0f32, 0.0, // 1+0i
            0.0, 0.0, // 0+0i
            0.0, 0.0, // 0+0i
            0.0, 0.0, // 0+0i
        ];

        let tensor = Tensor::from_data(&data, vec![4, 2], device.clone()).unwrap();
        let fft = Fft1D::new(tensor, 4).unwrap();
        let result = fft.execute().unwrap();

        let result_data = result.to_vec().unwrap();

        // FFT([1,0,0,0]) = [1,1,1,1] (all ones in frequency domain)
        assert!((result_data[0] - 1.0).abs() < 1e-5); // DC component
        assert!((result_data[2] - 1.0).abs() < 1e-5); // First harmonic
    }
}
