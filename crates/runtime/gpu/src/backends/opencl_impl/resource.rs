// SPDX-License-Identifier: AGPL-3.0-or-later
//! OpenCL Compute Resource - UniversalComputeResource implementation
//!
//! Wraps OpenClBackend and provides utilization monitoring, execution time
//! estimation, and context creation for the universal compute scheduler.

use super::backend::OpenClBackend;
use super::context::OpenClComputeContext;
use crate::universal::*;
use async_trait::async_trait;
use ocl::Device;
use std::sync::Arc;
use toadstool::error::ToadStoolResult;
use uuid::Uuid;

/// OpenCL compute resource implementation
pub struct OpenClComputeResource {
    backend: Arc<OpenClBackend>,
    resource_id: String,
    /// Cached capabilities to avoid recomputation
    capabilities: ComputeCapabilities,
}

impl OpenClComputeResource {
    /// Create new OpenCL compute resource
    pub fn new() -> ToadStoolResult<Self> {
        let backend = OpenClBackend::new()?;
        let resource_id = format!(
            "opencl-{}",
            backend.device_info.name.replace(' ', "-").to_lowercase()
        );
        let capabilities = backend.capabilities();

        Ok(Self {
            backend: Arc::new(backend),
            resource_id,
            capabilities,
        })
    }

    /// Create with custom device selector
    pub fn with_selector<F>(selector: F) -> ToadStoolResult<Self>
    where
        F: FnOnce(Vec<Device>) -> Option<Device>,
    {
        let backend = OpenClBackend::with_device_selector(selector)?;
        let resource_id = format!(
            "opencl-{}",
            backend.device_info.name.replace(' ', "-").to_lowercase()
        );
        let capabilities = backend.capabilities();

        Ok(Self {
            backend: Arc::new(backend),
            resource_id,
            capabilities,
        })
    }

    /// Query GPU utilization from system
    ///
    /// Uses capability-based detection: tries multiple monitoring sources
    async fn query_gpu_utilization(&self) -> Option<f32> {
        // Try nvidia-smi for NVIDIA GPUs
        if self
            .backend
            .device_info
            .vendor
            .to_lowercase()
            .contains("nvidia")
            && let Ok(output) = tokio::process::Command::new("nvidia-smi")
                .args([
                    "--query-gpu=utilization.gpu",
                    "--format=csv,noheader,nounits",
                ])
                .output()
                .await
            && output.status.success()
            && let Ok(stdout) = String::from_utf8(output.stdout)
            && let Ok(util) = stdout.trim().parse::<f32>()
        {
            return Some(util / 100.0);
        }

        // Try radeontop for AMD GPUs
        if self
            .backend
            .device_info
            .vendor
            .to_lowercase()
            .contains("amd")
        {
            // AMD monitoring would go here
            // radeontop, rocm-smi, etc.
        }

        // Try intel_gpu_top for Intel GPUs
        if self
            .backend
            .device_info
            .vendor
            .to_lowercase()
            .contains("intel")
        {
            // Intel monitoring would go here
        }

        // No monitoring available - return None (caller falls back to 0.0)
        None
    }

    /// Estimate execution time from requirements
    ///
    /// Performance model based on device capabilities and workload characteristics
    fn estimate_time_from_requirements(
        &self,
        requirements: &ComputeRequirements,
    ) -> std::time::Duration {
        // Calculate estimated FLOPs needed
        let estimated_flops = requirements.estimated_operations.unwrap_or(1_000_000) as f64;

        // Get device peak performance
        let peak_flops = self.capabilities.performance.peak_flops;

        // Account for sustained performance (typically 70-90% of peak)
        let sustained_percent =
            self.capabilities.performance.sustained_performance_percent as f64 / 100.0;
        let effective_flops = peak_flops * sustained_percent;

        // Calculate compute time
        let compute_seconds = estimated_flops / effective_flops;

        // Add memory transfer overhead
        let data_bytes = requirements.memory_bytes as f64;
        let bandwidth = self.capabilities.memory.bandwidth_bytes_per_sec as f64;
        let transfer_seconds = (data_bytes * 2.0) / bandwidth; // 2x for upload + download

        // Add kernel launch overhead
        let launch_overhead_seconds =
            self.capabilities.performance.startup_latency_us as f64 / 1_000_000.0;

        // Total time
        let total_seconds = compute_seconds + transfer_seconds + launch_overhead_seconds;

        // Add 20% buffer for scheduling/context switching
        let buffered_seconds = total_seconds * 1.2;

        std::time::Duration::from_secs_f64(buffered_seconds.max(0.001))
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl UniversalComputeResource for OpenClComputeResource {
    fn capabilities(&self) -> &ComputeCapabilities {
        // Capabilities cached at construction time - zero overhead
        &self.capabilities
    }

    fn resource_id(&self) -> &str {
        &self.resource_id
    }

    async fn create_context(&self) -> ToadStoolResult<Box<dyn ComputeContext>> {
        Ok(Box::new(OpenClComputeContext {
            backend: Arc::clone(&self.backend),
            context_id: Uuid::new_v4(),
            resource_id: self.resource_id.clone(),
        }))
    }

    async fn utilization(&self) -> f32 {
        // Query actual GPU utilization from system
        // Uses capability-based detection: checks what monitoring is available
        self.query_gpu_utilization().await.unwrap_or(0.0)
    }

    fn estimate_execution_time(&self, requirements: &ComputeRequirements) -> std::time::Duration {
        // Model-based estimation using device capabilities
        self.estimate_time_from_requirements(requirements)
    }
}
