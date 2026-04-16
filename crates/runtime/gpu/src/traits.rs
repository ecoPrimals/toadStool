// SPDX-License-Identifier: AGPL-3.0-or-later
//! Trait definitions for Universal GPU Compute Runtime

use super::types::{
    CompiledKernel, DeviceId, DeviceRequirements, DeviceUsage, GpuFramework, KernelFormat,
    KernelInput, KernelOutput, UniversalComputeDevice,
};
use std::collections::HashMap;
use toadstool::error::ToadStoolResult;
use uuid::Uuid;

/// Trait for parallel compute frameworks
pub trait ParallelComputeFramework: Send + Sync {
    /// Get framework type
    fn framework_type(&self) -> GpuFramework;

    /// Discover available devices
    async fn discover_devices(&self) -> ToadStoolResult<Vec<UniversalComputeDevice>>;

    /// Create compute session
    async fn create_session(&self, device_id: &DeviceId) -> ToadStoolResult<Uuid>;

    /// Compile kernel for device
    async fn compile_kernel(
        &self,
        session_id: Uuid,
        kernel_source: &str,
        format: KernelFormat,
    ) -> ToadStoolResult<CompiledKernel>;

    /// Execute compiled kernel
    async fn execute_kernel(
        &self,
        session_id: Uuid,
        kernel: &CompiledKernel,
        inputs: &[KernelInput],
    ) -> ToadStoolResult<KernelOutput>;

    /// Destroy compute session
    async fn destroy_session(&self, session_id: Uuid) -> ToadStoolResult<()>;

    /// Get device usage information
    async fn get_device_usage(&self, device_id: &DeviceId) -> ToadStoolResult<DeviceUsage>;

    /// Check if framework supports recursive execution
    fn supports_recursion(&self) -> bool;

    /// Spawn recursive compute session
    async fn spawn_recursive_session(
        &self,
        parent_session: Uuid,
        device_id: &DeviceId,
    ) -> ToadStoolResult<Uuid>;
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
