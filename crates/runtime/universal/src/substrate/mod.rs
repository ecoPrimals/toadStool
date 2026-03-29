// SPDX-License-Identifier: AGPL-3.0-only
//! Simplified Substrate Abstraction
//!
//! **Deep Debt**: Agnostic, capability-based substrate interface
//!
//! This module provides a simplified trait for compute substrates that's easier
//! to implement for specific workloads (like homomorphic encryption) while still
//! being compatible with the full `ComputeUnit` trait.
//!
//! # Architecture
//!
//! ```text
//! ComputeSubstrate (Simple)    ComputeUnit (Full)
//!        │                            │
//!        ├── Simple operations        ├── Complex workloads
//!        ├── Buffer management        ├── Scheduling
//!        └── Power measurement        └── Capability discovery
//!                │                            │
//!                └────────────────────────────┘
//!                      SubstrateAdapter
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use toadstool_runtime_universal::substrate::*;
//!
//! struct MyGpuSubstrate {
//!     device: WgpuDevice,
//! }
//!
//! #[async_trait::async_trait]
//! impl ComputeSubstrate for MyGpuSubstrate {
//!     fn name(&self) -> &str { "My GPU" }
//!     fn substrate_type(&self) -> SubstrateType { SubstrateType::Gpu }
//!     
//!     async fn execute_buffer_op(
//!         &self,
//!         operation: BufferOperation,
//!     ) -> Result<BufferOutput> {
//!         // Implement GPU-specific operation
//!         Ok(BufferOutput::default())
//!     }
//! }
//! ```

mod adapter;
mod buffer;
mod capabilities;
mod compute_substrate;
mod substrate_kind;

#[cfg(test)]
mod tests;

pub use adapter::SubstrateAdapter;
pub use buffer::{
    BufferMetadata, BufferOperation, BufferOutput, PerformanceMetrics, PowerMeasurement, UnaryOp,
};
pub use capabilities::SubstrateCapabilities;
pub use compute_substrate::ComputeSubstrate;
pub use substrate_kind::SubstrateType;
