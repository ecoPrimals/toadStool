//! Cdist - Pairwise distance computation - Pure WGSL
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (GPU-optimized)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready)
//!
//! ## Algorithm
//!
//! Computes pairwise distances between vectors:
//! ```text
//! Input A: [M, D] - M vectors of dimension D
//! Input B: [N, D] - N vectors of dimension D
//! Output:  [M, N] - Distance matrix
//!
//! Supports: Euclidean (L2), Manhattan (L1), Cosine
//! ```

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;

#[derive(Clone, Copy)]
pub enum DistanceMetric {
    Euclidean = 0,
    Manhattan = 1,
    Cosine = 2,
}

pub struct Cdist {
    input_a: Tensor,
    input_b: Tensor,
    metric: DistanceMetric,
}

impl Cdist {
    pub fn new(input_a: Tensor, input_b: Tensor, metric: DistanceMetric) -> Self {
        Self {
            input_a,
            input_b,
            metric,
        }
    }

    /// f32 WGSL shader (legacy, retained as fossil reference).
    #[allow(dead_code)]
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/misc/cdist.wgsl")
    }

    /// f64 version for universal math library portability.
    pub fn wgsl_shader_f64() -> &'static str {
        include_str!("../shaders/misc/cdist_f64.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input_a.device();
        let shape_a = self.input_a.shape();
        let shape_b = self.input_b.shape();

        // Expect 2D tensors [M, D] and [N, D]
        if shape_a.len() != 2 || shape_b.len() != 2 {
            return Err(crate::error::BarracudaError::InvalidShape {
                expected: vec![0, 0],
                actual: shape_a.to_vec(),
            });
        }

        let m = shape_a[0]; // Number of vectors in A
        let d_a = shape_a[1]; // Dimension of A
        let n = shape_b[0]; // Number of vectors in B
        let d_b = shape_b[1]; // Dimension of B

        if d_a != d_b {
            return Err(crate::error::BarracudaError::InvalidShape {
                expected: vec![n, d_a],
                actual: vec![n, d_b],
            });
        }

        let d = d_a;
        let output_buffer = device.create_buffer_f32(m * n)?;

        // Create params buffer
        let params_data = [m as u32, n as u32, d as u32, self.metric as u32];
        let params_buffer = device.create_uniform_buffer("Params", &params_data);

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Cdist BGL"),
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

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cdist BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input_a.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.input_b.buffer().as_entire_binding(),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Cdist"));
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Cdist PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Cdist Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Cdist Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Cdist Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
            let workgroups_x = (m as u32).div_ceil(optimal_wg_size);
            let workgroups_y = (n as u32).div_ceil(optimal_wg_size);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            vec![m, n],
            device.clone(),
        ))
    }
}

impl Tensor {
    pub fn cdist_wgsl(self, other: Tensor, metric: DistanceMetric) -> Result<Self> {
        Cdist::new(self, other, metric).execute()
    }
}

/// Standalone f64 pairwise distance computation (no Tensor needed).
///
/// * `x1` — `[n1 * d]` f64 flattened row-major
/// * `x2` — `[n2 * d]` f64 flattened row-major
///
/// Returns `[n1 * n2]` f64 distance matrix.
pub fn compute_distances_f64_gpu(
    device: &crate::device::WgpuDevice,
    x1: &[f64],
    n1: usize,
    x2: &[f64],
    n2: usize,
    n_dim: usize,
    metric: DistanceMetric,
) -> Result<Vec<f64>> {
    use bytemuck::{Pod, Zeroable};
    use wgpu::util::DeviceExt;

    #[repr(C)]
    #[derive(Copy, Clone, Pod, Zeroable)]
    struct CdistParams {
        m: u32,
        n: u32,
        d: u32,
        metric: u32,
    }

    let module = device.compile_shader_f64(Cdist::wgsl_shader_f64(), Some("cdist_f64"));
    let d = &device.device;
    let q = &device.queue;

    let a_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("cdist:a"),
        contents: bytemuck::cast_slice(x1),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let b_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("cdist:b"),
        contents: bytemuck::cast_slice(x2),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let out_size = (n1 * n2 * 8) as u64;
    let out_buf = d.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cdist:out"),
        size: out_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let params = CdistParams {
        m: n1 as u32,
        n: n2 as u32,
        d: n_dim as u32,
        metric: metric as u32,
    };
    let params_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("cdist:params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let bgl = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("cdist_f64:bgl"),
        entries: &[
            bgl_storage(0, true),
            bgl_storage(1, true),
            bgl_storage(2, false),
            bgl_uniform(3),
        ],
    });

    let bg = d.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("cdist_f64:bg"),
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: a_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: b_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: out_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: params_buf.as_entire_binding(),
            },
        ],
    });

    let pl = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("cdist_f64:pl"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    let pipeline = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("cdist_f64:pipeline"),
        layout: Some(&pl),
        module: &module,
        entry_point: "main",
        compilation_options: Default::default(),
        cache: None,
    });

    let mut enc = d.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("cdist_f64:enc"),
    });
    {
        let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("cdist_f64:pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bg, &[]);
        pass.dispatch_workgroups((n1 as u32).div_ceil(16), (n2 as u32).div_ceil(16), 1);
    }

    let readback = d.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cdist_f64:rb"),
        size: out_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    enc.copy_buffer_to_buffer(&out_buf, 0, &readback, 0, out_size);
    q.submit(Some(enc.finish()));

    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).ok();
    });
    d.poll(wgpu::Maintain::Wait);
    rx.recv()
        .map_err(|_| crate::error::BarracudaError::Gpu("cdist readback channel".into()))?
        .map_err(|e| crate::error::BarracudaError::Gpu(format!("cdist map: {e}")))?;

    let data = slice.get_mapped_range();
    let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    readback.unmap();

    Ok(result)
}

fn bgl_storage(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bgl_uniform(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_cdist_euclidean() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let a_data = vec![0.0, 0.0];
        let b_data = vec![3.0, 4.0];

        let a = Tensor::from_vec_on(a_data, vec![1, 2], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(b_data, vec![1, 2], device)
            .await
            .unwrap();

        let result = a.cdist_wgsl(b, DistanceMetric::Euclidean).unwrap();
        let output = result.to_vec().unwrap();

        assert!((output[0] - 5.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_cdist_manhattan() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let a_data = vec![0.0, 0.0];
        let b_data = vec![3.0, 4.0];

        let a = Tensor::from_vec_on(a_data, vec![1, 2], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(b_data, vec![1, 2], device)
            .await
            .unwrap();

        let result = a.cdist_wgsl(b, DistanceMetric::Manhattan).unwrap();
        let output = result.to_vec().unwrap();

        assert!((output[0] - 7.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_cdist_f64_euclidean() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // 2 points in 3D: (1,2,3) and (4,6,3)
        let x1 = vec![1.0_f64, 2.0, 3.0, 0.0, 0.0, 0.0];
        let x2 = vec![4.0_f64, 6.0, 3.0];

        let result =
            compute_distances_f64_gpu(&device, &x1, 2, &x2, 1, 3, DistanceMetric::Euclidean)
                .unwrap();

        // d((1,2,3),(4,6,3)) = sqrt(9+16+0) = 5.0
        assert!(
            (result[0] - 5.0).abs() < 1e-10,
            "expected 5.0, got {}",
            result[0]
        );
        // d((0,0,0),(4,6,3)) = sqrt(16+36+9) = sqrt(61) ≈ 7.8102
        let expected = 61.0_f64.sqrt();
        assert!(
            (result[1] - expected).abs() < 1e-10,
            "expected {expected}, got {}",
            result[1]
        );
    }
}
