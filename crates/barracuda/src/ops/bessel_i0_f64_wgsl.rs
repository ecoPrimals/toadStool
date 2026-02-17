//! BESSEL I0 F64 - Modified Bessel function of first kind, order 0 - f64 precision WGSL
//!
//! Deep Debt Principles apply. See bessel_j0_f64_wgsl.rs for details.
//!
//! Applications: Kaiser windows, cylindrical heat conduction, neutron diffusion

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// f64 Modified Bessel I0 function evaluator
pub struct BesselI0F64 {
    device: Arc<WgpuDevice>,
}

impl BesselI0F64 {
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/special/bessel_i0_f64.wgsl")
    }

    /// Compute I₀(x) for each element
    pub fn i0(&self, x: &[f64]) -> Result<Vec<f64>> {
        if x.is_empty() {
            return Ok(vec![]);
        }
        let size = x.len();
        if size < 256 {
            return Ok(self.i0_cpu(x));
        }
        self.i0_gpu(x)
    }

    fn i0_cpu(&self, x: &[f64]) -> Vec<f64> {
        x.iter().map(|&xi| Self::i0_scalar(xi)).collect()
    }

    fn i0_scalar(x: f64) -> f64 {
        let ax = x.abs();
        if ax < 3.75 {
            let y = x / 3.75;
            let t = y * y;
            1.0 + t * (3.5156229 + t * (3.0899424 + t * (1.2067492 + t * (0.2659732 + t * (0.0360768 + t * 0.0045813)))))
        } else {
            let t = 3.75 / ax;
            let p = 0.39894228 + t * (0.01328592 + t * (0.00225319 + t * (-0.00157565 + t * (0.00916281 + t * (-0.02057706 + t * (0.02635537 + t * (-0.01647633 + t * 0.00392377)))))));
            ax.exp() * p / ax.sqrt()
        }
    }

    fn i0_gpu(&self, x: &[f64]) -> Result<Vec<f64>> {
        let size = x.len();

        let input_buf = self.device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Bessel I0 f64 Input"),
            contents: bytemuck::cast_slice(x),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let output_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bessel I0 f64 Output"),
            size: std::mem::size_of_val(x) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Metadata { size: u32, _pad0: u32, _pad1: u32, _pad2: u32 }

        let metadata = Metadata { size: size as u32, _pad0: 0, _pad1: 0, _pad2: 0 };
        let metadata_buf = self.device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Bessel I0 f64 Metadata"),
            contents: bytemuck::bytes_of(&metadata),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let shader_module = self.device.compile_shader(Self::wgsl_shader(), Some("Bessel I0 f64"));

        let bind_group_layout = self.device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bessel I0 f64 BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let bind_group = self.device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bessel I0 f64 BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: output_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: metadata_buf.as_entire_binding() },
            ],
        });

        let pipeline_layout = self.device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Bessel I0 f64 PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = self.device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Bessel I0 f64 Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
        cache: None,
        compilation_options: Default::default(),
        });

        let mut encoder = self.device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Bessel I0 f64 Encoder") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Bessel I0 f64 Pass"), timestamp_writes: None });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((size as u32).div_ceil(256), 1, 1);
        }

        let staging_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bessel I0 f64 Staging"),
            size: std::mem::size_of_val(x) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, std::mem::size_of_val(x) as u64);
        self.device.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| { tx.send(result).unwrap(); });
        self.device.device.poll(wgpu::Maintain::Wait);
        rx.recv().map_err(|e| BarracudaError::Device(format!("Channel error: {}", e)))?.map_err(|e| BarracudaError::Device(format!("Buffer map error: {:?}", e)))?;

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_device() -> Result<Arc<WgpuDevice>> {
        let device = pollster::block_on(async { WgpuDevice::new_f64_capable().await })?;
        Ok(Arc::new(device))
    }

    #[test]
    fn test_i0_at_zero() -> Result<()> {
        let device = create_test_device()?;
        let bessel = BesselI0F64::new(device)?;
        let result = bessel.i0(&[0.0])?;
        assert!((result[0] - 1.0).abs() < 1e-10, "I₀(0) = {}, expected 1", result[0]);
        Ok(())
    }

    #[test]
    fn test_i0_known_values() -> Result<()> {
        let device = create_test_device()?;
        let bessel = BesselI0F64::new(device)?;
        let x = vec![1.0, 2.0, 3.0];
        let expected = vec![1.2660658777520082, 2.2795853023360673, 4.880792585865024];
        let result = bessel.i0(&x)?;
        for (i, &val) in result.iter().enumerate() {
            assert!((val - expected[i]).abs() < 1e-5, "I₀({}) = {}, expected {}", x[i], val, expected[i]);
        }
        Ok(())
    }

    #[test]
    fn test_i0_symmetry() -> Result<()> {
        let device = create_test_device()?;
        let bessel = BesselI0F64::new(device)?;
        let x = vec![-2.0, 2.0];
        let result = bessel.i0(&x)?;
        assert!((result[0] - result[1]).abs() < 1e-10, "I₀(-x) != I₀(x)");
        Ok(())
    }
}
