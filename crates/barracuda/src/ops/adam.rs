//! Adam Optimizer - GPU-accelerated Adaptive Moment Estimation
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
//! m_t = β₁ * m_{t-1} + (1 - β₁) * g_t
//! v_t = β₂ * v_{t-1} + (1 - β₂) * g_t²
//! m̂_t = m_t / (1 - β₁^t)
//! v̂_t = v_t / (1 - β₂^t)
//! θ_t = θ_{t-1} - α * m̂_t / (sqrt(v̂_t) + ε)
//! ```
//!
//! **Key Properties**:
//! - Most widely used optimizer in deep learning
//! - Combines momentum and RMSprop
//! - Bias correction for moving averages
//! - Works well with sparse gradients
//! - Computationally efficient
//!
//! **Parameters**:
//! - `learning_rate` (α): Step size, typically 0.001
//! - `beta1` (β₁): Exponential decay for first moment, typically 0.9
//! - `beta2` (β₂): Exponential decay for second moment, typically 0.999
//! - `epsilon` (ε): Numerical stability, typically 1e-8
//! - `step`: Current iteration number (for bias correction)
//!
//! **Used By**: Almost all modern deep learning (GPT, BERT, ResNet, etc.)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let weights = Tensor::randn(vec![1000]).await?;
//! let gradients = Tensor::randn(vec![1000]).await?;
//!
//! // First step
//! let (w1, m1, v1) = weights.adam_step(&gradients, 0.001, 0.9, 0.999, 1, None, None)?;
//!
//! // Subsequent steps
//! let (w2, m2, v2) = w1.adam_step(&gradients, 0.001, 0.9, 0.999, 2, Some(&m1), Some(&v1))?;
//! ```

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AdamParams {
    num_params: u32,
    learning_rate: f32,
    beta1: f32,
    beta2: f32,
    epsilon: f32,
    weight_decay: f32,
    step: u32,
}

pub struct Adam {
    gradients: Tensor,
    params: Tensor,
    m: Option<Tensor>,
    v: Option<Tensor>,
    learning_rate: f32,
    beta1: f32,
    beta2: f32,
    step: usize,
}

impl Adam {
    pub fn new(
        params: Tensor,
        gradients: Tensor,
        learning_rate: f32,
        beta1: f32,
        beta2: f32,
        step: usize,
        m: Option<Tensor>,
        v: Option<Tensor>,
    ) -> Result<Self> {
        // Validate shapes match
        if params.shape() != gradients.shape() {
            return Err(BarracudaError::shape_mismatch(
                params.shape().to_vec(),
                gradients.shape().to_vec(),
            ));
        }

        // Validate learning rate is positive
        if learning_rate <= 0.0 {
            return Err(BarracudaError::invalid_op(
                "adam",
                "learning_rate must be positive",
            ));
        }

        // Validate betas in valid range
        if !(0.0..1.0).contains(&beta1) {
            return Err(BarracudaError::invalid_op(
                "adam",
                "beta1 must be in range [0.0, 1.0)",
            ));
        }

        if !(0.0..1.0).contains(&beta2) {
            return Err(BarracudaError::invalid_op(
                "adam",
                "beta2 must be in range [0.0, 1.0)",
            ));
        }

        // Validate step is positive
        if step == 0 {
            return Err(BarracudaError::invalid_op(
                "adam",
                "step must be >= 1 (starts at 1, not 0)",
            ));
        }

        // Validate m and v shapes if provided
        if let Some(ref m_tensor) = m {
            if m_tensor.shape() != params.shape() {
                return Err(BarracudaError::shape_mismatch(
                    m_tensor.shape().to_vec(),
                    params.shape().to_vec(),
                ));
            }
        }

        if let Some(ref v_tensor) = v {
            if v_tensor.shape() != params.shape() {
                return Err(BarracudaError::shape_mismatch(
                    v_tensor.shape().to_vec(),
                    params.shape().to_vec(),
                ));
            }
        }

        Ok(Self {
            gradients,
            params,
            m,
            v,
            learning_rate,
            beta1,
            beta2,
            step,
        })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/adam.wgsl")
    }

