//! TopK operation - Find K largest values and their indices
//!
//! ## Deep Debt Principles
//!
//! - **Complete implementation**: Not just indices, returns values too
//! - **Production-ready**: Handles edge cases (k > size, empty input)
//! - **Modern Rust**: Clean API, proper error handling
//!
//! ## Implementation Note
//!
//! Current WGSL shader is a simple selection sort (O(k*n)). For production at scale,
//! this should evolve to parallel sorting (radix/bitonic) for O(n log k) performance.
//! This is a "good enough" implementation following deep debt principle:
//! "ship complete, then optimize."

use wgpu::util::DeviceExt;

/// TopK parameters
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TopKParams {
    /// Number of top elements to select
    pub k: u32,
    pub _padding: [u32; 3], // Pad to 16 bytes for uniform buffer alignment
}

/// TopK result containing both indices and values
#[derive(Debug, Clone)]
pub struct TopKResult {
    /// Indices of top K elements in the input
    pub indices: Vec<u32>,
    /// Values of top K elements
    pub values: Vec<f32>,
}

/// Find top K largest values and their indices
///
/// ## Usage
///
/// ```no_run
/// use barracuda::ops::topk::*;
///
/// # async fn example(device: &wgpu::Device, queue: &wgpu::Queue) {
/// let input = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
/// let result = topk(device, queue, &input, 3).await.unwrap();
/// // result.indices = [5, 7, 4] (indices of 9.0, 6.0, 5.0)
/// // result.values = [9.0, 6.0, 5.0]
/// # }
/// ```
///
/// ## Complexity
///
/// - Time: O(k * n) for selection sort
/// - Space: O(k) output
///
/// ## Deep Debt Evolution Path
///
/// Future optimization: Implement parallel sorting for O(n log k) when k is large.
/// Current implementation prioritizes correctness over performance - "make it work,
/// make it right, make it fast" principle.
pub async fn topk(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    input: &[f32],
    k: usize,
) -> Result<TopKResult, Box<dyn std::error::Error>> {
    if input.is_empty() {
        return Ok(TopKResult {
            indices: Vec::new(),
            values: Vec::new(),
        });
    }
    
    let k = k.min(input.len()); // Clamp k to input size
    
    // Create params
    let params = TopKParams {
        k: k as u32,
        _padding: [0; 3],
    };
    
    // Create buffers
    let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("TopK Input"),
        contents: bytemuck::cast_slice(input),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("TopK Output"),
        size: (k * std::mem::size_of::<u32>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    
    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("TopK Params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    
    // Load shader
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("TopK Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/topk.wgsl").into()),
    });
    
    // Create pipeline
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("TopK Bind Group Layout"),
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
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("TopK Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("TopK Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
    });
    
    // Create bind group
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("TopK Bind Group"),
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
    
    // Execute
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("TopK Encoder"),
    });
    
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("TopK Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1); // Single workgroup for serial selection
    }
    
    // Read back indices
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("TopK Staging"),
        size: (k * std::mem::size_of::<u32>()) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    
    encoder.copy_buffer_to_buffer(
        &output_buffer,
        0,
        &staging_buffer,
        0,
        (k * std::mem::size_of::<u32>()) as u64,
    );
    
    queue.submit(Some(encoder.finish()));
    
    let buffer_slice = staging_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);
    
    let data = buffer_slice.get_mapped_range();
    let indices: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging_buffer.unmap();
    
    // Extract values from input using indices
    let values: Vec<f32> = indices.iter().map(|&idx| input[idx as usize]).collect();
    
    Ok(TopKResult { indices, values })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_topk_basic() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        
        let input = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        let result = topk(&device, &queue, &input, 3).await.unwrap();
        
        assert_eq!(result.indices.len(), 3);
        assert_eq!(result.values.len(), 3);
        
        // Should find indices of 9.0, 6.0, 5.0
        assert_eq!(result.indices[0], 5); // 9.0
        assert_eq!(result.values[0], 9.0);
    }
    
    #[tokio::test]
    async fn test_topk_k_larger_than_input() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        
        let input = vec![3.0, 1.0, 4.0];
        let result = topk(&device, &queue, &input, 10).await.unwrap();
        
        // Should clamp to input size
        assert_eq!(result.indices.len(), 3);
        assert_eq!(result.values.len(), 3);
    }
    
    #[tokio::test]
    async fn test_topk_empty() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        
        let input: Vec<f32> = vec![];
        let result = topk(&device, &queue, &input, 5).await.unwrap();
        
        assert!(result.indices.is_empty());
        assert!(result.values.is_empty());
    }
    
    #[tokio::test]
    async fn test_topk_single() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        
        let input = vec![1.0, 5.0, 3.0, 7.0, 2.0];
        let result = topk(&device, &queue, &input, 1).await.unwrap();
        
        assert_eq!(result.indices.len(), 1);
        assert_eq!(result.indices[0], 3); // Index of 7.0
        assert_eq!(result.values[0], 7.0);
    }
    
    #[tokio::test]
    async fn test_topk_large_tensor() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        
        // Large tensor with 1000 elements
        let size = 1000;
        let input: Vec<f32> = (0..size).map(|i| (i % 100) as f32).collect();
        
        let k = 10;
        let result = topk(&device, &queue, &input, k).await.unwrap();
        
        assert_eq!(result.indices.len(), k);
        assert_eq!(result.values.len(), k);
        
        // All top values should be 99.0 (max value in the pattern)
        for &val in &result.values {
            assert!((val - 99.0).abs() < 1e-5 || (val - 98.0).abs() < 1e-5);
        }
    }
}
