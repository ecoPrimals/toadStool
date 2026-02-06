//! FHE Modulus Switching Operation
//!
//! **Purpose**: Reduce ciphertext noise by switching to a smaller modulus
//!
//! **Algorithm**: Scale-and-round modulus reduction
//! - Noise reduction: ~log(q_old/q_new) bits
//! - Preserves plaintext (decrypt correctness maintained)
//! - Essential for leveled FHE schemes (BFV, BGV)
//!
//! **Deep Debt Compliance**:
//! - ✅ Pure Rust + WGSL (no unsafe)
//! - ✅ GPU-accelerated via compute shaders
//! - ✅ Numerically precise (exact rounding)
//! - ✅ Production-ready (full error handling)
//!
//! ## Mathematical Background
//!
//! Modulus switching converts a ciphertext under modulus q_old to modulus q_new:
//! ```text
//! ct_new = round((q_new / q_old) * ct_old) mod q_new
//! ```
//!
//! **Key Properties**:
//! 1. **Correctness**: Dec(ct_new, sk, q_new) = Dec(ct_old, sk, q_old)
//! 2. **Noise Reduction**: noise_new ≈ noise_old * (q_new / q_old)
//! 3. **Homomorphism**: Can continue operations under q_new
//!
//! **Use Cases**:
//! - **Noise Management**: Reduce accumulated noise before overflow
//! - **Leveled FHE**: Enable deeper circuits without bootstrapping
//! - **Bandwidth**: Smaller ciphertexts for network transmission
//!
//! ## Usage Example
//!
//! ```no_run
//! use barracuda::ops::fhe_modulus_switch::FheModulusSwitch;
//! use barracuda::Tensor;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Ciphertext under large modulus (e.g., after multiplication)
//! let ct = Tensor::from_u64_poly(&ciphertext, degree).await?;
//!
//! // Switch to smaller modulus for noise reduction
//! let q_old = 1152921504606584833u64; // 60-bit prime
//! let q_new = 288230376151711777u64;  // 58-bit prime (4x smaller)
//!
//! let switch_op = FheModulusSwitch::new(ct, degree, q_old, q_new)?;
//! let ct_new = switch_op.execute()?;
//!
//! // ct_new has same plaintext, but ~4x less noise
//! # Ok(())
//! # }
//! ```

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// FHE Modulus Switching operation
///
/// Scales ciphertext coefficients to smaller modulus while preserving plaintext.
pub struct FheModulusSwitch {
    input: Tensor,
    degree: u32,
    modulus_old: u64,
    modulus_new: u64,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl FheModulusSwitch {
    /// Create a new modulus switching operation
    ///
    /// **Parameters**:
    /// - `input`: Ciphertext polynomial (2*degree u32 values, u64 emulated)
    /// - `degree`: Polynomial degree (power of 2)
    /// - `modulus_old`: Current modulus
    /// - `modulus_new`: Target modulus (must be < modulus_old)
    ///
    /// **Returns**: FheModulusSwitch operation ready to execute
    ///
    /// **Errors**:
    /// - Invalid degree (not power of 2)
    /// - modulus_new >= modulus_old
    /// - Input tensor size mismatch
    pub fn new(
        input: Tensor,
        degree: u32,
        modulus_old: u64,
        modulus_new: u64,
    ) -> Result<Self> {
        // ✅ VALIDATION: Degree must be power of 2
        if !degree.is_power_of_two() || degree < 4 {
            return Err(BarracudaError::InvalidInput {
                message: format!("Degree must be power of 2 >= 4, got {}", degree),
            });
        }

        // ✅ VALIDATION: New modulus must be smaller
        if modulus_new >= modulus_old {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "New modulus ({}) must be < old modulus ({})",
                    modulus_new, modulus_old
                ),
            });
        }

        // ✅ VALIDATION: Input tensor must be 2*degree (u64 as 2xu32)
        let expected_size = (degree * 2) as usize;
        if input.shape()[0] != expected_size {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Input must have {} elements (degree={}, u64 emulated), got {}",
                    expected_size, degree, input.shape()[0]
                ),
            });
        }

        let device = input.device();

        // Load WGSL shader
        let shader_source = include_str!("fhe_modulus_switch.wgsl");
        let shader_module = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("FHE Modulus Switch Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("FHE Modulus Switch Bind Group Layout"),
                    entries: &[
                        // Input buffer
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
                        // Output buffer
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
                        // Parameters buffer
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

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("FHE Modulus Switch Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("FHE Modulus Switch Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "modulus_switch",
            });

        Ok(Self {
            input,
            degree,
            modulus_old,
            modulus_new,
            pipeline,
            bind_group_layout,
        })
    }

    /// Execute modulus switching on GPU
    ///
    /// **Returns**: Tensor with coefficients scaled to new modulus
    ///
    /// **Performance**: O(n) GPU parallel execution
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();

        // Create output buffer
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FHE Modulus Switch Output"),
            size: self.input.buffer().size(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create parameters buffer
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct SwitchParams {
            degree: u32,
            modulus_old_lo: u32,
            modulus_old_hi: u32,
            modulus_new_lo: u32,
            modulus_new_hi: u32,
            _padding: [u32; 3], // Align to 16 bytes
        }

        let params = SwitchParams {
            degree: self.degree,
            modulus_old_lo: (self.modulus_old & 0xFFFFFFFF) as u32,
            modulus_old_hi: (self.modulus_old >> 32) as u32,
            modulus_new_lo: (self.modulus_new & 0xFFFFFFFF) as u32,
            modulus_new_hi: (self.modulus_new >> 32) as u32,
            _padding: [0; 3],
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("FHE Modulus Switch Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FHE Modulus Switch Bind Group"),
            layout: &self.bind_group_layout,
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

        // ✅ GPU EXECUTION: Parallel modulus switching
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("FHE Modulus Switch Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("FHE Modulus Switch Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(&device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::FHE);
            let workgroups = (self.degree + optimal_wg_size - 1) / optimal_wg_size;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(std::iter::once(encoder.finish()));

        // Return result tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            self.input.shape().to_vec(),
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_modulus_switch_validation() {
        // Test invalid degree
        let result = FheModulusSwitch::new(
            Tensor::zeros(vec![8]).await.unwrap(),
            3, // Not power of 2
            12289,
            6145,
        );
        assert!(result.is_err());

        // Test new modulus >= old modulus
        let result = FheModulusSwitch::new(
            Tensor::zeros(vec![8]).await.unwrap(),
            4,
            12289,
            12289, // Equal (should fail)
        );
        assert!(result.is_err());
    }

    // NOTE: Full integration tests require GPU + encryption setup
    // See examples/fhe_modulus_switch_validation.rs for round-trip tests
}
