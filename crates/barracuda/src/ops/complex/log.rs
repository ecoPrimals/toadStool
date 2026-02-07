//! Complex Logarithm

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

pub struct ComplexLog {
    input: Tensor,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl ComplexLog {
    pub fn new(input: Tensor) -> Result<Self> {
        if input.shape().last() != Some(&2) {
            return Err(BarracudaError::Device("Must have last dimension = 2".to_string()));
        }
        let device = input.device();
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Complex Log Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("log.wgsl").into()),
        });
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("PL"), bind_group_layouts: &[&bind_group_layout], push_constant_ranges: &[] });
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor { label: Some("Pipeline"), layout: Some(&pipeline_layout), module: &shader, entry_point: "main" });
        Ok(Self { input, pipeline, bind_group_layout })
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let n = self.input.len();
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor { label: Some("Out"), size: (n * std::mem::size_of::<f32>()) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
        let params = [n as u32 / 2];
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("P"), contents: bytemuck::cast_slice(&params), usage: wgpu::BufferUsages::UNIFORM });
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("BG"), layout: &self.bind_group_layout, entries: &[ wgpu::BindGroupEntry { binding: 0, resource: self.input.buffer().as_entire_binding() }, wgpu::BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() }, wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() }] });
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("E") });
        { let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("P"), timestamp_writes: None }); pass.set_pipeline(&self.pipeline); pass.set_bind_group(0, &bind_group, &[]); let caps = DeviceCapabilities::from_device(&device); let wg = ((n / 2) as u32 + caps.optimal_workgroup_size(WorkloadType::ElementWise) - 1) / caps.optimal_workgroup_size(WorkloadType::ElementWise); pass.dispatch_workgroups(wg, 1, 1); }
        device.queue.submit(Some(encoder.finish()));
        Ok(Tensor::from_buffer(output_buffer, self.input.shape().to_vec(), device.clone()))
    }
}
