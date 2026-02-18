//! Resource optimization for collaborative intelligence

mod allocation;
mod cost;
mod types;

use tracing::info;

use crate::graph_types::ExecutionGraph;
use crate::resource_estimator::ResourceEstimator;
use crate::resource_validator::SystemCapabilities;

pub use allocation::{discover_opportunities, identify_bottlenecks};
pub use cost::{estimate_improvement, rank_by_priority};
pub use types::{
    Bottleneck, BottleneckType, ImprovementEstimate, Opportunity, OpportunityType,
    OptimizationSuggestions,
};

/// Optimization error
#[derive(Debug, Clone, thiserror::Error)]
pub enum OptimizationError {
    #[error("Estimation failed: {0}")]
    EstimationFailed(#[from] crate::resource_estimator::EstimationError),

    #[error("System query failed: {0}")]
    SystemQueryFailed(String),

    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
}

/// Resource optimizer
pub struct ResourceOptimizer {
    estimator: ResourceEstimator,
}

impl Default for ResourceOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceOptimizer {
    pub fn new() -> Self {
        Self {
            estimator: ResourceEstimator::new(),
        }
    }

    pub async fn suggest_optimizations(
        &self,
        graph: &ExecutionGraph,
    ) -> Result<OptimizationSuggestions, OptimizationError> {
        info!(
            "Analyzing graph for optimization opportunities: {}",
            graph.id
        );

        let estimate = self
            .estimator
            .estimate(graph)
            .map_err(OptimizationError::EstimationFailed)?;

        let capabilities = self.query_system_capabilities().await?;

        let bottlenecks = identify_bottlenecks(graph, &estimate, &capabilities);
        let opportunities = discover_opportunities(graph, &estimate, &capabilities);
        let improvement = estimate_improvement(&estimate, &opportunities);
        let priority_order = rank_by_priority(&opportunities);

        info!(
            "Found {} bottlenecks and {} optimization opportunities for graph {}",
            bottlenecks.len(),
            opportunities.len(),
            graph.id
        );

        Ok(OptimizationSuggestions {
            graph_id: graph.id.clone(),
            bottlenecks,
            opportunities,
            estimated_improvement: improvement,
            priority_order,
        })
    }

    async fn query_system_capabilities(&self) -> Result<SystemCapabilities, OptimizationError> {
        let total_cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4);
        let available_cpu_cores = (total_cpu_cores as f32 * 0.8) as u32;

        let mut system = sysinfo::System::new_all();
        system.refresh_memory();

        let total_memory_bytes = system.total_memory();
        let available_memory_bytes = system.available_memory();
        let total_storage_bytes = system.total_swap();
        let available_storage_bytes = system.free_swap();

        let (total_gpu_memory_bytes, available_gpu_memory_bytes, gpu_count, gpu_types) =
            Self::query_gpu_capabilities().await;

        Ok(SystemCapabilities {
            total_cpu_cores,
            available_cpu_cores,
            total_memory_bytes,
            available_memory_bytes,
            total_gpu_memory_bytes,
            available_gpu_memory_bytes,
            total_storage_bytes,
            available_storage_bytes,
            network_bandwidth_mbps: 1000,
            gpu_count,
            gpu_types,
        })
    }

    #[cfg(feature = "gpu-discovery")]
    async fn query_gpu_capabilities() -> (u64, u64, usize, Vec<String>) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapters: Vec<_> = instance.enumerate_adapters(wgpu::Backends::all());
        if adapters.is_empty() {
            return (0, 0, 0, Vec::new());
        }
        let mut total_memory: u64 = 0;
        let mut gpu_types = Vec::new();
        let mut gpu_count = 0usize;
        for adapter in &adapters {
            let info = adapter.get_info();
            if matches!(
                info.device_type,
                wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
            ) {
                gpu_count += 1;
                let estimated_memory = match info.device_type {
                    wgpu::DeviceType::DiscreteGpu => 8 * 1024 * 1024 * 1024,
                    wgpu::DeviceType::IntegratedGpu => 2 * 1024 * 1024 * 1024,
                    _ => 0,
                };
                total_memory += estimated_memory;
                gpu_types.push(info.name.clone());
            }
        }
        let available_memory = (total_memory as f64 * 0.8) as u64;
        (total_memory, available_memory, gpu_count, gpu_types)
    }

    #[cfg(not(feature = "gpu-discovery"))]
    async fn query_gpu_capabilities() -> (u64, u64, usize, Vec<String>) {
        (0, 0, 0, Vec::new())
    }
}

#[cfg(test)]
mod tests;
