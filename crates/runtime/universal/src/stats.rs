// SPDX-License-Identifier: AGPL-3.0-or-later

/// Runtime statistics
#[derive(Debug, Default)]
pub struct RuntimeStats {
    /// Number of CPU units
    pub num_cpu: usize,
    /// Number of GPU units
    pub num_gpu: usize,
    /// Number of neuromorphic units
    pub num_neuromorphic: usize,
    /// Number of custom units
    pub num_custom: usize,
    /// Total memory across all units (bytes)
    pub total_memory: usize,
    /// Total compute throughput (ops/sec)
    pub total_compute_throughput: f64,
}

impl std::fmt::Display for RuntimeStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Universal Compute Runtime Statistics:")?;
        writeln!(f, "  CPU units: {}", self.num_cpu)?;
        writeln!(f, "  GPU units: {}", self.num_gpu)?;
        writeln!(f, "  Neuromorphic units: {}", self.num_neuromorphic)?;
        writeln!(f, "  Custom units: {}", self.num_custom)?;
        writeln!(
            f,
            "  Total memory: {:.2} GB",
            self.total_memory as f64 / 1e9
        )?;
        writeln!(
            f,
            "  Total throughput: {:.2} GFLOPS",
            self.total_compute_throughput / 1e9
        )?;
        Ok(())
    }
}
