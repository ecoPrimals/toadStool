//! Capability Display Operations
//!
//! Extension trait for displaying platform capabilities and summaries.

use crate::Result;
use std::future::Future;
use tracing::info;

/// Capability display operations trait
pub trait CapabilityDisplayOps {
    /// Print detection summary
    fn print_detection_summary(&self) -> impl Future<Output = Result<()>> + Send;

    /// Print benchmark table
    fn print_benchmark_table(&self) -> impl Future<Output = Result<()>> + Send;

    /// Print capabilities table
    fn print_capabilities_table(&self, detailed: bool) -> impl Future<Output = Result<()>> + Send;
}

/// Implementation of capability display operations
impl CapabilityDisplayOps for crate::universal::UniversalComputeManager {
    async fn print_detection_summary(&self) -> Result<()> {
        info!("🎯 Detection Summary");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        let total_platforms = self.platforms.len();
        let available = self
            .platforms
            .values()
            .filter(|p| matches!(p.status, crate::universal::types::PlatformStatus::Available))
            .count();

        info!("📊 Total Platforms: {}", total_platforms);
        info!("✅ Available: {}", available);
        info!("⚠️  Other: {}", total_platforms - available);

        // Group by category
        let mut categories: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for platform_id in self.platforms.keys() {
            let category = platform_id.split('_').next().unwrap_or("unknown");
            *categories.entry(category.to_string()).or_insert(0) += 1;
        }

        info!("\n📋 By Category:");
        for (category, count) in categories.iter() {
            info!("  {} {}: {}", "▪️", category, count);
        }

        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        Ok(())
    }

    async fn print_benchmark_table(&self) -> Result<()> {
        info!("📊 Benchmark Results");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        if self.benchmarks.is_empty() {
            info!("No benchmarks run yet");
            return Ok(());
        }

        // Print header
        info!("{:<30} {:<15} {:<15}", "Platform", "Suite", "Score");
        info!("{}", "─".repeat(60));

        // Print each benchmark
        for (platform_id, result) in &self.benchmarks {
            info!(
                "{:<30} {:<15} {:<15.2}",
                platform_id, result.suite, result.overall_score
            );
        }

        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        Ok(())
    }

    async fn print_capabilities_table(&self, detailed: bool) -> Result<()> {
        info!("🎯 Platform Capabilities");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        if self.platforms.is_empty() {
            info!("No platforms detected yet");
            return Ok(());
        }

        // Print header
        if detailed {
            info!("{:<30} {:<15} {:<40}", "Platform", "Status", "Type");
        } else {
            info!("{:<30} {:<15}", "Platform", "Status");
        }
        info!("{}", "─".repeat(60));

        // Print each platform
        for (platform_id, platform) in &self.platforms {
            let status_str = match &platform.status {
                crate::universal::types::PlatformStatus::Available => "✅ Available",
                crate::universal::types::PlatformStatus::Testing => "🧪 Testing",
                crate::universal::types::PlatformStatus::Degraded => "⚠️  Degraded",
                crate::universal::types::PlatformStatus::Unavailable => "❌ Unavailable",
                crate::universal::types::PlatformStatus::Error(_) => "🔴 Error",
            };

            if detailed {
                let type_str = format!("{:?}", platform.platform_type);
                info!("{:<30} {:<15} {:<40}", platform_id, status_str, type_str);
            } else {
                info!("{:<30} {:<15}", platform_id, status_str);
            }
        }

        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        Ok(())
    }
}
