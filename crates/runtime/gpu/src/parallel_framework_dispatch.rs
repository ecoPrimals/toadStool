// SPDX-License-Identifier: AGPL-3.0-or-later
//! Enum dispatch for [`ParallelComputeFramework`](crate::traits::ParallelComputeFramework).

use crate::frameworks::{FallbackFramework, WebGpuFramework};
use crate::traits::ParallelComputeFramework;
use crate::types::{
    CompiledKernel, DeviceId, DeviceUsage, GpuFramework, KernelFormat, KernelInput, KernelOutput,
    UniversalComputeDevice,
};
use toadstool::error::ToadStoolResult;
use uuid::Uuid;

/// Closed set of parallel compute framework implementations.
pub enum ParallelComputeFrameworkDispatch {
    /// `WebGPU` (`wgpu`) framework.
    WebGpu(WebGpuFramework),
    /// Placeholder when a framework is unavailable on the current platform.
    Fallback(FallbackFramework),
}

impl ParallelComputeFramework for ParallelComputeFrameworkDispatch {
    fn framework_type(&self) -> GpuFramework {
        match self {
            Self::WebGpu(f) => f.framework_type(),
            Self::Fallback(f) => f.framework_type(),
        }
    }

    async fn discover_devices(&self) -> ToadStoolResult<Vec<UniversalComputeDevice>> {
        match self {
            Self::WebGpu(f) => f.discover_devices().await,
            Self::Fallback(f) => f.discover_devices().await,
        }
    }

    async fn create_session(&self, device_id: &DeviceId) -> ToadStoolResult<Uuid> {
        match self {
            Self::WebGpu(f) => f.create_session(device_id).await,
            Self::Fallback(f) => f.create_session(device_id).await,
        }
    }

    async fn compile_kernel(
        &self,
        session_id: Uuid,
        kernel_source: &str,
        format: KernelFormat,
    ) -> ToadStoolResult<CompiledKernel> {
        match self {
            Self::WebGpu(f) => f.compile_kernel(session_id, kernel_source, format).await,
            Self::Fallback(f) => f.compile_kernel(session_id, kernel_source, format).await,
        }
    }

    async fn execute_kernel(
        &self,
        session_id: Uuid,
        kernel: &CompiledKernel,
        inputs: &[KernelInput],
    ) -> ToadStoolResult<KernelOutput> {
        match self {
            Self::WebGpu(f) => f.execute_kernel(session_id, kernel, inputs).await,
            Self::Fallback(f) => f.execute_kernel(session_id, kernel, inputs).await,
        }
    }

    async fn destroy_session(&self, session_id: Uuid) -> ToadStoolResult<()> {
        match self {
            Self::WebGpu(f) => f.destroy_session(session_id).await,
            Self::Fallback(f) => f.destroy_session(session_id).await,
        }
    }

    async fn get_device_usage(&self, device_id: &DeviceId) -> ToadStoolResult<DeviceUsage> {
        match self {
            Self::WebGpu(f) => f.get_device_usage(device_id).await,
            Self::Fallback(f) => f.get_device_usage(device_id).await,
        }
    }

    fn supports_recursion(&self) -> bool {
        match self {
            Self::WebGpu(f) => f.supports_recursion(),
            Self::Fallback(f) => f.supports_recursion(),
        }
    }

    async fn spawn_recursive_session(
        &self,
        parent_session: Uuid,
        device_id: &DeviceId,
    ) -> ToadStoolResult<Uuid> {
        match self {
            Self::WebGpu(f) => f.spawn_recursive_session(parent_session, device_id).await,
            Self::Fallback(f) => f.spawn_recursive_session(parent_session, device_id).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frameworks::FallbackFramework;
    use crate::traits::ParallelComputeFramework;
    use crate::types::{DeviceId, GpuFramework};

    #[tokio::test]
    async fn fallback_dispatch_discovers_no_devices() {
        let fb = FallbackFramework::new(GpuFramework::Vulkan);
        let d = ParallelComputeFrameworkDispatch::Fallback(fb);
        assert_eq!(d.framework_type(), GpuFramework::Vulkan);
        let devs = d.discover_devices().await.unwrap();
        assert!(devs.is_empty());
    }

    #[tokio::test]
    async fn fallback_dispatch_session_errors_without_hardware() {
        let fb = FallbackFramework::new(GpuFramework::WebGpu);
        let d = ParallelComputeFrameworkDispatch::Fallback(fb);
        let id = DeviceId {
            framework: GpuFramework::WebGpu,
            device_index: 0,
            uuid: "test".to_string(),
        };
        assert!(d.create_session(&id).await.is_err());
    }
}
