//! Leaky Integrate-and-Fire (LIF) neuron operation
//!
//! Implements a bio-inspired spiking neuron model for neuromorphic computing.
//! Neurons accumulate input current and fire spikes when threshold is reached.
//!
//! # Neuromorphic Computing
//!
//! LIF neurons are the foundation of spiking neural networks (SNNs). They model
//! the behavior of biological neurons through integration and thresholding.
//!
//! # Model Dynamics
//!
//! - **Integration**: Membrane potential accumulates input current with leak
//! - **Threshold**: Spike fires when potential exceeds threshold
//! - **Reset**: Potential resets after spike
//! - **Leak**: Potential decays exponentially (time constant tau)
//!
//! # Example
//!
//! ```no_run
//! use barracuda::lif_neuron;
//! use barracuda::WgpuDevice;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let device = WgpuDevice::new().await?;
//!
//! // Simulate LIF neuron receiving input current
//! let input_current = vec![0.5, 0.8, 1.2, 0.3];  // Input per time step
//! let tau = 10.0;           // Time constant (ms)
//! let threshold = 1.0;      // Spike threshold
//! let reset = 0.0;          // Reset potential
//! let dt = 1.0;             // Time step (ms)
//!
//! let (potential, spikes) = lif_neuron(
//!     &device.device,
//!     &device.queue,
//!     &input_current,
//!     tau,
//!     threshold,
//!     reset,
//!     dt,
//! ).await?;
//!
//! // Output: membrane potential trace + spike times
//! # Ok(())
//! # }
//! ```

use std::borrow::Cow;
use wgpu::util::DeviceExt;

use crate::error::{BarracudaError, Result as BarracudaResult};

