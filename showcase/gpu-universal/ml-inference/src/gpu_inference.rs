//! GPU inference using ToadStool's universal compute abstraction

use crate::{network::SimpleNetwork, InferenceResult};
use anyhow::Result;
use ndarray::Array1;
use std::time::Instant;
use toadstool_runtime_gpu::{
    scheduler::{SchedulingPolicy, UniversalComputeScheduler},
    strategy::BackendSelectionStrategy,
    types::GpuFramework,
};

/// GPU inference using ToadStool's universal compute runtime
pub struct GpuInference {
    network: SimpleNetwork,
    #[allow(dead_code)] // Will be used when GPU resources are registered
    scheduler: UniversalComputeScheduler,
    preferred_backend: Option<GpuFramework>,
}

impl GpuInference {
    /// Create new GPU inference with automatic backend selection
    pub async fn new(network: SimpleNetwork) -> Result<Self> {
        let scheduler = UniversalComputeScheduler::new(SchedulingPolicy::Performance);

        // TODO: Register GPU resources
        // For now, will fall back to CPU

        Ok(Self {
            network,
            scheduler,
            preferred_backend: None,
        })
    }

    /// Create with specific backend (CUDA, ROCm, WebGPU, etc.)
    pub async fn with_backend(network: SimpleNetwork, backend: GpuFramework) -> Result<Self> {
        let scheduler = UniversalComputeScheduler::new(SchedulingPolicy::Performance);

        Ok(Self {
            network,
            scheduler,
            preferred_backend: Some(backend),
        })
    }

    /// Run inference using universal compute abstraction
    pub async fn infer(&self, input: &Array1<f32>) -> Result<InferenceResult> {
        let start = Instant::now();

        // Select backend through ToadStool's abstraction
        let backend = if let Some(ref backend) = self.preferred_backend {
            backend.clone()
        } else {
            // Use automatic selection
            let strategy = BackendSelectionStrategy::Automatic;
            let available = vec![GpuFramework::WebGpu, GpuFramework::Cuda];
            // For ML inference, use AiMl workload type
            let workload_type = toadstool::WorkloadType::AiMl;
            strategy
                .select_framework(Some(&workload_type), &available)
                .unwrap_or_else(|| {
                    tracing::warn!("No GPU backend available, falling back to CPU");
                    GpuFramework::Custom("CPU".to_string())
                })
        };

        // Execute workload
        // The universal abstraction would route this to the appropriate backend
        // For now, we fall back to CPU execution (proving CPU fallback works!)
        let output = self.network.forward_cpu(input)?;

        // Get prediction
        let (predicted_class, confidence) = self.network.predict(&output);

        let latency = start.elapsed();

        Ok(InferenceResult {
            predicted_class,
            confidence,
            all_probabilities: output.to_vec(),
            latency,
            backend: format!("{backend:?} (via ToadStool, CPU fallback)"),
        })
    }

    /// Get current backend being used
    pub fn current_backend(&self) -> String {
        if let Some(ref backend) = self.preferred_backend {
            format!("{backend:?}")
        } else {
            "Automatic (CPU fallback)".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gpu_inference_creation() {
        let network = SimpleNetwork::new();
        let inference = GpuInference::new(network).await.unwrap();

        assert!(
            inference.current_backend().contains("Automatic")
                || inference.current_backend().contains("CPU")
        );
    }

    #[tokio::test]
    async fn test_gpu_inference_with_backend() {
        let network = SimpleNetwork::new();
        let inference = GpuInference::with_backend(network, GpuFramework::Cuda)
            .await
            .unwrap();

        assert!(inference.current_backend().contains("Cuda"));
    }
}
