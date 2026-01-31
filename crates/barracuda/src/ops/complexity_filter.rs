//! Complexity filter for sequence analysis
//!
//! Identifies low-complexity regions in DNA/RNA sequences,
//! such as homopolymers (AAAA) or simple repeats (ATATAT).
//!
//! # Low-Complexity Filtering
//!
//! Low-complexity regions are stretches of sequence with limited diversity,
//! often artifacts or of low informational value. Filtering them improves:
//! - Sequence alignment quality
//! - Gene finding accuracy
//! - Database search specificity
//!
//! # Algorithm
//!
//! **Sliding window entropy**: Measure base diversity in fixed windows
//! - Input: DNA/RNA sequence + window size + threshold
//! - Output: Boolean array (1.0 = low complexity, 0.0 = normal)
//! - Method: Count unique bases in window, flag if below threshold
//!
//! # Example
//!
//! ```no_run
//! use barracuda::complexity_filter;
//! use barracuda::WgpuDevice;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let device = WgpuDevice::new().await?;
//!
//! let sequence = b"AAAAAAATCGATCG";  // Low complexity at start
//! let result = complexity_filter(&device.device, &device.queue, sequence, 5, 2).await?;
//! // First positions flagged as low-complexity (only A's)
//! # Ok(())
//! # }
//! ```

use std::borrow::Cow;
use wgpu::util::DeviceExt;

use crate::error::{BarracudaError, Result as BarracudaResult};

/// Filter low-complexity regions in sequences
///
/// # Arguments
///
/// * `device` - The `wgpu` device
/// * `queue` - The `wgpu` queue  
/// * `sequence` - DNA/RNA sequence (ASCII)
/// * `window_size` - Size of sliding window
/// * `min_unique` - Minimum unique bases to be considered complex
///
/// # Returns
///
/// Boolean array (1.0 = low complexity, 0.0 = normal complexity)
///
/// # Errors
///
/// Returns `BarracudaError` if:
/// - Sequence is empty
/// - Window size is zero or larger than sequence
/// - GPU execution fails
pub async fn complexity_filter(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sequence: &[u8],
    window_size: u32,
    min_unique: u32,
) -> BarracudaResult<Vec<f32>> {
    if sequence.is_empty() {
        return Err(BarracudaError::InvalidInput {
            message: "Sequence cannot be empty".to_string(),
        });
    }

    if window_size == 0 || window_size as usize > sequence.len() {
        return Err(BarracudaError::InvalidInput {
            message: "Invalid window size".to_string(),
        });
    }

    let n = sequence.len() as u32;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Complexity Filter Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("complexity_filter.wgsl"))),
    });

    // Convert u8 to u32 for GPU
    let sequence_u32: Vec<u32> = sequence.iter().map(|&x| x as u32).collect();

    let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Sequence Buffer"),
        contents: bytemuck::cast_slice(&sequence_u32),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: (n * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
        window_size: u32,
        min_unique: u32,
    }

    let params = Params {
        n,
        window_size,
        min_unique,
    };

    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Params Buffer"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Complexity Filter Layout"),
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

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Complexity Filter Bind Group"),
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

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Complexity Filter Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Complexity Filter Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "complexity_filter",
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Complexity Filter Encoder"),
    });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Complexity Filter Pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups((n + 255) / 256, 1, 1);
    }

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: (n * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, (n * std::mem::size_of::<f32>() as u32) as u64);
    queue.submit(Some(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| { let _ = sender.send(result); });
    device.poll(wgpu::Maintain::Wait);
    receiver.await.map_err(|_| BarracudaError::ExecutionError { message: "Failed to receive buffer".to_string() })?
        .map_err(|e| BarracudaError::ExecutionError { message: format!("Buffer mapping failed: {:?}", e) })?;

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
    async fn test_complexity_filter_basic() {
        let device = WgpuDevice::new().await.unwrap();
        let sequence = b"AAAAAAATCGATCG";  // Low complexity at start (all A's)
        let result = complexity_filter(&device.device, &device.queue, sequence, 5, 2).await.unwrap();
        assert_eq!(result.len(), 14);
        // First positions should be flagged as low complexity
        assert!(result[0] > 0.5, "Position 0 should be low complexity");
        assert!(result[1] > 0.5, "Position 1 should be low complexity");
        // Later positions with diverse bases should be normal
        assert!(result[10] < 0.5, "Position 10 should be normal complexity");
    }

    #[tokio::test]
    async fn test_complexity_filter_edge_cases() {
        let device = WgpuDevice::new().await.unwrap();
        let all_same = b"AAAAAAAA";
        let result = complexity_filter(&device.device, &device.queue, all_same, 4, 2).await.unwrap();
        // Only check positions where window fits (0-4)
        assert!(result[0..=4].iter().all(|&x| x > 0.5), "Positions 0-4 should be low complexity");

        let all_diverse = b"ATCGATCG";
        let result2 = complexity_filter(&device.device, &device.queue, all_diverse, 4, 2).await.unwrap();
        // All windows have 4 unique bases, should be normal complexity
        assert!(result2[0..=4].iter().all(|&x| x < 0.5), "Positions 0-4 should be normal complexity");
    }

    #[tokio::test]
    async fn test_complexity_filter_boundary() {
        let device = WgpuDevice::new().await.unwrap();
        let empty: &[u8] = b"";
        assert!(complexity_filter(&device.device, &device.queue, empty, 5, 2).await.is_err());
        
        let short = b"ATCG";
        assert!(complexity_filter(&device.device, &device.queue, short, 0, 2).await.is_err());
        assert!(complexity_filter(&device.device, &device.queue, short, 10, 2).await.is_err());
    }

    #[tokio::test]
    async fn test_complexity_filter_large_tensor() {
        let device = WgpuDevice::new().await.unwrap();
        let large: Vec<u8> = (0..10000).map(|i| match i % 4 {
            0 => b'A',
            1 => b'T',
            2 => b'G',
            3 => b'C',
            _ => unreachable!(),
        }).collect();
        let result = complexity_filter(&device.device, &device.queue, &large, 4, 2).await.unwrap();
        assert_eq!(result.len(), 10000);
        assert!(result.iter().all(|&x| x.is_finite()));
        // With ATGC pattern, all windows should have 4 unique bases (normal complexity)
        assert!(result.iter().all(|&x| x < 0.5));
    }

    #[tokio::test]
    async fn test_complexity_filter_precision() {
        let device = WgpuDevice::new().await.unwrap();
        let sequence = b"ATATATATAT";  // Only 2 unique bases (A, T)
        let result = complexity_filter(&device.device, &device.queue, sequence, 5, 3).await.unwrap();
        assert!(result.iter().all(|&x| x == 0.0 || x == 1.0));
        // With min_unique=3, all windows should be flagged as low complexity
        let low_count = result.iter().filter(|&&x| x > 0.5).count();
        assert!(low_count > 0, "Expected low complexity regions");
    }
}
