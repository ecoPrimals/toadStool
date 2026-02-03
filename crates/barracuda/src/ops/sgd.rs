//! SGD Optimizer - GPU-accelerated Stochastic Gradient Descent
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (existing shader evolved)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready)
//! - ✅ Modern idiomatic Rust (no traits, direct impl)
//!
//! ## Algorithm
//!
//! ```text
//! Without momentum:
//! w = w - lr * (g + weight_decay * w)
//!
//! With momentum:
//! v = momentum * v + g
//! w = w - lr * v
//! ```
//!
//! **Key Properties**:
//! - Foundation optimizer for deep learning
//! - Optional momentum for faster convergence
//! - Optional weight decay for regularization
//! - Simple and robust
//!
//! **Parameters**:
//! - `learning_rate`: Step size, typically 0.01-0.1
//! - `momentum`: Momentum factor, typically 0.9 (0.0 = no momentum)
//! - `weight_decay`: L2 regularization, typically 0.0001-0.001
//!
//! **Used By**: All deep learning training (foundational optimizer)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let weights = Tensor::randn(vec![1000]).await?;
//! let gradients = Tensor::randn(vec![1000]).await?;
//!
//! // Without momentum
//! let (updated_weights, _) =
//!     weights.sgd_step(&gradients, 0.01, 0.0, 0.0, None)?;
//!
//! // With momentum
//! let (w1, v1) = weights.sgd_step(&gradients, 0.01, 0.9, 0.0, None)?;
//! let (w2, v2) = w1.sgd_step(&gradients, 0.01, 0.9, 0.0, v1.as_ref())?;
//! ```

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SGDParams {
    learning_rate: f32,
    momentum: f32,
    weight_decay: f32,
    dampening: f32,
}

pub struct SGD {
    weights: Tensor,
    gradients: Tensor,
    velocity: Option<Tensor>,
    learning_rate: f32,
    momentum: f32,
    weight_decay: f32,
}

impl SGD {
    pub fn new(
        weights: Tensor,
        gradients: Tensor,
        learning_rate: f32,
        momentum: f32,
        weight_decay: f32,
        velocity: Option<Tensor>,
    ) -> Result<Self> {
        // Validate shapes match
        if weights.shape() != gradients.shape() {
            return Err(BarracudaError::shape_mismatch(
                weights.shape().to_vec(),
                gradients.shape().to_vec(),
            ));
        }

        // Validate learning rate is positive
        if learning_rate <= 0.0 {
            return Err(BarracudaError::invalid_op(
                "sgd",
                "learning_rate must be positive",
            ));
        }

        // Validate momentum in valid range
        if !(0.0..=1.0).contains(&momentum) {
            return Err(BarracudaError::invalid_op(
                "sgd",
                "momentum must be in range [0.0, 1.0]",
            ));
        }

        // Validate weight_decay is non-negative
        if weight_decay < 0.0 {
            return Err(BarracudaError::invalid_op(
                "sgd",
                "weight_decay must be non-negative",
            ));
        }

        // Validate velocity shape if provided
        if let Some(ref v) = velocity {
            if v.shape() != weights.shape() {
                return Err(BarracudaError::shape_mismatch(
                    v.shape().to_vec(),
                    weights.shape().to_vec(),
                ));
            }
        }

        Ok(Self {
            weights,
            gradients,
            velocity,
            learning_rate,
            momentum,
            weight_decay,
        })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/sgd.wgsl")
    }

