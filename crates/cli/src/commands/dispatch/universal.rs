// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal compute command handlers
//!
//! Detect, Benchmark, Migrate, Federate - universal compute substrate operations.
//! Capabilities - system capability discovery.

use tracing::info;

use crate::{Result, universal::UniversalComputeManager};

use super::super::definitions::UniversalCommands;

/// Execute universal compute operations
pub async fn execute(operation: &UniversalCommands) -> Result<()> {
    let mut manager = UniversalComputeManager::new().await?;

    match operation {
        UniversalCommands::Detect {
            categories,
            test,
            output,
        } => {
            info!("🔍 Detecting universal compute substrates");
            manager
                .detect_platforms(categories.clone(), *test, output.clone())
                .await?;
        }

        UniversalCommands::Benchmark {
            suite,
            platforms,
            format,
        } => {
            info!("📊 Running benchmark suite: {}", suite);
            manager
                .run_benchmarks(suite.clone(), platforms.clone(), format.clone())
                .await?;
        }

        #[cfg(feature = "migration-preview")]
        UniversalCommands::Migrate {
            source,
            target,
            pause,
            verify,
        } => {
            info!("🚚 Migrating workload: {} → {}", source, target);
            manager
                .migrate_workload(source.clone(), target.clone(), *pause, *verify)
                .await?;
        }

        UniversalCommands::Federate {
            endpoint,
            mode,
            resources,
        } => {
            info!("🤝 Establishing federation");
            manager
                .establish_federation(endpoint.clone(), mode.clone(), resources.clone())
                .await?;
        }
    }

    Ok(())
}

/// Show system capabilities
pub async fn execute_capabilities(
    format: &str,
    detailed: bool,
    test_platform: &Option<String>,
) -> Result<()> {
    let manager = UniversalComputeManager::new().await?;

    if let Some(platform) = test_platform {
        info!("🧪 Testing platform: {}", platform);
    }

    manager.show_capabilities(format, detailed).await?;

    Ok(())
}
