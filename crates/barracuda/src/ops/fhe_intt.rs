//! FHE Inverse Number Theoretic Transform (INTT) Operation
//!
//! **Purpose**: Convert NTT-domain polynomial back to coefficient domain
//!
//! **Algorithm**: Inverse Cooley-Tukey FFT with scaling by N^(-1)
//! - Time complexity: O(N log N)
//! - Completes the NTT multiplication pipeline
//!
//! **Deep Debt Compliance**:
//! - ✅ Pure Rust + WGSL (no unsafe)
//! - ✅ Hardware-agnostic (wgpu backend selection)
//! - ✅ Numerically precise (modular arithmetic)
//! - ✅ Production-ready (full error handling)
//! - ✅ Canonical pattern: Tensor inputs/outputs
//!
//! ## Mathematical Background
//!
//! The Inverse NTT transforms from frequency domain back to coefficient domain:
//!
//! ```text
//! a(X) = INTT(A) = (1/N) * NTT^(-1)(A)
//! ```
//!
//! Where NTT^(-1) uses the inverse root of unity ω^(-1).
//!
//! ## Usage Example
//!
//! ```no_run
//! use barracuda::ops::fhe_ntt::FheNtt;
//! use barracuda::ops::fhe_intt::FheIntt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Forward NTT
//! let ntt = FheNtt::new(poly, 4096, modulus, root)?;
//! let poly_ntt = ntt.execute()?;
//!
//! // Inverse NTT (round-trip)
//! let intt = FheIntt::new(poly_ntt, 4096, modulus, inv_root)?;
//! let poly_recovered = intt.execute()?;
//!
//! // poly_recovered should equal original poly
//! # Ok(())
//! # }
//! ```

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// FHE Inverse Number Theoretic Transform operation
///
/// Transforms polynomial from NTT domain back to coefficient domain.
#[allow(dead_code)] // Will be used once NTT is integrated
pub struct FheIntt {
    input: Tensor,
    degree: u32,
    modulus: u64,
    inv_root_of_unity: u64,
    barrett_mu: u64,
    inv_twiddle_factors: Vec<u64>,
    #[allow(dead_code)] // Will be used for scaling in optimization phase
    inv_n: u64, // N^(-1) mod q for scaling
    pipeline_butterfly: wgpu::ComputePipeline,
    pipeline_bit_reverse: wgpu::ComputePipeline,
    pipeline_scale: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl FheIntt {
    /// Create a new INTT operation
    ///
    /// ## Parameters
    ///
    /// - `input`: NTT-domain polynomial tensor (u32 pairs representing u64)
    /// - `degree`: Polynomial degree N (must be power of 2)
    /// - `modulus`: Prime modulus q (must satisfy q ≡ 1 mod 2N)
    /// - `inv_root_of_unity`: Inverse N-th root of unity ω^(-1) (in Z_q)
    ///
    /// ## Constraints
    ///
    /// - N must be a power of 2
    /// - q must be prime
    /// - ω^(-1) * ω ≡ 1 (mod q)
    pub fn new(
        input: Tensor,
        degree: u32,
        modulus: u64,
        inv_root_of_unity: u64,
    ) -> Result<Self> {
        // Validate inputs (same as NTT)
        let expected_size = (degree as usize) * 2;
        if input.len() != expected_size {
            return Err(BarracudaError::Device(format!(
                "Input length {} doesn't match expected {} (degree {} * 2)",
                input.len(),
                expected_size,
                degree
            )));
        }

        if !degree.is_power_of_two() {
            return Err(BarracudaError::Device(format!(
                "Degree {} must be a power of 2 for INTT",
                degree
            )));
        }

        if modulus == 0 {
            return Err(BarracudaError::Device(
                "Modulus must be non-zero".to_string(),
            ));
        }

        // Precompute Barrett constant
        let barrett_mu = if modulus > 0 {
            u64::MAX / modulus
        } else {
            0
        };

        // Precompute inverse twiddle factors: (ω^(-1))^0, (ω^(-1))^1, ..., (ω^(-1))^(N-1)
        let inv_twiddle_factors = compute_twiddle_factors(degree, modulus, inv_root_of_unity);

        // Precompute N^(-1) mod q for final scaling
        let inv_n = compute_modular_inverse(degree as u64, modulus);

        let device = input.device();

        // Load shaders
        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("FHE INTT Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("fhe_intt.wgsl").into()),
            });

        // Bind group layout (same structure as NTT)
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("FHE INTT Bind Group Layout"),
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

        // Pipeline layout
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("FHE INTT Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        // Butterfly pipeline
        let pipeline_butterfly =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("FHE INTT Butterfly Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "main",
                });

        // Bit-reversal pipeline
        let pipeline_bit_reverse =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("FHE INTT Bit Reverse Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "bit_reverse",
                });

