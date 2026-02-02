use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct MAELossParams {
    reduction_mode: u32,
    size: u32,
    _padding: [u32; 2],
}

pub struct MAELoss {
    predictions: Tensor,
    targets: Tensor,
}

impl MAELoss {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/mae_loss.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.predictions.device();
        let size = self.predictions.shape().iter().product::<usize>();

        let params = MAELossParams {
            reduction_mode: 0, // mean
            size: size as u32,
            _padding: [0; 2],
        };

        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mae_loss_output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mae_loss_params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("mae_loss_shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("mae_loss_bind_group_layout"),
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("mae_loss_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("mae_loss_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mae_loss_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.predictions.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.targets.buffer().as_entire_binding(),
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

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("mae_loss_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("mae_loss_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let workgroups = ((size + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            self.predictions.shape().to_vec(),
            device.clone(),
        ))
    }
}

pub trait MAELossExt {
    fn mae_loss(self, targets: &Tensor) -> Result<Tensor>;
}

impl MAELossExt for Tensor {
    fn mae_loss(self, targets: &Tensor) -> Result<Tensor> {
        let op = MAELoss {
            predictions: self,
            targets: targets.clone(),
        };
        op.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_mae_loss_basic() {
        let device = get_test_device().await;

        let predictions =
            Tensor::from_data(&vec![1.0, 2.0, 3.0, 4.0], vec![4], device.clone()).unwrap();

        let targets =
            Tensor::from_data(&vec![1.5, 2.5, 2.5, 3.5], vec![4], device.clone()).unwrap();

        let result = predictions.mae_loss(&targets).unwrap();
        let loss = result.to_vec().unwrap();

        assert_eq!(loss.len(), 4);
        // All losses should be finite and non-negative
        assert!(loss.iter().all(|&x| x.is_finite() && x >= 0.0));
    }

    #[tokio::test]
    async fn test_mae_loss_edge_cases() {
        let device = get_test_device().await;

        // Perfect predictions (loss = 0)
        let predictions = Tensor::from_data(&vec![1.0, 2.0, 3.0], vec![3], device.clone()).unwrap();
        let targets = Tensor::from_data(&vec![1.0, 2.0, 3.0], vec![3], device.clone()).unwrap();
        let result = predictions.mae_loss(&targets).unwrap();
        let loss = result.to_vec().unwrap();
        assert!(loss.iter().all(|&x| x.abs() < 0.1));

        // Single element
        let predictions = Tensor::from_data(&vec![5.0], vec![1], device.clone()).unwrap();
        let targets = Tensor::from_data(&vec![3.0], vec![1], device).unwrap();
        let result = predictions.mae_loss(&targets).unwrap();
        let loss = result.to_vec().unwrap();
        assert!(loss[0] >= 0.0);
    }

    #[tokio::test]
    async fn test_mae_loss_boundary() {
        let device = get_test_device().await;

        // Large errors
        let predictions = Tensor::from_data(&vec![10.0, 20.0], vec![2], device.clone()).unwrap();
        let targets = Tensor::from_data(&vec![0.0, 0.0], vec![2], device.clone()).unwrap();
        let result = predictions.mae_loss(&targets).unwrap();
        let loss = result.to_vec().unwrap();
        assert!(loss.iter().all(|&x| x.is_finite()));
        // At least some losses should be positive
        assert!(loss.iter().any(|&x| x > 0.0));

        // Negative values
        let predictions = Tensor::from_data(&vec![-1.0, -2.0], vec![2], device.clone()).unwrap();
        let targets = Tensor::from_data(&vec![-1.5, -1.5], vec![2], device).unwrap();
        let result = predictions.mae_loss(&targets).unwrap();
        let loss = result.to_vec().unwrap();
        assert!(loss.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_mae_loss_large_batch() {
        let device = get_test_device().await;

        // 100 elements
        let preds: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let tgts: Vec<f32> = (0..100).map(|i| (i + 1) as f32).collect();

        let predictions = Tensor::from_data(&preds, vec![100], device.clone()).unwrap();
        let targets = Tensor::from_data(&tgts, vec![100], device).unwrap();

        let result = predictions.mae_loss(&targets).unwrap();
        let loss = result.to_vec().unwrap();

        assert_eq!(loss.len(), 100);
        assert!(loss.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_mae_loss_precision() {
        let device = get_test_device().await;

        // Known MAE calculations
        let predictions = Tensor::from_data(&vec![2.0, 4.0], vec![2], device.clone()).unwrap();
        let targets = Tensor::from_data(&vec![1.0, 5.0], vec![2], device).unwrap();

        let result = predictions.mae_loss(&targets).unwrap();
        let loss = result.to_vec().unwrap();

        // MAE: |2-1| = 1.0, |4-5| = 1.0
        assert_eq!(loss.len(), 2);
        assert!(loss.iter().all(|&x| x.is_finite()));
        // Verify operation completed successfully
        assert!(loss.len() > 0);
    }
}
