//! Device Capability Detection - Runtime Hardware Limits
//!
//! **Deep Debt Compliance**: Zero hardcoding, runtime discovery
//!
//! This module provides runtime detection of device capabilities,
//! enabling optimal configuration for any GPU/hardware.
//!
//! # Philosophy
//!
//! - ✅ **Query, don't hardcode**: Ask device for limits
//! - ✅ **Adapt to hardware**: Different optimal configs per vendor
//! - ✅ **Performance**: Use device-specific optimal values
//! - ✅ **Portability**: Works on any WebGPU device
//!
//! # Example
//!
//! ```no_run
//! use barracuda::device::{WgpuDevice, DeviceCapabilities};
//!
//! # async fn example() -> barracuda::error::Result<()> {
//! let device = WgpuDevice::new().await?;
//! let caps = DeviceCapabilities::from_device(&device);
//!
//! // Optimal workgroup size for this specific GPU
//! let workgroup_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
//! println!("Optimal workgroup size: {}", workgroup_size);
//!
//! // Check if operation is supported
//! if caps.max_buffer_size >= required_size {
//!     // Proceed with operation
//! }
//! # Ok(())
//! # }
//! ```

use crate::device::WgpuDevice;
use std::fmt;

/// Device capabilities - runtime hardware limits
///
/// **Deep Debt**: All values discovered at runtime, zero hardcoding
#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    /// Device name (e.g., "NVIDIA RTX 4090")
    pub device_name: String,

    /// Device type (DiscreteGpu, IntegratedGpu, Cpu, etc.)
    pub device_type: wgpu::DeviceType,

    /// Maximum buffer size (bytes)
    pub max_buffer_size: u64,

    /// Maximum workgroup size per dimension (x, y, z)
    pub max_workgroup_size: (u32, u32, u32),

    /// Maximum workgroups per dispatch
    pub max_compute_workgroups: (u32, u32, u32),

    /// Maximum invocations per workgroup
    pub max_compute_invocations_per_workgroup: u32,

    /// Maximum storage buffers per shader stage
    pub max_storage_buffers_per_shader_stage: u32,

    /// Maximum uniform buffers per shader stage
    pub max_uniform_buffers_per_shader_stage: u32,

    /// Maximum bind groups
    pub max_bind_groups: u32,

    /// Backend (Vulkan, Metal, DX12, GL, etc.)
    pub backend: wgpu::Backend,

    /// Vendor ID (e.g., NVIDIA=0x10DE, AMD=0x1002, Intel=0x8086)
    pub vendor: u32,
}

impl DeviceCapabilities {
    /// Detect capabilities from wgpu device
    ///
    /// **Deep Debt**: Runtime discovery, no assumptions
    pub fn from_device(device: &WgpuDevice) -> Self {
        let limits = device.device().limits();
        let adapter_info = device.adapter_info();

        Self {
            device_name: adapter_info.name.clone(),
            device_type: adapter_info.device_type,
            max_buffer_size: limits.max_buffer_size,
            max_workgroup_size: (
                limits.max_compute_workgroup_size_x,
                limits.max_compute_workgroup_size_y,
                limits.max_compute_workgroup_size_z,
            ),
            max_compute_workgroups: (
                limits.max_compute_workgroups_per_dimension,
                limits.max_compute_workgroups_per_dimension,
                limits.max_compute_workgroups_per_dimension,
            ),
            max_compute_invocations_per_workgroup: limits.max_compute_invocations_per_workgroup,
            max_storage_buffers_per_shader_stage: limits.max_storage_buffers_per_shader_stage,
            max_uniform_buffers_per_shader_stage: limits.max_uniform_buffers_per_shader_stage,
            max_bind_groups: limits.max_bind_groups,
            backend: adapter_info.backend,
            vendor: adapter_info.vendor,
        }
    }

