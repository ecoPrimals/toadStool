// SPDX-License-Identifier: AGPL-3.0-only

use crate::error::SubstrateError;
use async_trait::async_trait;

use super::buffer::{BufferOperation, BufferOutput, PerformanceMetrics, PowerMeasurement};
use super::capabilities::SubstrateCapabilities;
use super::substrate_kind::SubstrateType;

/// Simplified substrate trait for easier implementation
///
/// **Deep Debt**: Agnostic substrate interface, discover at runtime
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait ComputeSubstrate: Send + Sync {
    /// Human-readable name
    fn name(&self) -> &str;

    /// Substrate type (CPU, GPU, NPU, TPU)
    fn substrate_type(&self) -> SubstrateType;

    /// Get substrate capabilities
    fn capabilities(&self) -> SubstrateCapabilities {
        SubstrateCapabilities::default_for_type(self.substrate_type())
    }

    /// Execute a buffer operation
    ///
    /// **Deep Debt**: Simple, generic operation interface
    async fn execute_buffer_op(
        &self,
        operation: BufferOperation,
    ) -> Result<BufferOutput, SubstrateError>;

    /// Measure power consumption (optional, returns estimate if unavailable)
    ///
    /// **Deep Debt**: Measure actual power, don't hardcode
    async fn measure_power(&self) -> Result<PowerMeasurement, SubstrateError> {
        // Default: Estimate based on substrate type
        Ok(PowerMeasurement::estimated_for_type(self.substrate_type()))
    }

    /// Profile operation performance
    ///
    /// **Deep Debt**: Profile actual performance, don't hardcode
    async fn profile_operation(
        &self,
        operation: &BufferOperation,
    ) -> Result<PerformanceMetrics, SubstrateError> {
        let start = std::time::Instant::now();
        let _ = self.execute_buffer_op(operation.clone()).await?;
        let duration = start.elapsed();

        Ok(PerformanceMetrics {
            duration,
            throughput_ops_per_sec: if duration.as_secs_f64() > 0.0 {
                operation.buffer_size() as f64 / duration.as_secs_f64()
            } else {
                0.0
            },
            latency_ms: duration.as_secs_f64() * 1000.0,
        })
    }
}
