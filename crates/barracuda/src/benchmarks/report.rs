//! Benchmark Report Generation
//!
//! Generate comprehensive reports from benchmark results

use super::ComparisonResult;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Report generator
pub struct ReportGenerator {
    results: Vec<ComparisonResult>,
}

impl ReportGenerator {
    /// Create new report generator
    pub fn new(results: Vec<ComparisonResult>) -> Self {
        Self { results }
    }

    /// Generate markdown report
    pub fn generate_markdown(&self) -> String {
        let mut report = String::new();

        report.push_str("# BarraCUDA vs CUDA Performance Report\n\n");

        #[cfg(feature = "benchmarks")]
        {
            report.push_str(&format!("**Generated:** {}\n\n", chrono::Local::now()));
        }

        #[cfg(not(feature = "benchmarks"))]
        {
            report.push_str("**Generated:** [timestamp not available]\n\n");
        }

        report.push_str("## Summary\n\n");

        // Compute summary statistics
        let total_ops = self.results.len();
        if total_ops > 0 {
            let ops_with_cuda: Vec<_> = self.results.iter().filter(|r| r.cuda.is_some()).collect();
            let ops_at_90 = ops_with_cuda
                .iter()
                .filter(|r| r.parity_percent >= 90.0)
                .count();
            let ops_at_95 = ops_with_cuda
                .iter()
                .filter(|r| r.parity_percent >= 95.0)
                .count();
            let ops_at_100 = ops_with_cuda
                .iter()
                .filter(|r| r.parity_percent >= 100.0)
                .count();

            let avg_parity = if !ops_with_cuda.is_empty() {
                ops_with_cuda.iter().map(|r| r.parity_percent).sum::<f64>()
                    / ops_with_cuda.len() as f64
            } else {
                0.0
            };

            report.push_str("| Metric | Value |\n");
            report.push_str("|--------|-------|\n");
            report.push_str(&format!("| Total Operations | {} |\n", total_ops));
            report.push_str(&format!(
                "| Operations with CUDA comparison | {} |\n",
                ops_with_cuda.len()
            ));
            report.push_str(&format!(
                "| ≥90% parity | {} ({:.1}%) |\n",
                ops_at_90,
                ops_at_90 as f64 / ops_with_cuda.len().max(1) as f64 * 100.0
            ));
            report.push_str(&format!(
                "| ≥95% parity | {} ({:.1}%) |\n",
                ops_at_95,
                ops_at_95 as f64 / ops_with_cuda.len().max(1) as f64 * 100.0
            ));
            report.push_str(&format!(
                "| ≥100% parity (faster) | {} ({:.1}%) |\n",
                ops_at_100,
                ops_at_100 as f64 / ops_with_cuda.len().max(1) as f64 * 100.0
            ));
            report.push_str(&format!("| Average parity | {:.1}% |\n", avg_parity));
            report.push('\n');
        }

        report.push_str("## Detailed Results\n\n");
        for result in &self.results {
            report.push_str(&format!("### {}\n\n", result.operation));
            report.push_str(&format!("- **Hardware:** {}\n", result.hardware));
            report.push_str(&format!(
                "- **BarraCUDA:** {:.3}ms\n",
                result.barracuda.median_time.as_secs_f64() * 1000.0
            ));
            if let Some(ref cuda) = result.cuda {
                report.push_str(&format!(
                    "- **CUDA:** {:.3}ms\n",
                    cuda.median_time.as_secs_f64() * 1000.0
                ));
                report.push_str(&format!("- **Parity:** {:.1}%\n", result.parity_percent));
            }
            report.push('\n');
        }

        report
    }

    /// Save report to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let report = self.generate_markdown();
        let mut file = File::create(path)?;
        file.write_all(report.as_bytes())?;
        Ok(())
    }
}