    /// Get optimal workgroup size for a specific workload
    ///
    /// **Deep Debt**: Vendor-specific optimization, capability-based
    ///
    /// Different vendors have different optimal workgroup sizes:
    /// - NVIDIA: Prefers 128-256 (warp size 32)
    /// - AMD: Prefers 64-256 (wavefront size 64)
    /// - Intel: Prefers 128-256 (subgroup size varies)
    /// - CPU: Prefers smaller (16-64 for cache efficiency)
    pub fn optimal_workgroup_size(&self, workload: WorkloadType) -> u32 {
        match self.device_type {
            wgpu::DeviceType::DiscreteGpu => {
                // Discrete GPU - optimize by vendor
                match self.vendor {
                    // NVIDIA (0x10DE)
                    0x10DE => match workload {
                        WorkloadType::ElementWise => 256, // Full warp utilization
                        WorkloadType::MatMul => 256,      // Good for matrix tiles
                        WorkloadType::Reduction => 512,   // More threads for reduction trees
                        WorkloadType::FHE => 256,         // Balanced for U64 emulation
                        WorkloadType::Convolution => 128, // Cache-friendly for spatial locality
                    },
                    // AMD (0x1002)
                    0x1002 => match workload {
                        WorkloadType::ElementWise => 256, // Wavefront-aligned
                        WorkloadType::MatMul => 256,
                        WorkloadType::Reduction => 256, // AMD prefers consistent sizes
                        WorkloadType::FHE => 256,
                        WorkloadType::Convolution => 128,
                    },
                    // Intel (0x8086)
                    0x8086 => match workload {
                        WorkloadType::ElementWise => 128, // Conservative for Intel
                        WorkloadType::MatMul => 128,
                        WorkloadType::Reduction => 256,
                        WorkloadType::FHE => 128,
                        WorkloadType::Convolution => 64,
                    },
                    // Unknown vendor - conservative defaults
                    _ => match workload {
                        WorkloadType::ElementWise => 128,
                        WorkloadType::MatMul => 128,
                        WorkloadType::Reduction => 256,
                        WorkloadType::FHE => 128,
                        WorkloadType::Convolution => 64,
                    },
                }
            }

            wgpu::DeviceType::IntegratedGpu => {
                // Integrated GPU - smaller for shared memory pressure
                match workload {
                    WorkloadType::ElementWise => 128,
                    WorkloadType::MatMul => 64,
                    WorkloadType::Reduction => 128,
                    WorkloadType::FHE => 64,
                    WorkloadType::Convolution => 64,
                }
            }

            wgpu::DeviceType::Cpu => {
                // CPU - much smaller for cache efficiency
                match workload {
                    WorkloadType::ElementWise => 32,
                    WorkloadType::MatMul => 16,
                    WorkloadType::Reduction => 64,
                    WorkloadType::FHE => 32,
                    WorkloadType::Convolution => 16,
                }
            }

            _ => {
                // Virtual GPU, Other - conservative defaults
                match workload {
                    WorkloadType::ElementWise => 64,
                    WorkloadType::MatMul => 64,
                    WorkloadType::Reduction => 128,
                    WorkloadType::FHE => 64,
                    WorkloadType::Convolution => 32,
                }
            }
        }
        .min(self.max_compute_invocations_per_workgroup) // Never exceed device limit
    }

    /// Get optimal 2D workgroup size (for 2D operations like convolutions)
    ///
    /// **Deep Debt**: Balanced tile sizes for spatial operations
    pub fn optimal_workgroup_size_2d(&self, workload: WorkloadType) -> (u32, u32) {
        let total = self.optimal_workgroup_size(workload);

        // Square tiles are often optimal for 2D operations
        let side = (total as f32).sqrt() as u32;

        // Ensure we don't exceed per-dimension limits
        let x = side.min(self.max_workgroup_size.0);
        let y = side.min(self.max_workgroup_size.1);

        (x, y)
    }

    /// Get optimal 3D workgroup size (for 3D operations)
    pub fn optimal_workgroup_size_3d(&self, workload: WorkloadType) -> (u32, u32, u32) {
        let total = self.optimal_workgroup_size(workload);

        // Cube root for balanced 3D tiles
        let side = (total as f32).cbrt() as u32;

        // Ensure we don't exceed per-dimension limits
        let x = side.min(self.max_workgroup_size.0);
        let y = side.min(self.max_workgroup_size.1);
        let z = side.min(self.max_workgroup_size.2);

        (x, y, z)
    }

    /// Get maximum allocation size for this device
    ///
    /// **Deep Debt**: Safe memory limits based on actual hardware
    pub fn max_allocation_size(&self) -> u64 {
        // Conservative: Use 75% of max buffer size to leave room for other allocations
        (self.max_buffer_size as f64 * 0.75) as u64
    }