    pub fn execute(self) -> Result<(Tensor, Tensor, Tensor)> {
        let device = self.params.device();
        let size = self.params.shape().iter().product::<usize>();

        let adam_params = AdamParams {
            num_params: size as u32,
            learning_rate: self.learning_rate,
            beta1: self.beta1,
            beta2: self.beta2,
            epsilon: 1e-8,
            weight_decay: 0.0,
            step: self.step as u32,
        };

        // Create writable buffers (shader does in-place updates)
        let zeros = vec![0.0f32; size];

        // Copy params to writable buffer
        let params_data = self.params.to_vec()?;
        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("adam_params"),
                contents: bytemuck::cast_slice(&params_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });

        // Copy or create m buffer
        let m_data = if let Some(ref m_tensor) = self.m {
            m_tensor.to_vec()?
        } else {
            zeros.clone()
        };
        let m_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("adam_m"),
                contents: bytemuck::cast_slice(&m_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });

        // Copy or create v buffer
        let v_data = if let Some(ref v_tensor) = self.v {
            v_tensor.to_vec()?
        } else {
            zeros
        };
        let v_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("adam_v"),
                contents: bytemuck::cast_slice(&v_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });

        let adam_params_buffer =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("adam_params"),
                    contents: bytemuck::cast_slice(&[adam_params]),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("adam_shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("adam_bind_group_layout"),
                    entries: &[
                        // binding 0: gradients (read)
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
                        // binding 1: params (read_write)
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
                        // binding 2: m (read_write)
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
                        // binding 3: v (read_write)
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
                        // binding 4: adam_params (uniform)
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
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
                    label: Some("adam_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("adam_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("adam_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.gradients.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: m_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: v_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: adam_params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("adam_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("adam_pass"),
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

        let updated_params =
            Tensor::from_buffer(params_buffer, self.params.shape().to_vec(), device.clone());

        let updated_m = Tensor::from_buffer(m_buffer, self.params.shape().to_vec(), device.clone());

        let updated_v = Tensor::from_buffer(v_buffer, self.params.shape().to_vec(), device.clone());

        Ok((updated_params, updated_m, updated_v))
    }
}

// ═══════════════════════════════════════════════════════════════
// TENSOR API INTEGRATION (MODERN IDIOMATIC RUST)
// ═══════════════════════════════════════════════════════════════

impl Tensor {
    /// Adam optimizer step - most widely used optimizer in deep learning
    ///
    /// **Deep Debt**: Foundation for modern AI (GPT, BERT, ResNet, etc.)
    ///
    /// # Arguments
    /// - `gradients`: Gradient tensor [same shape as params]
    /// - `learning_rate`: Step size, typically 0.001
    /// - `beta1`: Exponential decay for first moment, typically 0.9
    /// - `beta2`: Exponential decay for second moment, typically 0.999
    /// - `step`: Current iteration (starts at 1, not 0)
    /// - `m`: First moment estimate (None for first step)
    /// - `v`: Second moment estimate (None for first step)
    ///
    /// # Returns
    /// - Tuple: (updated_params, updated_m, updated_v)
    ///
    /// # Example
    /// ```rust,ignore
    /// // First step
    /// let (w1, m1, v1) = weights.adam_step(&grads, 0.001, 0.9, 0.999, 1, None, None)?;
    ///
    /// // Subsequent steps
    /// let (w2, m2, v2) = w1.adam_step(&grads, 0.001, 0.9, 0.999, 2, Some(&m1), Some(&v1))?;
    /// ```
    ///
    /// # Note
    /// - Most widely used optimizer in deep learning
    /// - learning_rate must be positive
    /// - beta1, beta2 must be in [0.0, 1.0)
    /// - step must start at 1 (not 0)
    pub fn adam_step(
        self,
        gradients: &Self,
        learning_rate: f32,
        beta1: f32,
        beta2: f32,
        step: usize,
        m: Option<&Self>,
        v: Option<&Self>,
    ) -> Result<(Self, Self, Self)> {
        Adam::new(
            self,
            gradients.clone(),
            learning_rate,
            beta1,
            beta2,
            step,
            m.cloned(),
            v.cloned(),
        )?
        .execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_adam_basic() {
        let device = get_test_device().await;

        let params = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0], vec![4], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.1, 0.2, 0.3, 0.4], vec![4], device.clone())
            .await
            .unwrap();

        let (updated_params, _m, _v) = params
            .adam_step(&gradients, 0.001, 0.9, 0.999, 1, None, None)
            .unwrap();
        let result = updated_params.to_vec().unwrap();

        assert_eq!(result.len(), 4);
        assert!(result.iter().all(|&x| x.is_finite()));
        assert!(result[0] < 1.0, "Expected descent, got {}", result[0]);
    }

    #[tokio::test]
    async fn test_adam_with_state() {
        let device = get_test_device().await;

        let params = Tensor::from_vec_on(vec![1.0; 4], vec![4], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.1; 4], vec![4], device.clone())
            .await
            .unwrap();

        // Step 1
        let (params1, m1, v1) = params
            .adam_step(&gradients, 0.001, 0.9, 0.999, 1, None, None)
            .unwrap();

        let m_data = m1.to_vec().unwrap();
        let v_data = v1.to_vec().unwrap();
        assert!(m_data.iter().all(|&x| x.is_finite()));
        assert!(v_data.iter().all(|&x| x.is_finite()));

        // Step 2 with accumulated state
        let (params2, _m2, _v2) = params1
            .adam_step(&gradients, 0.001, 0.9, 0.999, 2, Some(&m1), Some(&v1))
            .unwrap();

        let result = params2.to_vec().unwrap();
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adam_validation() {
        let device = get_test_device().await;

        let params = Tensor::from_vec_on(vec![1.0; 10], vec![10], device.clone())
            .await
            .unwrap();
        let gradients = Tensor::from_vec_on(vec![0.1; 5], vec![5], device.clone())
            .await
            .unwrap();
        let grads_correct = Tensor::from_vec_on(vec![0.1; 10], vec![10], device.clone())
            .await
            .unwrap();

        // Shape mismatch
        assert!(params
            .clone()
            .adam_step(&gradients, 0.001, 0.9, 0.999, 1, None, None)
            .is_err());

        // Invalid learning rate
        assert!(params
            .clone()
            .adam_step(&grads_correct, -0.001, 0.9, 0.999, 1, None, None)
            .is_err());

        // Invalid beta1
        assert!(params
            .clone()
            .adam_step(&grads_correct, 0.001, -0.1, 0.999, 1, None, None)
            .is_err());
        assert!(params
            .clone()
            .adam_step(&grads_correct, 0.001, 1.0, 0.999, 1, None, None)
            .is_err());

        // Invalid step
        assert!(params
            .adam_step(&grads_correct, 0.001, 0.9, 0.999, 0, None, None)
            .is_err());
    }

    #[tokio::test]
    async fn test_adam_large_batch() {
        let device = get_test_device().await;

        let size = 128;
        let params = Tensor::from_vec_on(vec![1.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.01; size], vec![size], device.clone())
            .await
            .unwrap();

        let (updated_params, updated_m, updated_v) = params
            .adam_step(&gradients, 0.001, 0.9, 0.999, 1, None, None)
            .unwrap();

        let result = updated_params.to_vec().unwrap();
        let m = updated_m.to_vec().unwrap();
        let v = updated_v.to_vec().unwrap();

        assert_eq!(result.len(), size);
        assert_eq!(m.len(), size);
        assert_eq!(v.len(), size);
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adam_multi_step() {
        let device = get_test_device().await;

        let params = Tensor::from_vec_on(vec![10.0, 20.0], vec![2], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![1.0, 2.0], vec![2], device.clone())
            .await
            .unwrap();

        // Step 1
        let (params1, m1, v1) = params
            .adam_step(&gradients, 0.01, 0.9, 0.999, 1, None, None)
            .unwrap();
        let result1 = params1.to_vec().unwrap();

        assert!(result1[0] < 10.0, "Expected descent, got {}", result1[0]);
        assert!(result1[1] < 20.0, "Expected descent, got {}", result1[1]);

        // Step 2 with accumulated state
        let (params2, _m2, _v2) = params1
            .adam_step(&gradients, 0.01, 0.9, 0.999, 2, Some(&m1), Some(&v1))
            .unwrap();
        let result2 = params2.to_vec().unwrap();

        // Should continue descending
        assert!(result2[0] < result1[0]);
        assert!(result2[1] < result1[1]);
    }
}
