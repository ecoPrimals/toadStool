// SPDX-License-Identifier: AGPL-3.0-only
//! CUDA Backend Implementation
//!
//! Fast AND safe CUDA execution for NVIDIA GPUs
//! Pragmatic support for Python AI ecosystem (PyTorch, TensorFlow)
//! Evolution path: Migrate to WebGPU when ecosystem matures
//!
//! ## Philosophy
//! - **Fast**: Direct CUDA API, zero overhead
//! - **Safe**: Comprehensive error handling, no panics
//! - **Pragmatic**: Supports Python AI workloads today
//! - **Evolvable**: Clear migration path to WebGPU
//!
//! ## Deep Debt: cudarc 0.19 Upgrade (Feb 2026)
//! - Proper device queries: name(), compute_capability(), attribute()
//! - Stream-based memory management (CudaStream)
//! - Modern launch_builder() API for kernels

mod device;
mod kernels;
mod ptx;
mod resource;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Arc;

use cudarc::driver::safe::{CudaContext, CudaModule, CudaStream};

/// CUDA compute backend - real NVIDIA GPU execution
///
/// Provides high-performance GPU compute via CUDA for AI/ML workloads
///
/// ## cudarc 0.19 Architecture
/// - `CudaContext`: Handle to device, manages lifetime
/// - `CudaStream`: Schedules work on device (memory ops, kernel launches)
/// - `CudaSlice`: Device memory owned by a context
pub struct CudaBackend {
    /// CUDA context (device handle) - cudarc 0.19 uses CudaContext instead of CudaDevice
    pub(crate) context: Arc<CudaContext>,
    /// Default stream for synchronous operations
    pub(crate) stream: Arc<CudaStream>,
    /// Device info discovered at runtime
    pub(crate) device_info: DeviceInfo,
    /// Module cache for PTX compilation
    pub(crate) module_cache: Arc<tokio::sync::RwLock<HashMap<String, Arc<CudaModule>>>>,
}

/// CUDA device information discovered at runtime via cudarc device queries
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Human-readable device name (e.g. "NVIDIA GeForce RTX 3090")
    pub name: String,
    /// Device ordinal (index in CUDA device list)
    pub ordinal: usize,
    /// SM compute capability as (major, minor) -- e.g. (8, 6) for Ampere
    pub compute_capability: (usize, usize),
    /// Total device memory in bytes
    pub total_memory: usize,
    /// Number of streaming multiprocessors
    pub multiprocessor_count: usize,
    /// Maximum threads per block (typically 1024)
    pub max_threads_per_block: usize,
    /// Maximum resident threads per multiprocessor
    pub max_threads_per_multiprocessor: usize,
    /// Core clock rate in kHz
    pub clock_rate_khz: usize,
    /// Memory clock rate in kHz
    pub memory_clock_rate_khz: usize,
    /// Memory bus width in bits (e.g. 384 for RTX 3090)
    pub memory_bus_width: usize,
}

pub use resource::CudaComputeResource;