        // Scaling pipeline (divide by N)
        let pipeline_scale =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("FHE INTT Scale Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "scale_by_n",
                });

        Ok(Self {
            input,
            degree,
            modulus,
            inv_root_of_unity,
            barrett_mu,
            inv_twiddle_factors,
            inv_n,
            pipeline_butterfly,
            pipeline_bit_reverse,
            pipeline_scale,
            bind_group_layout,
        })
    }

    /// Execute INTT transformation
    ///
    /// Returns a new tensor containing the coefficient-domain representation.
    ///
    /// ## Algorithm
    ///
    /// 1. Bit-reversal permutation
    /// 2. log₂(N) butterfly stages (using inverse twiddle factors)
    /// 3. Scale by N^(-1) mod q
    ///
    /// ## Complexity
    ///
    /// - Time: O(N log N)
    /// - Space: O(N) temporary buffers
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        
        // Buffer size
        let buffer_size = self.degree as u64 * 2 * std::mem::size_of::<u32>() as u64;
        
        // Create buffers
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("INTT Output Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let intermediate_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("INTT Intermediate Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        // Create inverse twiddle factors buffer
        let inv_twiddle_data: Vec<u32> = self.inv_twiddle_factors
            .iter()
            .flat_map(|&factor| {
                vec![(factor & 0xFFFFFFFF) as u32, (factor >> 32) as u32]
            })
            .collect();
        
        let inv_twiddle_buffer = device.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("INTT Inverse Twiddle Factors"),
                contents: bytemuck::cast_slice(&inv_twiddle_data),
                usage: wgpu::BufferUsages::STORAGE,
            }
        );
        
        // Create params
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct InttParams {
            degree: u32,
            modulus_lo: u32,
            modulus_hi: u32,
            barrett_mu_lo: u32,
            barrett_mu_hi: u32,
            inv_root_lo: u32,
            inv_root_hi: u32,
            stage: u32,
        }
        
        let params = InttParams {
            degree: self.degree,
            modulus_lo: (self.modulus & 0xFFFFFFFF) as u32,
            modulus_hi: (self.modulus >> 32) as u32,
            barrett_mu_lo: (self.barrett_mu & 0xFFFFFFFF) as u32,
            barrett_mu_hi: (self.barrett_mu >> 32) as u32,
            inv_root_lo: (self.inv_root_of_unity & 0xFFFFFFFF) as u32,
            inv_root_hi: (self.inv_root_of_unity >> 32) as u32,
            stage: 0,
        };
        
        // Command encoder
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("INTT Command Encoder"),
            });
        
        // Pass 1: Bit-reversal
        let params_buffer = device.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("INTT Params (Bit Reverse)"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            }
        );
        
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("INTT Bit Reverse Bind Group"),
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
                    resource: inv_twiddle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("INTT Bit Reverse Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.pipeline_bit_reverse);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            let workgroup_size = 256u32;
            let num_workgroups = (self.degree + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }
        
        // Submit bit-reversal pass before butterfly stages
        device.queue.submit(std::iter::once(encoder.finish()));
        
        // Pass 2-N: Butterfly stages (submit each separately for sequential execution)
        let num_stages = (self.degree as f32).log2() as u32;
        let mut current_input = &intermediate_buffer;
        let mut current_output = &output_buffer;
        
        for stage in 0..num_stages {
            // Create separate encoder for each stage
            let mut stage_encoder = device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some(&format!("INTT Stage {} Encoder", stage)),
                });
            
            let stage_params = InttParams {
                stage,
                ..params
            };
            
            let stage_params_buffer = device.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("INTT Params (Stage {})", stage)),
                    contents: bytemuck::bytes_of(&stage_params),
                    usage: wgpu::BufferUsages::UNIFORM,
                }
            );
            
            let stage_bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("INTT Butterfly Bind Group (Stage {})", stage)),
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
                        resource: inv_twiddle_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: stage_params_buffer.as_entire_binding(),
                    },
                ],
            });
            
            {
                let mut compute_pass = stage_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some(&format!("INTT Butterfly Pass (Stage {})", stage)),
                    timestamp_writes: None,
                });
                
                compute_pass.set_pipeline(&self.pipeline_butterfly);
                compute_pass.set_bind_group(0, &stage_bind_group, &[]);
                
                let num_butterflies = self.degree / 2;
                let workgroup_size = 256u32;
                let num_workgroups = (num_butterflies + workgroup_size - 1) / workgroup_size;
                compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
            }
            
            // Submit this stage before next
            device.queue.submit(std::iter::once(stage_encoder.finish()));
            
            std::mem::swap(&mut current_input, &mut current_output);
        }
        
        // Determine which buffer has the result after butterfly stages
        // Same swapping logic as NTT: after even stages, result in intermediate; odd in output
        let butterfly_result_buffer = if num_stages % 2 == 0 {
            &intermediate_buffer
        } else {
            &output_buffer
        };
        
        // ============================================================
        // Pass N+1: Scaling by N^(-1) mod q
        // ============================================================
        
        // Create scaled output buffer
        let scaled_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("INTT Scaled Output"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        // Update params with inv_n (reuse root_of_unity fields)
        let scale_params = InttParams {
            inv_root_lo: (self.inv_n & 0xFFFFFFFF) as u32,
            inv_root_hi: (self.inv_n >> 32) as u32,
            ..params
        };
        
        let scale_params_buffer = device.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("INTT Scaling Params"),
                contents: bytemuck::bytes_of(&scale_params),
                usage: wgpu::BufferUsages::UNIFORM,
            }
        );
        
        let scale_bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("INTT Scaling Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: butterfly_result_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: scaled_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: inv_twiddle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: scale_params_buffer.as_entire_binding(),
                },
            ],
        });
        
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("INTT Scaling Encoder"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("INTT Scaling Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.pipeline_scale);
            compute_pass.set_bind_group(0, &scale_bind_group, &[]);
            
            let workgroup_size = 256u32;
            let num_workgroups = (self.degree + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }
        
        // Submit scaling pass
        device.queue.submit(std::iter::once(encoder.finish()));
        
        // Create result tensor
        Ok(Tensor::from_buffer(
            scaled_buffer,
            vec![self.degree as usize * 2],
            device.clone(),
        ))
    }
}