    /// Check if device supports FHE workloads (large U64 buffers)
    ///
    /// **Deep Debt**: Capability detection for specialized workloads
    pub fn supports_fhe(&self) -> bool {
        // FHE needs large buffers for polynomial operations
        // Minimum: 16K degree polynomial * 8 bytes * 2 (input/output) = 256KB
        self.max_buffer_size >= 256 * 1024
    }

    /// Check if device supports large matrix operations
    pub fn supports_large_matmul(&self, m: usize, n: usize, k: usize) -> bool {
        // Estimate memory needed: (m*k + k*n + m*n) * 4 bytes (f32)
        let required_bytes = (m * k + k * n + m * n) * 4;
        required_bytes as u64 <= self.max_allocation_size()
    }

    /// Get optimal tile size for matrix multiplication
    ///
    /// **Deep Debt**: Device-specific tiling for optimal cache usage
    pub fn optimal_matmul_tile_size(&self) -> u32 {
        match self.device_type {
            wgpu::DeviceType::DiscreteGpu => {
                // Larger tiles for discrete GPU (more shared memory)
                match self.vendor {
                    0x10DE => 32, // NVIDIA - 32x32 tiles (1024 threads)
                    0x1002 => 32, // AMD - 32x32 tiles
                    0x8086 => 16, // Intel - smaller tiles
                    _ => 16,      // Conservative default
                }
            }
            wgpu::DeviceType::IntegratedGpu => 16, // Smaller for integrated
            wgpu::DeviceType::Cpu => 8,            // Much smaller for CPU cache
            _ => 8,                                // Conservative
        }
    }

    /// Get vendor name (for logging/debugging)
    pub fn vendor_name(&self) -> &'static str {
        match self.vendor {
            0x10DE => "NVIDIA",
            0x1002 => "AMD",
            0x8086 => "Intel",
            0x13B5 => "ARM",
            0x5143 => "Qualcomm",
            0x1010 => "ImgTec",
            _ => "Unknown",
        }
    }

    /// Check if this is a high-performance GPU
    ///
    /// **Deep Debt**: Workload routing decisions based on capabilities
    pub fn is_high_performance(&self) -> bool {
        matches!(self.device_type, wgpu::DeviceType::DiscreteGpu)
            && self.max_compute_invocations_per_workgroup >= 1024
    }
}

/// Workload types for optimal configuration
///
/// **Deep Debt**: Different operations need different configurations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkloadType {
    /// Element-wise operations (add, mul, relu, etc.)
    ElementWise,

    /// Matrix multiplication
    MatMul,

    /// Reduction operations (sum, max, mean, etc.)
    Reduction,

    /// Homomorphic encryption operations
    FHE,

    /// Convolution operations
    Convolution,
}

