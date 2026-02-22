// BarraCuda Validation Library
// Shared utilities for validation benchmarks

pub mod power_measurement;

pub use power_measurement::{query_cpu_power, query_gpu_power, query_npu_power};
