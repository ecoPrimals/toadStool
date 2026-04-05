// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hardware and compute constants

/// Default GPU max parallel compute invocations per workgroup dimension
pub const DEFAULT_MAX_COMPUTE_INVOCATIONS: u32 = 1024;

/// Default max buffer size (1 `GiB`)
pub const DEFAULT_MAX_BUFFER_SIZE: u64 = 1_073_741_824;

/// Default hash bucket count for unique operations
pub const DEFAULT_HASH_BUCKETS: u32 = 65536;

/// GELU approximation constant (used in activation functions)
pub const GELU_APPROX_COEFF: f64 = 0.044_715;

/// GELU tanh scaling constant
pub const GELU_TANH_SCALE: f64 = 0.842_7;

/// Error function approximation coefficient
pub const ERF_APPROX_A1: f64 = 0.327_591_1;

/// Default DRI render node path (Linux kernel GPU subsystem)
pub const DEFAULT_DRI_CARD: &str = "/dev/dri/card0";
