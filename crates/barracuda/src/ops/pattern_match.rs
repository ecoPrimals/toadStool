//! Pattern matching operation for sequence analysis
//!
//! Performs efficient pattern matching on character sequences,
//! essential for bioinformatics and data filtering applications.
//!
//! # Pattern Matching
//!
//! This operation searches for occurrences of a pattern within a target sequence.
//! Commonly used in DNA/RNA sequence analysis, text processing, and data validation.
//!
//! # Algorithm
//!
//! **Naive string matching**: Compare pattern at each position
//! - Input: Target sequence (bytes) + pattern (bytes)
//! - Output: Match positions (bool array)
//! - Complexity: O(n*m) where n=target length, m=pattern length
//!
//! # Example
//!
//! ```no_run
//! use barracuda::pattern_match;
//! use barracuda::WgpuDevice;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let device = WgpuDevice::new().await?;
//!
//! let target = b"ATCGATCGATCG";
//! let pattern = b"TCG";
//! let matches = pattern_match(&device.device, &device.queue, target, pattern).await?;
//! // Returns: [0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0]
//! //           A  T  C  G  A  T  C  G  A  T  C  G
//! //              ^TCG     ^TCG        ^TCG
//! # Ok(())
//! # }
//! ```

use std::borrow::Cow;
use wgpu::util::DeviceExt;

use crate::error::{BarracudaError, Result as BarracudaResult};

/// Pattern matching on byte sequences
///
/// # Arguments
///
/// * `device` - The `wgpu` device
/// * `queue` - The `wgpu` queue  
/// * `target` - Target sequence to search in
/// * `pattern` - Pattern to search for
///
/// # Returns
///
/// Boolean array indicating match positions (1.0 = match at this position, 0.0 = no match)
///
/// # Errors
///
/// Returns `BarracudaError` if:
/// - Target or pattern is empty
/// - Pattern is longer than target
/// - GPU execution fails
pub async fn pattern_match(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    target: &[u8],
    pattern: &[u8],
) -> BarracudaResult<Vec<f32>> {
    if target.is_empty() || pattern.is_empty() {
        return Err(BarracudaError::InvalidInput {
            message: "Target and pattern cannot be empty".to_string(),
        });
    }

    if pattern.len() > target.len() {
        return Err(BarracudaError::InvalidInput {
            message: "Pattern cannot be longer than target".to_string(),
        });
    }

    let target_len = target.len() as u32;
    let pattern_len = pattern.len() as u32;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Pattern Match Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("pattern_match.wgsl"))),
    });

    // Convert u8 to u32 for GPU (WGSL doesn't have u8)
    let target_u32: Vec<u32> = target.iter().map(|&x| x as u32).collect();
    let pattern_u32: Vec<u32> = pattern.iter().map(|&x| x as u32).collect();

    let target_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Target Buffer"),
        contents: bytemuck::cast_slice(&target_u32),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let pattern_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Pattern Buffer"),
        contents: bytemuck::cast_slice(&pattern_u32),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: (target_len * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        target_len: u32,
        pattern_len: u32,
    }

    let params = Params {
        target_len,
        pattern_len,
    };

    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Params Buffer"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Pattern Match Layout"),
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

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Pattern Match Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: target_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: pattern_buffer.as_entire_binding(),
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

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pattern Match Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Pattern Match Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "pattern_match",
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Pattern Match Encoder"),
    });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Pattern Match Pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups((target_len + 255) / 256, 1, 1);
    }

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: (target_len * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(
        &output_buffer,
        0,
        &staging_buffer,
        0,
        (target_len * std::mem::size_of::<f32>() as u32) as u64,
    );
    queue.submit(Some(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    receiver
        .await
        .map_err(|_| BarracudaError::ExecutionError {
            message: "Failed to receive buffer".to_string(),
        })?
        .map_err(|e| BarracudaError::ExecutionError {
            message: format!("Buffer mapping failed: {:?}", e),
        })?;

    let data = buffer_slice.get_mapped_range();
    let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging_buffer.unmap();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;

    #[tokio::test]
    async fn test_pattern_match_basic() {
        let device = WgpuDevice::new().await.unwrap();
        let target = b"ATCGATCGATCG";
        let pattern = b"TCG";
        let result = pattern_match(&device.device, &device.queue, target, pattern)
            .await
            .unwrap();
        assert_eq!(result.len(), 12);
        // Pattern "TCG" starts at positions 1, 5, 9
        // A T C G A T C G A T C G
        // 0 1 2 3 4 5 6 7 8 9 10 11
        //   ^TCG    ^TCG    ^TCG
        assert!(result[1] > 0.5, "Expected match at position 1");
        assert!(result[5] > 0.5, "Expected match at position 5");
        assert!(result[9] > 0.5, "Expected match at position 9");
        assert!(result[0] < 0.5);
        assert!(result[2] < 0.5);
    }

    #[tokio::test]
    async fn test_pattern_match_edge_cases() {
        let device = WgpuDevice::new().await.unwrap();
        let target = b"AAAAAAA";
        let pattern = b"A";
        let result = pattern_match(&device.device, &device.queue, target, pattern)
            .await
            .unwrap();
        assert!(result.iter().all(|&x| x > 0.5));

        let target2 = b"ATCGATCG";
        let pattern2 = b"XYZ";
        let result2 = pattern_match(&device.device, &device.queue, target2, pattern2)
            .await
            .unwrap();
        assert!(result2.iter().all(|&x| x < 0.5));
    }

    #[tokio::test]
    async fn test_pattern_match_boundary() {
        let device = WgpuDevice::new().await.unwrap();
        let empty: &[u8] = b"";
        assert!(pattern_match(&device.device, &device.queue, empty, b"A")
            .await
            .is_err());
        assert!(pattern_match(&device.device, &device.queue, b"A", empty)
            .await
            .is_err());
        assert!(pattern_match(&device.device, &device.queue, b"AT", b"ATG")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_pattern_match_large_tensor() {
        let device = WgpuDevice::new().await.unwrap();
        let large_target: Vec<u8> = (0..10000).map(|i| b'A' + (i % 4) as u8).collect();
        let pattern = b"ABC";
        let result = pattern_match(&device.device, &device.queue, &large_target, pattern)
            .await
            .unwrap();
        assert_eq!(result.len(), 10000);
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_pattern_match_precision() {
        let device = WgpuDevice::new().await.unwrap();
        let target = b"ABCDEFABCDEF";
        let pattern = b"DEF";
        let result = pattern_match(&device.device, &device.queue, target, pattern)
            .await
            .unwrap();
        assert!(result.iter().all(|&x| x == 0.0 || x == 1.0));
        let match_count = result.iter().filter(|&&x| x > 0.5).count();
        assert_eq!(match_count, 2);
    }
}
