// SPDX-License-Identifier: AGPL-3.0-or-later
//! Trait definitions for Universal GPU Compute Runtime

use super::types::{
    CompiledKernel, DeviceId, DeviceRequirements, DeviceUsage, GpuFramework, KernelFormat,
    KernelInput, KernelOutput, UniversalComputeDevice,
};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use toadstool::error::ToadStoolResult;
use uuid::Uuid;

/// Trait for parallel compute frameworks
pub trait ParallelComputeFramework: Send + Sync {
    /// Get framework type
    fn framework_type(&self) -> GpuFramework;

    /// Discover available devices
    fn discover_devices(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<UniversalComputeDevice>>> + Send + '_>>;

    /// Create compute session
    fn create_session<'a>(
        &'a self,
        device_id: &'a DeviceId,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Uuid>> + Send + 'a>>;

    /// Compile kernel for device
    fn compile_kernel<'a>(
        &'a self,
        session_id: Uuid,
        kernel_source: &'a str,
        format: KernelFormat,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<CompiledKernel>> + Send + 'a>>;

    /// Execute compiled kernel
    fn execute_kernel<'a>(
        &'a self,
        session_id: Uuid,
        kernel: &'a CompiledKernel,
        inputs: &'a [KernelInput],
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<KernelOutput>> + Send + 'a>>;

    /// Destroy compute session
    fn destroy_session(
        &self,
        session_id: Uuid,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Get device usage information
    fn get_device_usage<'a>(
        &'a self,
        device_id: &'a DeviceId,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<DeviceUsage>> + Send + 'a>>;

    /// Check if framework supports recursive execution
    fn supports_recursion(&self) -> bool;

    /// Spawn recursive compute session
    fn spawn_recursive_session<'a>(
        &'a self,
        parent_session: Uuid,
        device_id: &'a DeviceId,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Uuid>> + Send + 'a>>;
}

/// Trait for kernel optimization
pub trait KernelOptimizer: Send + Sync {
    /// Optimize kernel for specific device
    ///
    /// # Errors
    ///
    /// Returns when optimization cannot be applied for this kernel or device.
    fn optimize(&self, kernel: &str, device: &UniversalComputeDevice) -> ToadStoolResult<String>;

    /// Get supported optimization passes
    fn supported_passes(&self) -> Vec<String>;
}

/// Trait for load balancing
pub trait LoadBalancer: Send + Sync {
    /// Select optimal device for new workload
    ///
    /// # Errors
    ///
    /// Returns when no device satisfies the requirements or selection fails.
    fn select_device(
        &self,
        devices: &[DeviceId],
        requirements: &DeviceRequirements,
    ) -> ToadStoolResult<DeviceId>;

    /// Update device load information
    fn update_device_load(&mut self, device_id: &DeviceId, usage: &DeviceUsage);

    /// Get load balancing statistics
    fn get_statistics(&self) -> HashMap<String, f64>;
}
