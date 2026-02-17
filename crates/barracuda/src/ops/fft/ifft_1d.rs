//! Inverse 1D Fast Fourier Transform Operation
//!
//! **Purpose**: Transform from frequency domain back to time/spatial domain
//! **Algorithm**: FFT with conjugated twiddles + normalization by 1/N
//!
//! **Mathematical Property**:
//! ```text
//! IFFT(FFT(x)) = x
//! ```
//! This is THE validation test for FFT correctness!

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// 1D Inverse Complex FFT operation
pub struct Ifft1D {
    input: Tensor,
    degree: u32,
    twiddle_factors: Vec<f32>,
    pipeline_butterfly: wgpu::ComputePipeline,
    pipeline_bit_reverse: wgpu::ComputePipeline,
    pipeline_normalize: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group_layout_normalize: wgpu::BindGroupLayout,
}

impl Ifft1D {
    /// Create a new 1D IFFT operation
    pub fn new(input: Tensor, degree: u32) -> Result<Self> {
        let shape = input.shape();
        if shape.last() != Some(&2) {
            return Err(BarracudaError::Device(
                "IFFT input must have last dimension = 2 (complex)".to_string(),
            ));
        }

        if degree & (degree - 1) != 0 {
            return Err(BarracudaError::Device(format!(
                "IFFT degree {} must be power of 2",
                degree
            )));
        }

        let device = input.device();

        // Precompute twiddle factors (CONJUGATED for inverse transform)
        // twiddle[k] = exp(+2πik/N) (note: positive sign for inverse!)
        let mut twiddle_factors = Vec::with_capacity((degree * 2) as usize);
        let pi = std::f32::consts::PI;

        for k in 0..degree {
            let angle = 2.0 * pi * (k as f32) / (degree as f32); // Positive for IFFT!
            let real = angle.cos();
            let imag = angle.sin();
            twiddle_factors.push(real);
            twiddle_factors.push(imag);
        }

        // Load shader (reuse FFT shader!)
        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("IFFT 1D Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("fft_1d.wgsl").into()),
            });

        // Normalization shader
        let normalize_shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("IFFT Normalize Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("ifft_normalize.wgsl").into()),
            });

        // Bind group layout (same as FFT)
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("IFFT 1D Bind Group Layout"),
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
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        // Normalize bind group layout (simpler - just input/output + params)
        let bind_group_layout_normalize =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("IFFT Normalize BGL"),
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("IFFT 1D Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let normalize_pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("IFFT Normalize Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout_normalize],
                    push_constant_ranges: &[],
                });

        let pipeline_butterfly =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("IFFT 1D Butterfly Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
                });

        let pipeline_bit_reverse =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("IFFT 1D Bit Reverse Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "bit_reverse",
                cache: None,
                compilation_options: Default::default(),
                });

        let pipeline_normalize =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("IFFT Normalize Pipeline"),
                    layout: Some(&normalize_pipeline_layout),
                    module: &normalize_shader,
                    entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
                });

        Ok(Self {
            input,
            degree,
            twiddle_factors,
            pipeline_butterfly,
            pipeline_bit_reverse,
            pipeline_normalize,
            bind_group_layout,
            bind_group_layout_normalize,
        })
    }

    /// Execute IFFT transformation
    ///
    /// Returns time/spatial domain representation.
    /// Output is normalized by 1/N.
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let buffer_size = self.degree as u64 * 2 * std::mem::size_of::<f32>() as u64;

        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("IFFT Output Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let intermediate_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("IFFT Intermediate Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let twiddle_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("IFFT Twiddle Factors"),
                contents: bytemuck::cast_slice(&self.twiddle_factors),
                usage: wgpu::BufferUsages::STORAGE,
            });

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
            inverse: 1, // IFFT flag
            _padding: 0,
        };

        // Pass 1: Bit-reversal
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("IFFT Command Encoder"),
            });

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("IFFT Params (Bit Reverse)"),
                contents: bytemuck::bytes_of(&base_params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("IFFT Bit Reverse Bind Group"),
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
                label: Some("IFFT Bit Reverse Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline_bit_reverse);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let workgroup_size = 256u32;
            let num_workgroups = self.degree.div_ceil(workgroup_size);
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        device.queue.submit(std::iter::once(encoder.finish()));

        // Pass 2-N: Butterfly stages
        let num_stages = (self.degree as f32).log2() as u32;
        let mut current_input = &intermediate_buffer;
        let mut current_output = &output_buffer;

        for stage in 0..num_stages {
            let mut stage_encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some(&format!("IFFT Stage {} Encoder", stage)),
                    });

            let stage_params = FftParams {
                stage,
                ..base_params
            };

            let stage_params_buffer =
                device
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("IFFT Params (Stage {})", stage)),
                        contents: bytemuck::bytes_of(&stage_params),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

            let stage_bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("IFFT Butterfly Bind Group (Stage {})", stage)),
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
                        label: Some(&format!("IFFT Butterfly Pass (Stage {})", stage)),
                        timestamp_writes: None,
                    });

                compute_pass.set_pipeline(&self.pipeline_butterfly);
                compute_pass.set_bind_group(0, &stage_bind_group, &[]);

                let num_butterflies = self.degree / 2;
                let workgroup_size = 256u32;
                let num_workgroups = num_butterflies.div_ceil(workgroup_size);
                compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
            }

            device.queue.submit(std::iter::once(stage_encoder.finish()));
            std::mem::swap(&mut current_input, &mut current_output);
        }

        let butterfly_result_buffer = if num_stages.is_multiple_of(2) {
            &intermediate_buffer
        } else {
            &output_buffer
        };

        // Pass N+1: Normalize by 1/N
        let final_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("IFFT Final Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct NormalizeParams {
            degree: u32,
            _padding: [u32; 3],
        }

        let normalize_params = NormalizeParams {
            degree: self.degree,
            _padding: [0; 3],
        };

        let normalize_params_buffer =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("IFFT Normalize Params"),
                    contents: bytemuck::bytes_of(&normalize_params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let normalize_bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("IFFT Normalize Bind Group"),
            layout: &self.bind_group_layout_normalize,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: butterfly_result_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: final_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: normalize_params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut normalize_encoder =
            device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("IFFT Normalize Encoder"),
                });

        {
            let mut compute_pass =
                normalize_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("IFFT Normalize Pass"),
                    timestamp_writes: None,
                });

            compute_pass.set_pipeline(&self.pipeline_normalize);
            compute_pass.set_bind_group(0, &normalize_bind_group, &[]);

            let workgroup_size = 256u32;
            let num_workgroups = self.degree.div_ceil(workgroup_size);
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        device
            .queue
            .submit(std::iter::once(normalize_encoder.finish()));

        Ok(Tensor::from_buffer(
            final_buffer,
            self.input.shape().to_vec(), // Preserve input shape!
            device.clone(),
        ))
    }
}
