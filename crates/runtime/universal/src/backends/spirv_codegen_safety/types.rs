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
    /// Single-precision f32.
    F32,
    /// Double-precision f64.
    F64,
    /// High-precision f64 (transcendentals).
    F64Precise,
    /// Double-float f32-pair emulation.
    Df64,
}

/// Capability of a single precision tier.
#[derive(Debug, Clone)]
pub struct TierCapability {
    /// Precision tier.
    pub tier: PrecisionTier,
    /// Whether shaders compile for this tier.
    pub compiles: bool,
    /// Whether dispatches succeed.
    pub dispatches: bool,
    /// Whether transcendentals are safe.
    pub transcendentals_safe: bool,
    /// Dispatch latency ratio vs f32 baseline.
    pub dispatch_latency_ratio: f64,
}

/// Precision requirement hint from science domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrecisionHint {
    /// Precision-critical workload (e.g. numerics).
    Critical,
    /// Moderate precision requirements.
    Moderate,
    /// Throughput-bound, precision secondary.
    ThroughputBound,
    /// Low precision acceptable.
    LowPrecision,
}