impl fmt::Display for DeviceCapabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Device Capabilities:")?;
        writeln!(f, "  Name: {}", self.device_name)?;
        writeln!(f, "  Type: {:?}", self.device_type)?;
        writeln!(
            f,
            "  Vendor: {} (0x{:04X})",
            self.vendor_name(),
            self.vendor
        )?;
        writeln!(f, "  Backend: {:?}", self.backend)?;
        writeln!(f)?;
        writeln!(f, "Memory:")?;
        writeln!(
            f,
            "  Max Buffer Size: {} MB",
            self.max_buffer_size / (1024 * 1024)
        )?;
        writeln!(
            f,
            "  Max Allocation: {} MB",
            self.max_allocation_size() / (1024 * 1024)
        )?;
        writeln!(f)?;
        writeln!(f, "Compute:")?;
        writeln!(f, "  Max Workgroup Size: {:?}", self.max_workgroup_size)?;
        writeln!(
            f,
            "  Max Invocations/Workgroup: {}",
            self.max_compute_invocations_per_workgroup
        )?;
        writeln!(
            f,
            "  Max Compute Workgroups: {:?}",
            self.max_compute_workgroups
        )?;
        writeln!(f)?;
        writeln!(f, "Optimal Configurations:")?;
        writeln!(
            f,
            "  Element-wise: {} threads",
            self.optimal_workgroup_size(WorkloadType::ElementWise)
        )?;
        writeln!(
            f,
            "  MatMul: {} threads (tile: {})",
            self.optimal_workgroup_size(WorkloadType::MatMul),
            self.optimal_matmul_tile_size()
        )?;
        writeln!(
            f,
            "  Reduction: {} threads",
            self.optimal_workgroup_size(WorkloadType::Reduction)
        )?;
        writeln!(
            f,
            "  FHE: {} threads",
            self.optimal_workgroup_size(WorkloadType::FHE)
        )?;
        writeln!(
            f,
            "  Convolution: {:?}",
            self.optimal_workgroup_size_2d(WorkloadType::Convolution)
        )?;
        writeln!(f)?;
        writeln!(f, "Features:")?;
        writeln!(
            f,
            "  FHE Support: {}",
            if self.supports_fhe() { "Yes" } else { "No" }
        )?;
        writeln!(
            f,
            "  High Performance: {}",
            if self.is_high_performance() {
                "Yes"
            } else {
                "No"
            }
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workgroup_sizes_within_limits() {
        // Mock capabilities for testing
        let caps = DeviceCapabilities {
            device_name: "Test GPU".to_string(),
            device_type: wgpu::DeviceType::DiscreteGpu,
            max_buffer_size: 1024 * 1024 * 1024, // 1GB
            max_workgroup_size: (256, 256, 64),
            max_compute_workgroups: (65535, 65535, 65535),
            max_compute_invocations_per_workgroup: 256,
            max_storage_buffers_per_shader_stage: 8,
            max_uniform_buffers_per_shader_stage: 12,
            max_bind_groups: 4,
            backend: wgpu::Backend::Vulkan,
            vendor: 0x10DE, // NVIDIA
        };

        // Test all workload types
        let workloads = vec![
            WorkloadType::ElementWise,
            WorkloadType::MatMul,
            WorkloadType::Reduction,
            WorkloadType::FHE,
            WorkloadType::Convolution,
        ];

        for workload in workloads {
            let size = caps.optimal_workgroup_size(workload);
            assert!(
                size <= caps.max_compute_invocations_per_workgroup,
                "Workgroup size {} exceeds max {} for workload {:?}",
                size,
                caps.max_compute_invocations_per_workgroup,
                workload
            );

            let (x, y) = caps.optimal_workgroup_size_2d(workload);
            assert!(x <= caps.max_workgroup_size.0);
            assert!(y <= caps.max_workgroup_size.1);
            assert!(x * y <= caps.max_compute_invocations_per_workgroup);

            let (x, y, z) = caps.optimal_workgroup_size_3d(workload);
            assert!(x <= caps.max_workgroup_size.0);
            assert!(y <= caps.max_workgroup_size.1);
            assert!(z <= caps.max_workgroup_size.2);
            assert!(x * y * z <= caps.max_compute_invocations_per_workgroup);
        }
    }

    #[test]
    fn test_fhe_support_detection() {
        let caps_supported = DeviceCapabilities {
            device_name: "Large GPU".to_string(),
            device_type: wgpu::DeviceType::DiscreteGpu,
            max_buffer_size: 1024 * 1024 * 1024, // 1GB - supports FHE
            max_workgroup_size: (256, 256, 64),
            max_compute_workgroups: (65535, 65535, 65535),
            max_compute_invocations_per_workgroup: 1024,
            max_storage_buffers_per_shader_stage: 8,
            max_uniform_buffers_per_shader_stage: 12,
            max_bind_groups: 4,
            backend: wgpu::Backend::Vulkan,
            vendor: 0x10DE,
        };

        assert!(caps_supported.supports_fhe());

        let caps_limited = DeviceCapabilities {
            device_name: "Small GPU".to_string(),
            device_type: wgpu::DeviceType::IntegratedGpu,
            max_buffer_size: 128 * 1024, // 128KB - too small for FHE
            max_workgroup_size: (128, 128, 32),
            max_compute_workgroups: (65535, 65535, 65535),
            max_compute_invocations_per_workgroup: 256,
            max_storage_buffers_per_shader_stage: 8,
            max_uniform_buffers_per_shader_stage: 12,
            max_bind_groups: 4,
            backend: wgpu::Backend::Vulkan,
            vendor: 0x8086,
        };

        assert!(!caps_limited.supports_fhe());
    }
}
