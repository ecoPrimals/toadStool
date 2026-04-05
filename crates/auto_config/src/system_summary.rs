// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ecosystem::{DiscoveredServices, EcosystemDiscoverer};
use crate::error::ToadStoolResult;
use crate::hardware::{HardwareDetector, SystemCapabilities};

/// System information summary for display and debugging.
#[derive(Debug, Clone)]
pub struct SystemSummary {
    /// CPU model and core count.
    pub cpu_info: String,
    /// Memory size summary.
    pub memory_info: String,
    /// GPU count or type.
    pub gpu_info: String,
    /// Storage capacity summary.
    pub storage_info: String,
    /// Discovered ecosystem services.
    pub ecosystem_services: Vec<String>,
    /// Performance class (e.g. low, medium, high).
    pub performance_class: String,
    /// Recommended runtime types.
    pub optimal_runtimes: Vec<String>,
}

impl SystemSummary {
    /// Create a system summary from detected capabilities
    #[must_use]
    pub fn from_capabilities(
        capabilities: &SystemCapabilities,
        ecosystem: &DiscoveredServices,
    ) -> Self {
        Self {
            cpu_info: format!(
                "{} ({} cores)",
                capabilities.cpu_info.model_name, capabilities.cpu_cores
            ),
            memory_info: format!("{:.1} GB", capabilities.memory_gb),
            gpu_info: if capabilities.gpu_count > 0 {
                format!("{} GPU(s)", capabilities.gpu_count)
            } else {
                "Integrated Graphics".to_string()
            },
            storage_info: format!(
                "{:.1} GB {:?}",
                capabilities.storage_gb, capabilities.storage_info.storage_type
            ),
            ecosystem_services: ecosystem.discovered_services.keys().cloned().collect(),
            performance_class: format!("{:?}", capabilities.performance_class),
            optimal_runtimes: vec!["Native".to_string()], // Would be determined by configuration
        }
    }

    /// Pretty print the system summary
    pub fn display(&self) {
        println!("🖥️  System Summary:");
        println!("   CPU: {}", self.cpu_info);
        println!("   Memory: {}", self.memory_info);
        println!("   GPU: {}", self.gpu_info);
        println!("   Storage: {}", self.storage_info);
        println!("   Performance: {}", self.performance_class);
        println!(
            "   Ecosystem Services: {}",
            if self.ecosystem_services.is_empty() {
                "None".to_string()
            } else {
                self.ecosystem_services.join(", ")
            }
        );
        println!("   Optimal Runtimes: {}", self.optimal_runtimes.join(", "));
    }
}

/// Get a human-readable system summary
///
/// This function performs basic hardware detection and ecosystem discovery
/// to provide a summary of the system capabilities and available services.
///
/// # Examples
///
/// ```rust,no_run
/// use toadstool_auto_config::get_system_summary;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let summary = get_system_summary().await?;
///     summary.display();
///     Ok(())
/// }
/// ```
pub async fn get_system_summary() -> ToadStoolResult<SystemSummary> {
    let mut hardware_detector = HardwareDetector::new();
    let mut ecosystem_discoverer = EcosystemDiscoverer::new();

    // Run hardware detection and ecosystem discovery sequentially (both need &mut self)
    let capabilities = hardware_detector.scan_system().await?;
    let ecosystem = ecosystem_discoverer.discover_services().await?;

    Ok(SystemSummary::from_capabilities(&capabilities, &ecosystem))
}