/// Simulate leaky integrate-and-fire neuron
///
/// # Arguments
///
/// * `device` - The `wgpu` device
/// * `queue` - The `wgpu` queue  
/// * `input_current` - Input current per time step
/// * `tau` - Membrane time constant (ms)
/// * `threshold` - Spike threshold
/// * `reset` - Reset potential
/// * `dt` - Time step (ms)
///
/// # Returns
///
/// Tuple of (membrane_potential, spike_flags)
/// - membrane_potential: Potential at each time step
/// - spike_flags: 1.0 where spike occurred, 0.0 otherwise
///
/// # Errors
///
/// Returns `BarracudaError` if:
/// - Input is empty
/// - Tau or dt is zero/negative
/// - GPU execution fails
pub async fn lif_neuron(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    input_current: &[f32],
    tau: f32,
    threshold: f32,
    reset: f32,
    dt: f32,
) -> BarracudaResult<(Vec<f32>, Vec<f32>)> {
    if input_current.is_empty() {
        return Err(BarracudaError::InvalidInput {
            message: "Input current cannot be empty".to_string(),
        });
    }

    if tau <= 0.0 || dt <= 0.0 {
        return Err(BarracudaError::InvalidInput {
            message: "Tau and dt must be positive".to_string(),
        });
    }

    let n = input_current.len() as u32;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("LIF Neuron Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("lif_neuron.wgsl"))),
    });

    let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("LIF Input Buffer"),
        contents: bytemuck::cast_slice(input_current),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let potential_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("LIF Potential Buffer"),
        size: (n * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let spikes_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("LIF Spikes Buffer"),
        size: (n * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
        tau: f32,
        threshold: f32,
        reset: f32,
        dt: f32,
    }

    let params = Params {
        n,
        tau,
        threshold,
        reset,
        dt,
    };

    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("LIF Params Buffer"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("LIF Bind Group Layout"),
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
        label: Some("LIF Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: potential_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: spikes_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("LIF Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("LIF Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "lif_neuron",
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("LIF Encoder"),
    });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("LIF Pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups(1, 1, 1); // Single neuron simulation (sequential)
    }

    // Read back both outputs
    let potential_staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Potential Staging"),
        size: (n * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let spikes_staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Spikes Staging"),
        size: (n * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(
        &potential_buffer,
        0,
        &potential_staging,
        0,
        (n * std::mem::size_of::<f32>() as u32) as u64,
    );

    encoder.copy_buffer_to_buffer(
        &spikes_buffer,
        0,
        &spikes_staging,
        0,
        (n * std::mem::size_of::<f32>() as u32) as u64,
    );

    queue.submit(Some(encoder.finish()));

    // Read potential
    let pot_slice = potential_staging.slice(..);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    pot_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    receiver
        .await
        .map_err(|_| BarracudaError::ExecutionError {
            message: "Failed to receive potential buffer".to_string(),
        })?
        .map_err(|e| BarracudaError::ExecutionError {
            message: format!("Potential buffer mapping failed: {:?}", e),
        })?;

    let pot_data = pot_slice.get_mapped_range();
    let potential: Vec<f32> = bytemuck::cast_slice(&pot_data).to_vec();
    drop(pot_data);
    potential_staging.unmap();

    // Read spikes
    let spike_slice = spikes_staging.slice(..);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    spike_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    receiver
        .await
        .map_err(|_| BarracudaError::ExecutionError {
            message: "Failed to receive spikes buffer".to_string(),
        })?
        .map_err(|e| BarracudaError::ExecutionError {
            message: format!("Spikes buffer mapping failed: {:?}", e),
        })?;

    let spike_data = spike_slice.get_mapped_range();
    let spikes: Vec<f32> = bytemuck::cast_slice(&spike_data).to_vec();
    drop(spike_data);
    spikes_staging.unmap();

    Ok((potential, spikes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;

    #[tokio::test]
    async fn test_lif_neuron_basic() {
        let device = WgpuDevice::new().await.unwrap();
        // Use stronger input (5.0) to ensure spiking with tau=10.0, threshold=1.0
        let input = vec![5.0; 20];
        let (potential, spikes) =
            lif_neuron(&device.device, &device.queue, &input, 10.0, 1.0, 0.0, 1.0)
                .await
                .unwrap();
        assert_eq!(potential.len(), input.len());
        assert!(potential.iter().all(|&x| x.is_finite()));
        let spike_count = spikes.iter().filter(|&&x| x > 0.5).count();
        assert!(
            spike_count >= 5,
            "Expected at least 5 spikes, got {}",
            spike_count
        );
    }

    #[tokio::test]
    async fn test_lif_neuron_edge_cases() {
        let device = WgpuDevice::new().await.unwrap();
        // Test with no input - should not spike
        let zeros = vec![0.0; 100];
        let (potential, spikes) =
            lif_neuron(&device.device, &device.queue, &zeros, 10.0, 1.0, 0.0, 1.0)
                .await
                .unwrap();
        assert!(potential.iter().all(|&x| x.is_finite()));
        assert!(spikes.iter().all(|&x| x == 0.0 || x == 1.0));
        assert_eq!(
            spikes.iter().filter(|&&x| x > 0.5).count(),
            0,
            "No spikes expected with zero input"
        );

        // Test with strong input - should spike frequently
        let large = vec![15.0; 50];
        let (_potential, spikes) =
            lif_neuron(&device.device, &device.queue, &large, 10.0, 1.0, 0.0, 1.0)
                .await
                .unwrap();
        let spike_count = spikes.iter().filter(|&&x| x > 0.5).count();
        assert!(
            spike_count >= 10,
            "Expected at least 10 spikes, got {}",
            spike_count
        );
    }

    #[tokio::test]
    async fn test_lif_neuron_boundary() {
        let device = WgpuDevice::new().await.unwrap();
        let input = vec![0.5];
        let empty: Vec<f32> = vec![];
        let result = lif_neuron(&device.device, &device.queue, &empty, 10.0, 1.0, 0.0, 1.0).await;
        assert!(result.is_err());
        let result = lif_neuron(&device.device, &device.queue, &input, 0.0, 1.0, 0.0, 1.0).await;
        assert!(result.is_err());
        let result = lif_neuron(&device.device, &device.queue, &input, 10.0, 1.0, 0.0, -1.0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_lif_neuron_large_tensor() {
        let device = WgpuDevice::new().await.unwrap();
        // Alternate between strong and weak input
        let large_input: Vec<f32> = (0..1000)
            .map(|i| if i % 100 < 50 { 8.0 } else { 0.5 })
            .collect();
        let (potential, spikes) = lif_neuron(
            &device.device,
            &device.queue,
            &large_input,
            10.0,
            1.0,
            0.0,
            1.0,
        )
        .await
        .unwrap();
        assert_eq!(potential.len(), 1000);
        assert!(potential.iter().all(|&x| x.is_finite()));
        assert!(spikes.iter().all(|&x| x == 0.0 || x == 1.0));
        // With strong input periods, expect some spikes
        let spike_count = spikes.iter().filter(|&&x| x > 0.5).count();
        assert!(
            spike_count > 0,
            "Expected some spikes in large tensor, got {}",
            spike_count
        );
    }

    #[tokio::test]
    async fn test_lif_neuron_precision() {
        let device = WgpuDevice::new().await.unwrap();
        // Use strong initial current followed by rest
        let input = vec![8.0, 8.0, 8.0, 0.0, 0.0];
        let (potential, spikes) =
            lif_neuron(&device.device, &device.queue, &input, 10.0, 1.0, 0.0, 1.0)
                .await
                .unwrap();
        assert!(potential.iter().all(|&x| x.is_finite() && x >= 0.0));
        assert!(spikes.iter().all(|&x| x == 0.0 || x == 1.0));
        let spike_count = spikes.iter().filter(|&&x| x > 0.5).count();
        assert!(
            spike_count >= 1,
            "Expected at least 1 spike, got {}",
            spike_count
        );
    }
}
