//! GC content calculation for DNA/RNA sequences
//!
//! Calculates the percentage of Guanine (G) and Cytosine (C) nucleotides
//! in DNA or RNA sequences, a critical metric in bioinformatics.
//!
//! # GC Content
//!
//! GC content is the percentage of nitrogenous bases in a DNA or RNA molecule
//! that are either guanine (G) or cytosine (C). It's used for:
//! - Gene prediction
//! - Primer design
//! - Species identification
//! - Quality control in sequencing
//!
//! # Algorithm
//!
//! **Simple counting**: Count G and C bases, divide by total
//! - Input: DNA/RNA sequence (bytes: A, T/U, G, C)
//! - Output: GC percentage (0.0-1.0)
//! - Formula: (G_count + C_count) / total_length
//!
//! # Example
//!
//! ```no_run
//! use barracuda::gc_content;
//! use barracuda::WgpuDevice;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let device = WgpuDevice::new().await?;
//!
//! let sequence = b"ATCGATCGATCG";  // 3 G's, 3 C's out of 12 = 50%
//! let gc = gc_content(&device.device, &device.queue, sequence).await?;
//! assert!((gc - 0.5).abs() < 0.01);
//! # Ok(())
//! # }
//! ```

use std::borrow::Cow;
use wgpu::util::DeviceExt;

use crate::error::{BarracudaError, Result as BarracudaResult};

/// Calculate GC content percentage
///
/// # Arguments
///
/// * `device` - The `wgpu` device
/// * `queue` - The `wgpu` queue  
/// * `sequence` - DNA/RNA sequence (ASCII: A, T, G, C, U)
///
/// # Returns
///
/// GC content as percentage (0.0-1.0)
///
/// # Errors
///
/// Returns `BarracudaError` if:
/// - Sequence is empty
/// - GPU execution fails
pub async fn gc_content(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sequence: &[u8],
) -> BarracudaResult<f32> {
    if sequence.is_empty() {
        return Err(BarracudaError::InvalidInput {
            message: "Sequence cannot be empty".to_string(),
        });
    }

    let n = sequence.len() as u32;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("GC Content Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("gc_content.wgsl"))),
    });

    // Convert u8 to u32 for GPU
    let sequence_u32: Vec<u32> = sequence.iter().map(|&x| x as u32).collect();

    let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Sequence Buffer"),
        contents: bytemuck::cast_slice(&sequence_u32),
        usage: wgpu::BufferUsages::STORAGE,
    });

    // Output: single counter buffer (atomic add for GC count) - must be initialized to 0
    let output_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("GC Count Buffer"),
        contents: bytemuck::bytes_of(&0u32),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
    }

    let params = Params {
        n,
    };

    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Params Buffer"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("GC Content Layout"),
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
        label: Some("GC Content Bind Group"),
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
        label: Some("GC Content Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("GC Content Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "gc_content",
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("GC Content Encoder"),
    });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("GC Content Pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups((n + 255) / 256, 1, 1);
    }

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: std::mem::size_of::<u32>() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, std::mem::size_of::<u32>() as u64);
    queue.submit(Some(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| { let _ = sender.send(result); });
    device.poll(wgpu::Maintain::Wait);
    receiver.await.map_err(|_| BarracudaError::ExecutionError { message: "Failed to receive buffer".to_string() })?
        .map_err(|e| BarracudaError::ExecutionError { message: format!("Buffer mapping failed: {:?}", e) })?;

    let data = buffer_slice.get_mapped_range();
    let gc_count: u32 = bytemuck::cast_slice(&data)[0];
    drop(data);
    staging_buffer.unmap();

    Ok(gc_count as f32 / n as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;

    #[tokio::test]
    async fn test_gc_content_basic() {
        let device = WgpuDevice::new().await.unwrap();
        let sequence = b"ATCGATCGATCG";  // 3 G's, 3 C's = 6/12 = 0.5
        let result = gc_content(&device.device, &device.queue, sequence).await.unwrap();
        assert!((result - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_gc_content_edge_cases() {
        let device = WgpuDevice::new().await.unwrap();
        let all_gc = b"GCGCGCGC";
        let result = gc_content(&device.device, &device.queue, all_gc).await.unwrap();
        assert!((result - 1.0).abs() < 0.01);

        let no_gc = b"ATATATAT";
        let result2 = gc_content(&device.device, &device.queue, no_gc).await.unwrap();
        assert!(result2 < 0.01);
    }

    #[tokio::test]
    async fn test_gc_content_boundary() {
        let device = WgpuDevice::new().await.unwrap();
        let single = b"G";
        let result = gc_content(&device.device, &device.queue, single).await.unwrap();
        assert!((result - 1.0).abs() < 0.01);

        let empty: &[u8] = b"";
        assert!(gc_content(&device.device, &device.queue, empty).await.is_err());
    }

    #[tokio::test]
    async fn test_gc_content_large_tensor() {
        let device = WgpuDevice::new().await.unwrap();
        let large: Vec<u8> = (0..10000).map(|i| match i % 4 {
            0 => b'A',
            1 => b'T',
            2 => b'G',
            3 => b'C',
            _ => unreachable!(),
        }).collect();
        let result = gc_content(&device.device, &device.queue, &large).await.unwrap();
        assert!((result - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_gc_content_precision() {
        let device = WgpuDevice::new().await.unwrap();
        let sequence = b"AAAGGGCCC";  // 6 GC out of 9 = 0.666...
        let result = gc_content(&device.device, &device.queue, sequence).await.unwrap();
        assert!(result >= 0.0 && result <= 1.0);
        assert!((result - 0.666).abs() < 0.01);
    }
}
