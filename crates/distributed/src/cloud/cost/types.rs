// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cost types, constants, and error definitions

use thiserror::Error;
use toadstool::error::ToadStoolError;

// ─── Named Constants ─────────────────────────────────────────────────────────

/// Default spot instance discount multiplier (spot is typically 60–70% cheaper than on-demand).
pub const SPOT_DISCOUNT_FACTOR: f64 = 0.35;

/// Hours per day for daily cost calculations.
pub const HOURS_PER_DAY: f64 = 24.0;

/// Days per month for monthly cost calculations.
pub const DAYS_PER_MONTH: f64 = 30.0;

/// Bytes per GB for storage conversions.
pub const BYTES_PER_GB: u64 = 1024 * 1024 * 1024;

// ─── Structured Cost Types ───────────────────────────────────────────────────

/// Structured cost breakdown for a single resource dimension.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CostLineItem {
    /// Resource category (e.g., "cpu", "memory", "gpu", "network", "storage").
    pub category: String,
    /// Quantity (e.g., core-hours, GB-hours).
    pub quantity: f64,
    /// Unit label (e.g., "core-hours", "GB-month").
    pub unit: String,
    /// Unit price in the configured currency.
    pub unit_price: f64,
    /// Total cost for this line item.
    pub total: f64,
}

/// Full cost estimate with breakdown.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CostEstimate {
    /// Per-resource cost line items.
    pub line_items: Vec<CostLineItem>,
    /// Sum of all line items.
    pub total_cost: f64,
    /// Provider/tier identifier used for estimation.
    pub tier: String,
    /// Whether spot/preemptible pricing was applied.
    pub uses_spot: bool,
    /// Duration in hours this estimate covers.
    pub duration_hours: f64,
}

/// Cost-related errors.
#[derive(Debug, Error)]
pub enum CostError {
    #[error("Budget limit exceeded: estimate ${estimate:.2} exceeds limit ${limit:.2}")]
    BudgetExceeded { estimate: f64, limit: f64 },

    #[error("Invalid resource requirement: {0}")]
    InvalidRequirement(String),

    #[error("Cost model not found for provider: {0}")]
    ModelNotFound(String),

    #[error("Negative or zero duration for cost estimation")]
    InvalidDuration,
}

impl From<CostError> for ToadStoolError {
    fn from(e: CostError) -> Self {
        Self::resource(e.to_string())
    }
}
