//! Edge Convolution for Graph Neural Networks
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Learns edge features by aggregating neighbor information

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct EdgeConvParams {
    num_nodes: u32,
    feature_dim: u32,
    output_dim: u32,
    k_neighbors: u32,
}

pub struct EdgeConv {
    node_features: Tensor,
    edge_index: Tensor,
    mlp_weight: Tensor,
    mlp_bias: Tensor,
    k_neighbors: u32,
}

impl EdgeConv {
    /// Create EdgeConv operation
    pub fn new(
        node_features: Tensor,
        edge_index: Tensor,
        mlp_weight: Tensor,
        mlp_bias: Tensor,
        k_neighbors: u32,
    ) -> Result<Self> {
        if k_neighbors == 0 {
            return Err(BarracudaError::invalid_op(
                "EdgeConv",
                "k_neighbors must be > 0",
            ));
        }

        Ok(Self {
            node_features,
            edge_index,
            mlp_weight,
            mlp_bias,
            k_neighbors,
        })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/edge_conv.wgsl")
    }

    /// Execute EdgeConv on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.node_features.device();
        let node_shape = self.node_features.shape();
        
        if node_shape.len() != 2 {
            return Err(BarracudaError::invalid_op(
                "EdgeConv",
                format!("node_features must be 2D [num_nodes, feature_dim], got shape {:?}", node_shape),
            ));
        }

        let num_nodes = node_shape[0];
        let feature_dim = node_shape[1];
        let output_dim = self.mlp_bias.len();

        // Create output buffer: [num_nodes, output_dim]
        let output_size = num_nodes * output_dim;
        let output_buffer = device.create_buffer_f32(output_size)?;

        let params = EdgeConvParams {
            num_nodes: num_nodes as u32,
            feature_dim: feature_dim as u32,
            output_dim: output_dim as u32,
            k_neighbors: self.k_neighbors,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("EdgeConv Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("EdgeConv Bind Group Layout"),
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
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("EdgeConv Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.node_features.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.edge_index.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.mlp_weight.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.mlp_bias.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("EdgeConv"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("EdgeConv Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("EdgeConv Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("EdgeConv Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("EdgeConv Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(&device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
            let workgroups = (num_nodes as u32 + optimal_wg_size - 1) / optimal_wg_size;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![num_nodes, output_dim],
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_edge_conv_basic() {
        let device = get_test_device().await;

        let num_nodes = 5;
        let feature_dim = 3;
        let output_dim = 4;

        let node_features = Tensor::from_vec_on(
            vec![1.0; num_nodes * feature_dim],
            vec![num_nodes, feature_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let edge_index = Tensor::from_vec_on(
            vec![0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0], // Simple chain graph
            vec![4, 2],
            device.clone(),
        )
        .await
        .unwrap();

        let mlp_weight = Tensor::from_vec_on(
            vec![0.1; output_dim * 2 * feature_dim],
            vec![output_dim, 2 * feature_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let mlp_bias = Tensor::from_vec_on(
            vec![0.0; output_dim],
            vec![output_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let result = EdgeConv::new(node_features, edge_index, mlp_weight, mlp_bias, 2)
            .unwrap()
            .execute()
            .unwrap();

        assert_eq!(result.shape(), &[num_nodes, output_dim]);
    }
}
