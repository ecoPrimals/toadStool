//! Pipeline Cache for BarraCUDA
//!
//! Caches shader modules, bind group layouts, and compute pipelines
//! to eliminate redundant GPU object creation.
//!
//! This is critical for achieving CUDA parity - native CUDA only
//! compiles kernels once, while naive wgpu creates them every dispatch.
//!
//! Note: Each wgpu Device has its own cache because GPU objects are
//! not transferable between devices.

use dashmap::DashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use wgpu::{BindGroupLayout, ComputePipeline, Device, ShaderModule};

/// Key for caching shader modules (includes device ID)
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ShaderKey {
    /// Hash of shader source
    source_hash: u64,
    /// Device global ID (unique per wgpu device)
    device_id: wgpu::Id<wgpu::Device>,
}

impl ShaderKey {
    pub fn new(source: &str, device: &Device) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        Self {
            source_hash: hasher.finish(),
            device_id: device.global_id(),
        }
    }
}

/// Bind group layout signature (without device - used for creating keys)
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct BindGroupLayoutSignature {
    /// Number of read-only storage buffers
    pub read_only_buffers: u32,
    /// Number of read-write storage buffers
    pub read_write_buffers: u32,
    /// Number of uniform buffers
    pub uniform_buffers: u32,
}

impl BindGroupLayoutSignature {
    /// Standard elementwise binary op (2 read, 1 write)
    pub fn elementwise_binary() -> Self {
        Self {
            read_only_buffers: 2,
            read_write_buffers: 1,
            uniform_buffers: 0,
        }
    }

    /// Standard unary op (1 read, 1 write)
    pub fn elementwise_unary() -> Self {
        Self {
            read_only_buffers: 1,
            read_write_buffers: 1,
            uniform_buffers: 0,
        }
    }

    /// Reduction op (1 read, 1 write, 1 uniform for params)
    pub fn reduction() -> Self {
        Self {
            read_only_buffers: 1,
            read_write_buffers: 1,
            uniform_buffers: 1,
        }
    }

    /// Matmul (2 read, 1 write, 1 uniform for dimensions)
    pub fn matmul() -> Self {
        Self {
            read_only_buffers: 2,
            read_write_buffers: 1,
            uniform_buffers: 1,
        }
    }
}

/// Key for caching bind group layouts (includes device ID)
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct BindGroupLayoutKey {
    signature: BindGroupLayoutSignature,
    device_id: wgpu::Id<wgpu::Device>,
}

impl BindGroupLayoutKey {
    pub fn new(signature: BindGroupLayoutSignature, device: &Device) -> Self {
        Self {
            signature,
            device_id: device.global_id(),
        }
    }
}

/// Key for caching compute pipelines (includes device ID)
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PipelineKey {
    source_hash: u64,
    layout_signature: BindGroupLayoutSignature,
    entry_point: String,
    device_id: wgpu::Id<wgpu::Device>,
}

impl PipelineKey {
    pub fn new(shader_source: &str, layout_signature: BindGroupLayoutSignature, entry_point: &str, device: &Device) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        shader_source.hash(&mut hasher);
        Self {
            source_hash: hasher.finish(),
            layout_signature,
            entry_point: entry_point.to_string(),
            device_id: device.global_id(),
        }
    }
}

/// Thread-safe pipeline cache
/// 
/// Note: Keys include device ID to ensure GPU objects are only used
/// with the device that created them.
pub struct PipelineCache {
    /// Cached shader modules (keyed by source hash + device)
    shaders: DashMap<ShaderKey, Arc<ShaderModule>>,
    
    /// Cached bind group layouts (keyed by signature + device)
    layouts: DashMap<BindGroupLayoutKey, Arc<BindGroupLayout>>,
    
    /// Cached compute pipelines (keyed by shader + layout + entry + device)
    pipelines: DashMap<PipelineKey, Arc<ComputePipeline>>,
}

impl PipelineCache {
    pub fn new() -> Self {
        Self {
            shaders: DashMap::new(),
            layouts: DashMap::new(),
            pipelines: DashMap::new(),
        }
    }

    /// Get or compile a shader module
    pub fn get_or_compile_shader(
        &self,
        device: &Device,
        source: &str,
        label: Option<&str>,
    ) -> Arc<ShaderModule> {
        let key = ShaderKey::new(source, device);
        
        self.shaders
            .entry(key)
            .or_insert_with(|| {
                let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label,
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                });
                Arc::new(module)
            })
            .clone()
    }

    /// Get or create a bind group layout
    pub fn get_or_create_layout(
        &self,
        device: &Device,
        signature: BindGroupLayoutSignature,
        label: Option<&str>,
    ) -> Arc<BindGroupLayout> {
        let key = BindGroupLayoutKey::new(signature, device);
        
        self.layouts
            .entry(key)
            .or_insert_with(|| {
                let mut entries = Vec::new();
                let mut binding = 0u32;

                // Read-only storage buffers
                for _ in 0..signature.read_only_buffers {
                    entries.push(wgpu::BindGroupLayoutEntry {
                        binding,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    });
                    binding += 1;
                }

                // Read-write storage buffers
                for _ in 0..signature.read_write_buffers {
                    entries.push(wgpu::BindGroupLayoutEntry {
                        binding,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    });
                    binding += 1;
                }

                // Uniform buffers
                for _ in 0..signature.uniform_buffers {
                    entries.push(wgpu::BindGroupLayoutEntry {
                        binding,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    });
                    binding += 1;
                }

                let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label,
                    entries: &entries,
                });
                Arc::new(layout)
            })
            .clone()
    }

    /// Get or create a compute pipeline
    pub fn get_or_create_pipeline(
        &self,
        device: &Device,
        shader_source: &str,
        layout_signature: BindGroupLayoutSignature,
        entry_point: &str,
        label: Option<&str>,
    ) -> Arc<ComputePipeline> {
        let key = PipelineKey::new(shader_source, layout_signature, entry_point, device);

        self.pipelines
            .entry(key)
            .or_insert_with(|| {
                // Get cached shader and layout
                let shader = self.get_or_compile_shader(device, shader_source, label);
                let layout = self.get_or_create_layout(device, layout_signature, label);

                let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label,
                    bind_group_layouts: &[&layout],
                    push_constant_ranges: &[],
                });

                let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label,
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point,
                });

                Arc::new(pipeline)
            })
            .clone()
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            shaders: self.shaders.len(),
            layouts: self.layouts.len(),
            pipelines: self.pipelines.len(),
        }
    }

    /// Clear all cached objects
    pub fn clear(&self) {
        self.shaders.clear();
        self.layouts.clear();
        self.pipelines.clear();
    }
}

impl Default for PipelineCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub shaders: usize,
    pub layouts: usize,
    pub pipelines: usize,
}

/// Global pipeline cache (singleton per device)
lazy_static::lazy_static! {
    pub static ref GLOBAL_CACHE: PipelineCache = PipelineCache::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_key_consistency() {
        let source = "fn main() {}";
        let key1 = ShaderKey::new(source);
        let key2 = ShaderKey::new(source);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_layout_key_presets() {
        let binary = BindGroupLayoutKey::elementwise_binary();
        assert_eq!(binary.read_only_buffers, 2);
        assert_eq!(binary.read_write_buffers, 1);
    }
}
