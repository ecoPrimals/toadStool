// SPDX-License-Identifier: AGPL-3.0-only

use super::substrate_kind::SubstrateType;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Buffer operation
///
/// **Deep Debt**: Simple, generic operation for substrates
#[derive(Debug, Clone)]
pub enum BufferOperation {
    /// Add two buffers element-wise
    Add {
        /// First operand buffer.
        a: Vec<u8>,
        /// Second operand buffer.
        b: Vec<u8>,
        /// Element size in bytes.
        element_size: usize,
    },

    /// Multiply two buffers element-wise
    Multiply {
        /// First operand buffer.
        a: Vec<u8>,
        /// Second operand buffer.
        b: Vec<u8>,
        /// Element size in bytes.
        element_size: usize,
    },

    /// Apply unary function to buffer
    Map {
        /// Input data buffer.
        data: Vec<u8>,
        /// Element size in bytes.
        element_size: usize,
        /// Unary operation to apply.
        operation: UnaryOp,
    },

    /// Custom operation (substrate-specific)
    Custom {
        /// Operation name.
        name: String,
        /// Input data.
        data: Vec<u8>,
        /// Operation metadata.
        metadata: serde_json::Value,
    },
}

impl BufferOperation {
    /// Get the total buffer size for this operation
    #[allow(clippy::missing_const_for_fn)] // Vec::len() not const
    pub fn buffer_size(&self) -> usize {
        match self {
            Self::Add { a, b, .. } => a.len() + b.len(),
            Self::Multiply { a, b, .. } => a.len() + b.len(),
            Self::Map { data, .. } => data.len(),
            Self::Custom { data, .. } => data.len(),
        }
    }
}

/// Unary operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnaryOp {
    /// Negate (unary minus).
    Negate,
    /// Square (x²).
    Square,
    /// Square root.
    Sqrt,
    /// Exponential.
    Exp,
    /// Natural logarithm.
    Log,
}

/// Buffer operation output
#[derive(Debug, Clone, Default)]
pub struct BufferOutput {
    /// Result data
    pub data: Vec<u8>,

    /// Execution metadata
    pub metadata: BufferMetadata,
}

/// Buffer execution metadata
#[derive(Debug, Clone, Default)]
pub struct BufferMetadata {
    /// Execution duration
    pub duration: Duration,

    /// Substrate that executed this
    pub substrate_name: String,

    /// Power consumed (if measured)
    pub power_consumed_mw: Option<f64>,
}

/// Power measurement
///
/// **Deep Debt**: Actual hardware measurement, not estimates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerMeasurement {
    /// Power in watts
    pub watts: f64,

    /// Whether this is a measured value (true) or estimate (false)
    pub measured: bool,

    /// Measurement method (e.g., "RAPL", "nvidia-smi", "estimated")
    pub method: String,
}

impl PowerMeasurement {
    /// Create an estimated power measurement for a substrate type
    pub fn estimated_for_type(substrate_type: SubstrateType) -> Self {
        let watts = match substrate_type {
            SubstrateType::Cpu => 65.0,
            SubstrateType::Gpu => 250.0,
            SubstrateType::IntegratedGpu => 15.0,
            SubstrateType::Npu => 2.0,
            SubstrateType::Tpu => 200.0,
            SubstrateType::Fpga => 25.0,
            SubstrateType::Dsp => 5.0,
            SubstrateType::Quantum => 15_000.0,
        };

        Self {
            watts,
            measured: false,
            method: format!("estimated ({})", substrate_type.as_str()),
        }
    }
}

/// Performance metrics
///
/// **Deep Debt**: Actual measured performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total duration
    pub duration: Duration,

    /// Throughput (operations/second)
    pub throughput_ops_per_sec: f64,

    /// Latency (milliseconds)
    pub latency_ms: f64,
}
