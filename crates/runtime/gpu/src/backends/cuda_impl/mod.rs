// SPDX-License-Identifier: AGPL-3.0-or-later
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

/// CUDA device information discovered at runtime
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub ordinal: usize,
    pub compute_capability: (usize, usize),
    pub total_memory: usize,
    pub multiprocessor_count: usize,
    pub max_threads_per_block: usize,
    pub max_threads_per_multiprocessor: usize,
    pub clock_rate_khz: usize,
    pub memory_clock_rate_khz: usize,
    pub memory_bus_width: usize,
}

pub use resource::CudaComputeResource;