    pub fn execute(self) -> Result<(Tensor, Option<Tensor>)> {
        let device = self.weights.device();
        let size = self.weights.shape().iter().product::<usize>();

        let params = SGDParams {
            learning_rate: self.learning_rate,
            momentum: self.momentum,
            weight_decay: self.weight_decay,
            dampening: 0.0,
        };

        // Create velocity buffer if not provided
        let velocity_in = if let Some(ref v) = self.velocity {
            v.buffer()
        } else {
            let zeros = vec![0.0f32; size];
            &device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("sgd_velocity_zeros"),
                    contents: bytemuck::cast_slice(&zeros),
                    usage: wgpu::BufferUsages::STORAGE,
                })
        };

        let weights_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sgd_weights_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let velocity_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sgd_velocity_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sgd_params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("sgd_shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("sgd_bind_group_layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("sgd_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("sgd_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sgd_bind_group"),
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
                    resource: velocity_in.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: weights_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: velocity_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sgd_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("sgd_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let workgroups = ((size + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        let updated_weights = Tensor::from_buffer(
            weights_out_buffer,
            self.weights.shape().to_vec(),
            device.clone(),
        );

        let updated_velocity = if self.momentum != 0.0 {
            Some(Tensor::from_buffer(
                velocity_out_buffer,
                self.weights.shape().to_vec(),
                device.clone(),
            ))
        } else {
            None
        };

        Ok((updated_weights, updated_velocity))
    }
}

// ═══════════════════════════════════════════════════════════════
// TENSOR API INTEGRATION (MODERN IDIOMATIC RUST)
// ═══════════════════════════════════════════════════════════════

