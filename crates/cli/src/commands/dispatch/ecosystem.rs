// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ecosystem integration command handlers
//!
//! Discover, Register, Auth, Storage - ecoPrimals ecosystem integration.
//!
//! NOTE: Uses legacy EcosystemIntegrator which internally calls deprecated service modules.
//! Migration to Adapters planned for v0.2.0.

use colored::Colorize;
use tracing::info;

use crate::{Result, ecosystem::EcosystemIntegrator};

use super::super::definitions::EcosystemCommands;

/// Execute ecosystem integration commands
pub async fn execute(action: &EcosystemCommands) -> Result<()> {
    let mut integrator = EcosystemIntegrator::new();

    match action {
        EcosystemCommands::Discover { services, timeout } => {
            info!("🔍 Discovering ecosystem services");
            let result = integrator
                .discover_services(services.clone(), *timeout)
                .await?;

            println!("🎯 Discovery Results:");
            println!("  Services Found: {}", result.total_discovered);
            println!("  Verified: {}", result.verified_count);
            println!(
                "  Scan Duration: {:.2}s",
                result.scan_duration.as_secs_f64()
            );

            for service in &result.services {
                println!(
                    "  {} {:?} - {:?} ({})",
                    "✅".green(),
                    service.service_type,
                    service.trust_level,
                    service.address
                );
            }
        }

        EcosystemCommands::Register { endpoint, token } => {
            info!("🎯 Registering with orchestrator");
            integrator
                .register_with_orchestrator(endpoint.clone(), token.clone())
                .await?;
        }

        EcosystemCommands::Auth {
            permission_file,
            validate_only,
        } => {
            info!("🔐 Installing crypto permissions");
            integrator
                .install_crypto_permissions(permission_file.clone(), *validate_only)
                .await?;
        }

        EcosystemCommands::Storage {
            endpoint,
            mount,
            dataset,
        } => {
            info!("🏠 Connecting to storage service");
            let mount_info = integrator
                .connect_distributed_storage(endpoint.clone(), mount.clone(), dataset.clone())
                .await?;

            println!("{} storage service connected:", "✅".green());
            println!("  Dataset: {}", mount_info.dataset_name);
            println!("  Mount Point: {}", mount_info.mount_point.display());
            println!("  Access Mode: {}", mount_info.access_mode);
        }
    }

    Ok(())
}
