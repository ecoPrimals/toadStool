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
        // TODO: Add summary table
        
        report.push_str("## Detailed Results\n\n");
        for result in &self.results {
            report.push_str(&format!("### {}\n\n", result.operation));
            report.push_str(&format!("- **Hardware:** {}\n", result.hardware));
            report.push_str(&format!("- **BarraCUDA:** {:.3}ms\n", 
                result.barracuda.median_time.as_secs_f64() * 1000.0));
            if let Some(ref cuda) = result.cuda {
                report.push_str(&format!("- **CUDA:** {:.3}ms\n", 
                    cuda.median_time.as_secs_f64() * 1000.0));
                report.push_str(&format!("- **Parity:** {:.1}%\n", result.parity_percent));
            }
            report.push_str("\n");
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
