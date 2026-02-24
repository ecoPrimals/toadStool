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
pub use optimizer::CloudCostOptimizer;
pub use pricing::{CloudCostModel, PricingTier};
pub use types::{CostError, CostEstimate, CostLineItem};

// Constants (re-exported for public API compatibility)
#[allow(unused_imports)]
pub use types::{BYTES_PER_GB, DAYS_PER_MONTH, HOURS_PER_DAY, SPOT_DISCOUNT_FACTOR};