impl Tensor {
    /// SGD optimizer step - foundational gradient descent optimizer
    ///
    /// **Deep Debt**: Foundation for all deep learning training
    ///
    /// # Arguments
    /// - `gradients`: Gradient tensor [same shape as weights]
    /// - `learning_rate`: Step size, typically 0.01-0.1
    /// - `momentum`: Momentum factor 0.0-1.0, typically 0.9 (0.0 = no momentum)
    /// - `weight_decay`: L2 regularization, typically 0.0001-0.001
    /// - `velocity`: Momentum velocity (None for first step)
    ///
    /// # Returns
    /// - Tuple: (updated_weights, updated_velocity)
    ///
    /// # Example
    /// ```rust,ignore
    /// // Without momentum
    /// let (w1, _) = weights.sgd_step(&grads, 0.01, 0.0, 0.0, None)?;
    /// 
    /// // With momentum
    /// let (w1, v1) = weights.sgd_step(&grads, 0.01, 0.9, 0.0, None)?;
    /// let (w2, v2) = w1.sgd_step(&grads, 0.01, 0.9, 0.0, v1.as_ref())?;
    /// ```
    ///
    /// # Note
    /// - Foundation optimizer for deep learning
    /// - learning_rate must be positive
    /// - momentum must be in [0.0, 1.0]
    /// - weight_decay must be non-negative
    pub fn sgd_step(
        self,
        gradients: &Self,
        learning_rate: f32,
        momentum: f32,
        weight_decay: f32,
        velocity: Option<&Self>,
    ) -> Result<(Self, Option<Self>)> {
        SGD::new(
            self,
            gradients.clone(),
            learning_rate,
            momentum,
            weight_decay,
            velocity.cloned(),
        )?
        .execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_sgd_basic() {
        let device = get_test_device().await;

        let weights = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0], vec![4], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.1, 0.2, 0.3, 0.4], vec![4], device.clone())
            .await
            .unwrap();

        let (updated_weights, _) = weights
            .sgd_step(&gradients, 0.01, 0.0, 0.0, None)
            .unwrap();
        let result = updated_weights.to_vec().unwrap();

        // Weights should decrease (gradient descent)
        assert_eq!(result.len(), 4);
        assert!(result.iter().all(|&x| x.is_finite()));
        assert!(result[0] < 1.0, "Expected descent, got {}", result[0]);
    }

    #[tokio::test]
    async fn test_sgd_with_momentum() {
        let device = get_test_device().await;

        let weights = Tensor::from_vec_on(vec![1.0; 4], vec![4], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.1; 4], vec![4], device.clone())
            .await
            .unwrap();

        // First step with momentum
        let (weights1, velocity1) = weights
            .sgd_step(&gradients, 0.01, 0.9, 0.0, None)
            .unwrap();

        assert!(velocity1.is_some());
        let v = velocity1.unwrap();
        let v_data = v.to_vec().unwrap();
        assert!(v_data.iter().all(|&x| x.is_finite()));

        // Second step with accumulated momentum
        let (weights2, _velocity2) = weights1
            .sgd_step(&gradients, 0.01, 0.9, 0.0, Some(&v))
            .unwrap();

        let result = weights2.to_vec().unwrap();
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_sgd_with_weight_decay() {
        let device = get_test_device().await;

        let weights = Tensor::from_vec_on(vec![1.0; 4], vec![4], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.1; 4], vec![4], device.clone())
            .await
            .unwrap();

        let (updated_weights, _) = weights
            .sgd_step(&gradients, 0.01, 0.0, 0.001, None)
            .unwrap();

        let result = updated_weights.to_vec().unwrap();
        assert!(result.iter().all(|&x| x.is_finite()));
        assert!(result[0] < 1.0); // Should have decreased
    }

    #[tokio::test]
    async fn test_sgd_validation() {
        let device = get_test_device().await;

        let weights = Tensor::from_vec_on(vec![1.0; 10], vec![10], device.clone())
            .await
            .unwrap();
        let gradients = Tensor::from_vec_on(vec![0.1; 5], vec![5], device.clone())
            .await
            .unwrap();
        let grads_correct = Tensor::from_vec_on(vec![0.1; 10], vec![10], device.clone())
            .await
            .unwrap();

        // Shape mismatch
        assert!(weights
            .clone()
            .sgd_step(&gradients, 0.01, 0.0, 0.0, None)
            .is_err());

        // Invalid learning rate
        assert!(weights
            .clone()
            .sgd_step(&grads_correct, -0.01, 0.0, 0.0, None)
            .is_err());
        assert!(weights
            .clone()
            .sgd_step(&grads_correct, 0.0, 0.0, 0.0, None)
            .is_err());

        // Invalid momentum
        assert!(weights
            .clone()
            .sgd_step(&grads_correct, 0.01, -0.1, 0.0, None)
            .is_err());
        assert!(weights
            .clone()
            .sgd_step(&grads_correct, 0.01, 1.5, 0.0, None)
            .is_err());

        // Invalid weight decay
        assert!(weights
            .sgd_step(&grads_correct, 0.01, 0.0, -0.001, None)
            .is_err());
    }

    #[tokio::test]
    async fn test_sgd_large_batch() {
        let device = get_test_device().await;

        let size = 128;
        let weights = Tensor::from_vec_on(vec![1.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.01; size], vec![size], device.clone())
            .await
            .unwrap();

        let (updated_weights, _) = weights
            .sgd_step(&gradients, 0.01, 0.0, 0.0, None)
            .unwrap();

        let result = updated_weights.to_vec().unwrap();
        assert_eq!(result.len(), size);
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_sgd_multi_step() {
        let device = get_test_device().await;

        let weights = Tensor::from_vec_on(vec![10.0, 20.0], vec![2], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![1.0, 2.0], vec![2], device.clone())
            .await
            .unwrap();

        // Step 1
        let (weights1, v1) = weights
            .sgd_step(&gradients, 0.1, 0.9, 0.0, None)
            .unwrap();
        let result1 = weights1.to_vec().unwrap();

        assert!(result1[0] < 10.0, "Expected descent, got {}", result1[0]);
        assert!(result1[1] < 20.0, "Expected descent, got {}", result1[1]);

        // Step 2 with momentum
        let (weights2, _v2) = weights1
            .sgd_step(&gradients, 0.1, 0.9, 0.0, v1.as_ref())
            .unwrap();
        let result2 = weights2.to_vec().unwrap();

        // Should continue descending
        assert!(result2[0] < result1[0]);
        assert!(result2[1] < result1[1]);
    }
}