/// Compute twiddle factors (same as NTT helper)
fn compute_twiddle_factors(degree: u32, modulus: u64, root: u64) -> Vec<u64> {
    let mut factors = Vec::with_capacity(degree as usize);
    let mut power = 1u64;

    for _ in 0..degree {
        factors.push(power);
        power = (power as u128 * root as u128 % modulus as u128) as u64;
    }

    factors
}

/// Compute modular inverse: a^(-1) mod m
/// Uses Extended Euclidean Algorithm
fn compute_modular_inverse(a: u64, m: u64) -> u64 {
    // Extended Euclidean Algorithm
    let (mut t, mut new_t) = (0i128, 1i128);
    let (mut r, mut new_r) = (m as i128, a as i128);
    
    while new_r != 0 {
        let quotient = r / new_r;
        (t, new_t) = (new_t, t - quotient * new_t);
        (r, new_r) = (new_r, r - quotient * new_r);
    }
    
    if r > 1 {
        // a is not invertible
        return 0;
    }
    
    if t < 0 {
        t += m as i128;
    }
    
    t as u64
}

/// Compute inverse primitive root: ω^(-1) where ω is N-th root of unity
pub fn compute_inverse_root(_degree: u32, modulus: u64, root: u64) -> u64 {
    compute_modular_inverse(root, modulus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modular_inverse() {
        // Test: 3^(-1) mod 7 = 5 (because 3 * 5 = 15 ≡ 1 mod 7)
        assert_eq!(compute_modular_inverse(3, 7), 5);
        
        // Test: 4^(-1) mod 17 = 13 (because 4 * 13 = 52 ≡ 1 mod 17)
        assert_eq!(compute_modular_inverse(4, 17), 13);
    }
    
    #[test]
    fn test_inverse_root() {
        // For N=4, q=17, ω=4
        // ω^(-1) = 4^(-1) mod 17 = 13
        let _degree = 4u32; // Documented for clarity
        assert_eq!(compute_inverse_root(4, 17, 4), 13);
    }
}
