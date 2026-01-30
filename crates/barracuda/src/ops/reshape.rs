//! Reshape operation - Change tensor shape without copying data
//!
//! ## Deep Debt Principles
//!
//! - **Zero-copy**: Memory layout unchanged, only shape metadata changes
//! - **Pure abstraction**: Shader is identity operation, Rust handles reshaping
//! - **Type-safe**: Compile-time shape validation where possible
//!
//! ## Implementation
//!
//! Reshape is fundamentally a metadata operation - the underlying data buffer
//! remains unchanged, only the interpretation of its dimensions changes.
//! The WGSL shader is an identity copy for compatibility, but the real work
//! happens in the Rust wrapper managing tensor metadata.

use wgpu::util::DeviceExt;

/// Reshape parameters
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReshapeParams {
    /// Total number of elements (for validation)
    pub num_elements: u32,
    pub _padding: [u32; 3],
}

/// Reshape operation - change tensor shape without data copy
///
/// ## Usage
///
/// ```no_run
/// use barracuda::ops::reshape::*;
///
/// # async fn example(device: &wgpu::Device, queue: &wgpu::Queue) {
/// let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // Shape: [6]
/// let output = reshape(device, queue, &input, &[2, 3]).await.unwrap();
/// // Output shape: [2, 3], same data: [[1,2,3], [4,5,6]]
/// # }
/// ```
///
/// ## Deep Debt Note
///
/// In a fully-evolved tensor library, this would be a pure metadata operation
/// with zero GPU invocation. Current implementation maintains shader compatibility
/// for integration with existing pipeline, but optimizes away to identity copy.
pub async fn reshape(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    input: &[f32],
    new_shape: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let num_elements = input.len();
    let new_total: usize = new_shape.iter().product();
    
    // Validate: total elements must match
    if num_elements != new_total {
        return Err(format!(
            "Cannot reshape: input has {} elements, new shape {:?} requires {}",
            num_elements, new_shape, new_total
        ).into());
    }
    
    // Create params
    let params = ReshapeParams {
        num_elements: num_elements as u32,
        _padding: [0; 3],
    };
    
    // Create buffers
    let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Reshape Input"),
        contents: bytemuck::cast_slice(input),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Reshape Output"),
        size: (num_elements * std::mem::size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    
    let _params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Reshape Params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    
    // Load shader
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Reshape Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/reshape.wgsl").into()),
    });
    
    // Create pipeline
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Reshape Bind Group Layout"),
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
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Reshape Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Reshape Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
    });
    
    // Create bind group
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Reshape Bind Group"),
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
    
    // Execute
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Reshape Encoder"),
    });
    
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Reshape Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        let workgroups = (num_elements as u32 + 255) / 256;
        pass.dispatch_workgroups(workgroups, 1, 1);
    }
    
    // Read back
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Reshape Staging"),
        size: (num_elements * std::mem::size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    
    encoder.copy_buffer_to_buffer(
        &output_buffer,
        0,
        &staging_buffer,
        0,
        (num_elements * std::mem::size_of::<f32>()) as u64,
    );
    
    queue.submit(Some(encoder.finish()));
    
    let buffer_slice = staging_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);
    
    let data = buffer_slice.get_mapped_range();
    let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging_buffer.unmap();
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_reshape_2d_to_1d() {
        let (device, queue) = crate::test_utils::create_device().await.unwrap();
        
        let input = vec![
            1.0, 2.0, 3.0,
            4.0, 5.0, 6.0,
        ]; // Shape: [2, 3]
        
        let output = reshape(&device, &queue, &input, &[6]).await.unwrap();
        
        assert_eq!(output.len(), 6);
        assert_eq!(output, input); // Data unchanged
    }
    
    #[tokio::test]
    async fn test_reshape_1d_to_2d() {
        let (device, queue) = crate::test_utils::create_device().await.unwrap();
        
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // Shape: [6]
        
        let output = reshape(&device, &queue, &input, &[2, 3]).await.unwrap();
        
        assert_eq!(output.len(), 6);
        assert_eq!(output, input); // Data unchanged, shape is metadata
    }
    
    #[tokio::test]
    async fn test_reshape_3d() {
        let (device, queue) = crate::test_utils::create_device().await.unwrap();
        
        let input: Vec<f32> = (0..24).map(|i| i as f32).collect(); // 24 elements
        
        let output = reshape(&device, &queue, &input, &[2, 3, 4]).await.unwrap();
        
        assert_eq!(output.len(), 24);
        assert_eq!(output, input);
    }
    
    #[tokio::test]
    async fn test_reshape_invalid_size() {
        let (device, queue) = crate::test_utils::create_device().await.unwrap();
        
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 6 elements
        
        let result = reshape(&device, &queue, &input, &[2, 4]).await; // Needs 8 elements
        
        assert!(result.is_err());
    }
}
