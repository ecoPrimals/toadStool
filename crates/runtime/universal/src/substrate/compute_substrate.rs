// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::error::SubstrateError;
use std::future::Future;

use super::buffer::{BufferOperation, BufferOutput, PerformanceMetrics, PowerMeasurement};
use super::capabilities::SubstrateCapabilities;
use super::substrate_kind::SubstrateType;

/// Simplified substrate trait for easier implementation
///
/// **Deep Debt**: Agnostic substrate interface, discover at runtime
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
    fn execute_buffer_op(
        &self,
        operation: BufferOperation,
    ) -> impl Future<Output = Result<BufferOutput, SubstrateError>> + Send + '_;

    /// Measure power consumption (optional, returns estimate if unavailable)
    ///
    /// **Deep Debt**: Measure actual power, don't hardcode
    fn measure_power(
        &self,
    ) -> impl Future<Output = Result<PowerMeasurement, SubstrateError>> + Send + '_ {
        let ty = self.substrate_type();
        async move { Ok(PowerMeasurement::estimated_for_type(ty)) }
    }

    /// Profile operation performance
    ///
    /// **Deep Debt**: Profile actual performance, don't hardcode
    fn profile_operation(
        &self,
        operation: &BufferOperation,
    ) -> impl Future<Output = Result<PerformanceMetrics, SubstrateError>> + Send + '_ {
        let op = operation.clone();
        async {
            let buffer_size = op.buffer_size();
            let start = std::time::Instant::now();
            let _ = self.execute_buffer_op(op).await?;
            let duration = start.elapsed();

            Ok(PerformanceMetrics {
                duration,
                throughput_ops_per_sec: if duration.as_secs_f64() > 0.0 {
                    buffer_size as f64 / duration.as_secs_f64()
                } else {
                    0.0
                },
                latency_ms: duration.as_secs_f64() * 1000.0,
            })
        }
    }
}
