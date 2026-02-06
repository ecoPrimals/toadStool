//! AdaDelta Optimizer - GPU-accelerated adaptive learning rate optimizer
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (existing shader evolved)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready)
//! - ✅ Modern idiomatic Rust (no traits, direct impl)
//! - ✅ Capability-based dispatch (vendor-optimized workgroups)
//!
//! ## Algorithm
//!
//! ```text
//! E[g²] = ρ * E[g²] + (1 - ρ) * g²
//! RMS[g] = sqrt(E[g²] + ε)
//! RMS[Δ] = sqrt(E[Δ²] + ε)
//! Δw = -(RMS[Δ] / RMS[g]) * g
//! w = w + Δw
//! E[Δ²] = ρ * E[Δ²] + (1 - ρ) * Δw²
//! ```
//!
//! **Key Properties**:
//! - No learning rate hyperparameter needed!
//! - Adapts learning rate per parameter
//! - More stable than AdaGrad (doesn't monotonically decrease)
//! - Uses moving average of gradients and updates
//!
//! **Parameters**:
//! - `rho` (ρ): Decay rate for moving averages, typically 0.95
//! - `epsilon` (ε): Numerical stability constant, typically 1e-6
//!
//! **Used By**: When you want to avoid tuning learning rates
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let weights = Tensor::randn(vec![1000]).await?;
//! let gradients = Tensor::randn(vec![1000]).await?;
//!
//! // First step (no accumulated state)
//! let (updated_weights, acc_grad, acc_delta) =
//!     weights.adadelta_step(&gradients, 0.95, None, None)?;
//!
//! // Subsequent steps (with accumulated state)
//! let (updated_weights2, acc_grad2, acc_delta2) =
//!     updated_weights.adadelta_step(
//!         &gradients,
//!         0.95,
//!         Some(&acc_grad),
//!         Some(&acc_delta),
//!     )?;
//! ```

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AdaDeltaParams {
    rho: f32,
    epsilon: f32,
    weight_decay: f32,
    _padding: u32,
}

pub struct AdaDelta {
    weights: Tensor,
    gradients: Tensor,
    acc_grad: Option<Tensor>,
    acc_delta: Option<Tensor>,
    rho: f32,
}

impl AdaDelta {
    pub fn new(
        weights: Tensor,
        gradients: Tensor,
        rho: f32,
        acc_grad: Option<Tensor>,
        acc_delta: Option<Tensor>,
    ) -> Result<Self> {
        // Validate shapes match
        if weights.shape() != gradients.shape() {
            return Err(BarracudaError::shape_mismatch(
                weights.shape().to_vec(),
                gradients.shape().to_vec(),
            ));
        }

        // Validate rho in valid range
        if !(0.0..=1.0).contains(&rho) {
            return Err(BarracudaError::invalid_op(
                "adadelta",
                "rho must be in range [0.0, 1.0]",
            ));
        }

        // Validate accumulator shapes if provided
        if let Some(ref ag) = acc_grad {
            if ag.shape() != weights.shape() {
                return Err(BarracudaError::shape_mismatch(
                    ag.shape().to_vec(),
                    weights.shape().to_vec(),
                ));
            }
        }

        if let Some(ref ad) = acc_delta {
            if ad.shape() != weights.shape() {
                return Err(BarracudaError::shape_mismatch(
                    ad.shape().to_vec(),
                    weights.shape().to_vec(),
                ));
            }
        }

        Ok(Self {
            weights,
            gradients,
            acc_grad,
            acc_delta,
            rho,
        })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/adadelta.wgsl")
    }

