// SPDX-License-Identifier: AGPL-3.0-only

/// NVVM poisoning risk classification inferred from driver identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvvmPoisoningRisk {
    /// Safe: NVK/Mesa driver handles all tiers correctly.
    None,
    /// Risk: proprietary NVIDIA driver; DF64/F64Precise transcendentals may poison device.
    TranscendentalOnly,
    /// Unknown driver — treat as risky.
    Unknown,
}

/// Precision tier (from hotSpring).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrecisionTier {
    F32,
    F64,
    F64Precise,
    Df64,
}

/// Capability of a single precision tier.
#[derive(Debug, Clone)]
pub struct TierCapability {
    pub tier: PrecisionTier,
    pub compiles: bool,
    pub dispatches: bool,
    pub transcendentals_safe: bool,
    pub dispatch_latency_ratio: f64,
}

/// Precision requirement hint from science domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrecisionHint {
    Critical,
    Moderate,
    ThroughputBound,
    LowPrecision,
}
