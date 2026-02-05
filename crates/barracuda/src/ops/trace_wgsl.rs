//! Trace - Sum of diagonal elements - Pure WGSL
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (GPU-optimized)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready)
//!
//! ## Algorithm
//!
//! Computes the trace of a square matrix:
//! ```text
//! trace(A) = sum of diagonal elements
//! For [[a, b], [c, d]]: trace = a + d
//! ```

use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

pub struct Trace {
    input: Tensor,
}

impl Trace {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }
    
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/trace.wgsl")
    }
    
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        
        // Expect 2D square matrix
        if shape.len() != 2 || shape[0] != shape[1] {
            return Err(crate::error::BarracudaError::InvalidShape {
                expected: vec![0, 0],
                actual: shape.to_vec(),
            });
        }
        
        let n = shape[0];
        
        // Calculate workgroups needed
        let workgroups = (n as u32 + 255) / 256;
        
        // Output buffer: single element for final result, or partial results if multi-workgroup
        let output_size = if workgroups > 1 { workgroups as usize } else { 1 };
        let output_buffer = device.create_buffer_f32(output_size)?;
        
        let params_buffer = device.create_uniform_buffer("Params", &[n as u32]);
        
        let bind_group_layout = device.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("Trace BGL"),
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
            }
        );
        
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Trace BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
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
        
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Trace"));
        let pipeline_layout = device.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Trace PL"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            }
        );
        
        let pipeline = device.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: Some("Trace Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            }
        );
        
        let mut encoder = device.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Trace Encoder"),
            }
        );
        
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Trace Pass"),
                timestamp_writes: None,
            });
            
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        
        device.queue.submit(Some(encoder.finish()));
        
        // If multiple workgroups, reduce partial results in a second pass using reduce shader
        let final_buffer = if workgroups > 1 {
            // Second pass: reduce partial results using reduce.wgsl shader
            let reduce_shader_source = include_str!("../shaders/reduce.wgsl");
            let reduce_shader = device.device.create_shader_module(
                wgpu::ShaderModuleDescriptor {
                    label: Some("Trace Reduce Shader"),
                    source: wgpu::ShaderSource::Wgsl(reduce_shader_source.into()),
                }
            );
            
            #[repr(C)]
            #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
            struct ReduceParams {
                size: u32,
                operation: u32,  // 0 = Sum
            }
            
            let reduce_params = ReduceParams {
                size: workgroups,
                operation: 0u32,  // Sum operation
            };
            
            let final_output_buffer = device.create_buffer_f32(1)?;
            let reduce_params_buffer = device.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Trace Reduce Params"),
                    contents: bytemuck::cast_slice(&[reduce_params]),
                    usage: wgpu::BufferUsages::UNIFORM,
                }
            );
            
            let bind_group_layout_2 = device.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("Trace Reduce BGL"),
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
                }
            );
            
            let bind_group_2 = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Trace Reduce BG"),
                layout: &bind_group_layout_2,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: final_output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: reduce_params_buffer.as_entire_binding(),
                    },
                ],
            });
            
            let pipeline_layout_2 = device.device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Trace Reduce PL"),
                    bind_group_layouts: &[&bind_group_layout_2],
                    push_constant_ranges: &[],
                }
            );
            
            let pipeline_2 = device.device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("Trace Reduce Pipeline"),
                    layout: Some(&pipeline_layout_2),
                    module: &reduce_shader,
                    entry_point: "main",
                }
            );
            
            let mut encoder_2 = device.device.create_command_encoder(
                &wgpu::CommandEncoderDescriptor {
                    label: Some("Trace Reduce Encoder"),
                }
            );
            
            {
                let mut pass_2 = encoder_2.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Trace Reduce Pass"),
                    timestamp_writes: None,
                });
                
                pass_2.set_pipeline(&pipeline_2);
                pass_2.set_bind_group(0, &bind_group_2, &[]);
                // Single workgroup for reducing partial results
                pass_2.dispatch_workgroups(1, 1, 1);
            }
            
            device.queue.submit(Some(encoder_2.finish()));
            final_output_buffer
        } else {
            output_buffer
        };
        
        // Return scalar tensor with trace result
        Ok(Tensor::from_buffer(
            final_buffer,
            vec![1],
            device.clone(),
        ))
    }
}

impl Tensor {
    pub fn trace_wgsl(self) -> Result<Self> {
        Trace::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_trace_2x2() {
        let device = get_test_device().await;
        let input_data = vec![
            1.0, 2.0,
            3.0, 4.0,
        ];
        let input = Tensor::from_vec_on(input_data, vec![2, 2], device)
            .await
            .unwrap();
        
        let result = input.trace_wgsl().unwrap();
        let trace_result = result.to_vec().unwrap();
        
        // Result should be scalar tensor [trace_value]
        assert_eq!(trace_result.len(), 1);
        // Trace = 1.0 + 4.0 = 5.0
        assert!((trace_result[0] - 5.0).abs() < 1e-5);
    }
    
    #[tokio::test]
    async fn test_trace_3x3() {
        let device = get_test_device().await;
        // Matrix: [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
        // Diagonal: [1, 5, 9], trace = 15
        let input_data = vec![
            1.0, 2.0, 3.0,
            4.0, 5.0, 6.0,
            7.0, 8.0, 9.0,
        ];
        let input = Tensor::from_vec_on(input_data, vec![3, 3], device)
            .await
            .unwrap();
        
        let result = input.trace_wgsl().unwrap();
        let trace_result = result.to_vec().unwrap();
        
        assert_eq!(trace_result.len(), 1);
        assert!((trace_result[0] - 15.0).abs() < 1e-5);
    }
    
    #[tokio::test]
    async fn test_trace_large_matrix() {
        let device = get_test_device().await;
        let n = 512; // Larger than workgroup size to test multi-workgroup reduction
        let mut input_data = vec![0.0; n * n];
        
        // Fill diagonal with sequential values: 1, 2, 3, ..., n
        for i in 0..n {
            input_data[i * n + i] = (i + 1) as f32;
        }
        
        let input = Tensor::from_vec_on(input_data, vec![n, n], device)
            .await
            .unwrap();
        
        let result = input.trace_wgsl().unwrap();
        let trace_result = result.to_vec().unwrap();
        
        assert_eq!(trace_result.len(), 1);
        // Sum of 1..n = n*(n+1)/2
        let expected = (n * (n + 1) / 2) as f32;
        assert!((trace_result[0] - expected).abs() < 1e-4);
    }
}
