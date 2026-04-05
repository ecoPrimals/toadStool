// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cost optimization across clouds
//!
//! This module provides real cost estimation based on resource requirements (CPU, GPU, memory,
//! network), capability-based pricing tiers, cost capping, and budget enforcement.

mod optimizer;
mod pricing;
#[cfg(test)]
mod tests;
mod types;

// Re-exports for public API
/// Cost optimizer: estimation, provider models, and budget enforcement.
pub use optimizer::CloudCostOptimizer;
/// Capability-based pricing tier and rate bundle (`CloudCostModel`).
pub use pricing::{CloudCostModel, PricingTier};
/// Structured estimates, line items, and cost errors.
pub use types::{CostError, CostEstimate, CostLineItem};

// Constants (re-exported for public API compatibility)
#[expect(unused_imports)]
/// Conversion and pricing constants for cost estimation.
pub use types::{BYTES_PER_GB, DAYS_PER_MONTH, HOURS_PER_DAY, SPOT_DISCOUNT_FACTOR};