    pub fn execute(self) -> Result<(Tensor, Tensor, Tensor)> {
        let device = self.weights.device();
        let size = self.weights.shape().iter().product::<usize>();

        let params = AdaDeltaParams {
            rho: self.rho,
            epsilon: 1e-6,
            weight_decay: 0.0,
            _padding: 0,
        };

        // Create state buffers if not provided
        let zeros = vec![0.0f32; size];
        let acc_grad_in = if let Some(ref tensor) = self.acc_grad {
            tensor.buffer()
        } else {
            &device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("adadelta_acc_grad_zeros"),
                    contents: bytemuck::cast_slice(&zeros),
                    usage: wgpu::BufferUsages::STORAGE,
                })
        };

        let acc_delta_in = if let Some(ref tensor) = self.acc_delta {
            tensor.buffer()
        } else {
            &device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("adadelta_acc_delta_zeros"),
                    contents: bytemuck::cast_slice(&zeros),
                    usage: wgpu::BufferUsages::STORAGE,
                })
        };

        let weights_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adadelta_weights_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let acc_grad_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adadelta_acc_grad_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let acc_delta_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adadelta_acc_delta_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("adadelta_params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("adadelta_shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("adadelta_bind_group_layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 7,
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("adadelta_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("adadelta_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("adadelta_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.weights.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.gradients.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: acc_grad_in.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: acc_delta_in.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: weights_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: acc_grad_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: acc_delta_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("adadelta_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("adadelta_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(&device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = ((size as u32) + optimal_wg_size - 1) / optimal_wg_size;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        let updated_weights = Tensor::from_buffer(
            weights_out_buffer,
            self.weights.shape().to_vec(),
            device.clone(),
        );

        let updated_acc_grad = Tensor::from_buffer(
            acc_grad_out_buffer,
            self.weights.shape().to_vec(),
            device.clone(),
        );

        let updated_acc_delta = Tensor::from_buffer(
            acc_delta_out_buffer,
            self.weights.shape().to_vec(),
            device.clone(),
        );

        Ok((updated_weights, updated_acc_grad, updated_acc_delta))
    }
}

// ═══════════════════════════════════════════════════════════════
// TENSOR API INTEGRATION (MODERN IDIOMATIC RUST)
// ═══════════════════════════════════════════════════════════════

impl Tensor {
    /// AdaDelta optimizer step - adaptive learning rate without lr hyperparameter
    ///
    /// **Deep Debt**: Essential for training without tuning learning rates
    ///
    /// # Arguments
    /// - `gradients`: Gradient tensor [same shape as weights]
    /// - `rho`: Decay rate for moving averages, typically 0.95
    /// - `acc_grad`: Accumulated squared gradients (None for first step)
    /// - `acc_delta`: Accumulated squared deltas (None for first step)
    ///
    /// # Returns
    /// - Tuple: (updated_weights, updated_acc_grad, updated_acc_delta)
    ///
    /// # Example
    /// ```rust,ignore
    /// // First step
    /// let (w1, ag1, ad1) = weights.adadelta_step(&grads, 0.95, None, None)?;
    ///
    /// // Subsequent steps
    /// let (w2, ag2, ad2) = w1.adadelta_step(&grads, 0.95, Some(&ag1), Some(&ad1))?;
    /// ```
    ///
    /// # Note
    /// - No learning rate hyperparameter needed!
    /// - More stable than AdaGrad
    /// - rho should be in [0.0, 1.0], typically 0.95
    pub fn adadelta_step(
        self,
        gradients: &Self,
        rho: f32,
        acc_grad: Option<&Self>,
        acc_delta: Option<&Self>,
    ) -> Result<(Self, Self, Self)> {
        AdaDelta::new(
            self,
            gradients.clone(),
            rho,
            acc_grad.cloned(),
            acc_delta.cloned(),
        )?
        .execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_adadelta_basic() {
        let device = get_test_device().await;

        let weights = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0], vec![4], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.1, 0.2, 0.3, 0.4], vec![4], device.clone())
            .await
            .unwrap();

        let (updated_weights, _acc_grad, _acc_delta) =
            weights.adadelta_step(&gradients, 0.95, None, None).unwrap();
        let result = updated_weights.to_vec().unwrap();

        // Weights should be updated
        assert_eq!(result.len(), 4);
        assert!(result.iter().all(|&x| x.is_finite()));
        // AdaDelta should decrease weights (gradient descent)
        assert!(
            result[0] < 1.0,
            "Expected result[0] < 1.0, got {}",
            result[0]
        );
    }

    #[tokio::test]
    async fn test_adadelta_zero_gradients() {
        let device = get_test_device().await;

        let weights = Tensor::from_vec_on(vec![1.0, 2.0], vec![2], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.0, 0.0], vec![2], device.clone())
            .await
            .unwrap();

        let (updated_weights, acc_grad, acc_delta) =
            weights.adadelta_step(&gradients, 0.95, None, None).unwrap();
        let result = updated_weights.to_vec().unwrap();
        let ag = acc_grad.to_vec().unwrap();
        let ad = acc_delta.to_vec().unwrap();

        assert!(result.iter().all(|&x| x.is_finite()));
        assert!(ag.iter().all(|&x| x.is_finite()));
        assert!(ad.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adadelta_different_rho() {
        let device = get_test_device().await;

        let weights1 = Tensor::from_vec_on(vec![1.0; 4], vec![4], device.clone())
            .await
            .unwrap();

        let weights2 = Tensor::from_vec_on(vec![1.0; 4], vec![4], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.1; 4], vec![4], device.clone())
            .await
            .unwrap();

        // Low rho (less momentum)
        let (updated1, _ag, _ad) = weights1.adadelta_step(&gradients, 0.5, None, None).unwrap();

        // High rho (more momentum)
        let (updated2, _ag, _ad) = weights2
            .adadelta_step(&gradients, 0.99, None, None)
            .unwrap();

        let result1 = updated1.to_vec().unwrap();
        let result2 = updated2.to_vec().unwrap();

        assert!(result1.iter().all(|&x| x.is_finite()));
        assert!(result2.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adadelta_validation() {
        let device = get_test_device().await;

        let weights = Tensor::from_vec_on(vec![1.0; 10], vec![10], device.clone())
            .await
            .unwrap();
        let gradients = Tensor::from_vec_on(vec![0.1; 5], vec![5], device.clone())
            .await
            .unwrap();

        // Shape mismatch
        assert!(weights
            .clone()
            .adadelta_step(&gradients, 0.95, None, None)
            .is_err());

        // Invalid rho
        let gradients_correct = Tensor::from_vec_on(vec![0.1; 10], vec![10], device.clone())
            .await
            .unwrap();
        assert!(weights
            .clone()
            .adadelta_step(&gradients_correct, -0.1, None, None)
            .is_err());
        assert!(weights
            .clone()
            .adadelta_step(&gradients_correct, 1.5, None, None)
            .is_err());
    }

    #[tokio::test]
    async fn test_adadelta_large_batch() {
        let device = get_test_device().await;

        let size = 128;
        let weights_data: Vec<f32> = (0..size).map(|i| (i as f32) / 10.0).collect();
        let grads_data = vec![0.01; size];

        let weights = Tensor::from_vec_on(weights_data, vec![size], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(grads_data, vec![size], device.clone())
            .await
            .unwrap();

        let (updated_weights, updated_ag, updated_ad) =
            weights.adadelta_step(&gradients, 0.95, None, None).unwrap();

        let result = updated_weights.to_vec().unwrap();
        let ag = updated_ag.to_vec().unwrap();
        let ad = updated_ad.to_vec().unwrap();

        assert_eq!(result.len(), size);
        assert_eq!(ag.len(), size);
        assert_eq!(ad.len(), size);
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adadelta_multi_step() {
        let device = get_test_device().await;

        let weights = Tensor::from_vec_on(vec![10.0, 20.0], vec![2], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![1.0, 2.0], vec![2], device.clone())
            .await
            .unwrap();

        // Step 1
        let (weights1, ag1, ad1) = weights.adadelta_step(&gradients, 0.95, None, None).unwrap();
        let result1 = weights1.to_vec().unwrap();

        assert!(result1[0] < 10.0, "Expected descent, got {}", result1[0]);
        assert!(result1[1] < 20.0, "Expected descent, got {}", result1[1]);

        // Step 2 with accumulated state
        let (weights2, _ag2, _ad2) = weights1
            .adadelta_step(&gradients, 0.95, Some(&ag1), Some(&ad1))
            .unwrap();
        let result2 = weights2.to_vec().unwrap();

        // Should continue optimizing
        assert!(result2.iter().all(|&x| x.is_finite()));
        assert!(result2[0] < 10.0);
        assert!(result2[1] < 20.0);
    }
}
