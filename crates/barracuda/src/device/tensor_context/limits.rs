//! WGSL device limits for scientific computing

/// Science-grade limits — 512 MiB binding, 1 GiB buffer, 12 storage buffers.
/// Validated by hotSpring nuclear EOS study (169/169 acceptance checks).
pub fn science_limits() -> wgpu::Limits {
    wgpu::Limits {
        max_storage_buffer_binding_size: 512 * 1024 * 1024,
        max_buffer_size: 1024 * 1024 * 1024,
        max_storage_buffers_per_shader_stage: 12,
        ..wgpu::Limits::default()
    }
}

/// High-capacity limits — 1GB binding, 2GB buffer.
pub fn high_capacity_limits() -> wgpu::Limits {
    wgpu::Limits {
        max_storage_buffer_binding_size: 1 << 30,
        max_buffer_size: 1 << 31,
        ..wgpu::Limits::default()
    }
}
