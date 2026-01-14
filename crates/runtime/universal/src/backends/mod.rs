//! Compute unit backend implementations

#[cfg(feature = "cpu")]
pub mod cpu;

#[cfg(feature = "opencl")]
pub mod opencl;

#[cfg(feature = "wgpu-backend")]
pub mod wgpu_backend;

#[cfg(feature = "cpu")]
pub use cpu::CpuComputeUnit;

#[cfg(feature = "opencl")]
pub use opencl::OpenClComputeUnit;

#[cfg(feature = "wgpu-backend")]
pub use wgpu_backend::WgpuComputeUnit;
