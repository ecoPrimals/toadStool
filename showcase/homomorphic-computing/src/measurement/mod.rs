//! Real Hardware Measurement Infrastructure
//!
//! **Deep Debt Principle**: No hardcoded values, measure actual hardware!
//!
//! This module implements runtime measurement of:
//! - Power consumption (RAPL, nvidia-smi, Akida API)
//! - Performance (actual benchmarks, not estimates)
//! - Hardware capabilities (runtime discovery)
//!
//! ## Design Philosophy
//!
//! 1. **Measure, Don't Estimate**: Use actual hardware APIs
//! 2. **Graceful Degradation**: Fallback to estimates if APIs unavailable
//! 3. **Cross-Platform**: Linux RAPL, Windows, macOS support
//! 4. **Zero Dependencies**: Pure Rust where possible
//!
//! ## Example
//!
//! ```rust,ignore
//! use homomorphic_computing::measurement::*;
//!
//! // Measure actual CPU power
//! let cpu_power = CpuPowerMonitor::new()?;
//! let watts = cpu_power.measure_watts()?;
//! println!("CPU: {:.2}W", watts);
//! // "CPU: 24.8W" (actual measurement via RAPL)
//!
//! // Measure actual GPU power
//! let gpu_power = GpuPowerMonitor::new()?;
//! let watts = gpu_power.measure_watts()?;
//! println!("GPU: {:.2}W", watts);
//! // "GPU: 147.3W" (actual measurement via nvidia-smi)
//! ```

pub mod power;
pub mod performance;

pub use power::{CpuPowerMonitor, GpuPowerMonitor, NpuPowerMonitor, PowerMeasurement};
pub use performance::{PerformanceProfiler, PerformanceMetrics};
