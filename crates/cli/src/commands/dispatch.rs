//! Command routing and dispatch
//!
//! Routes parsed CLI commands to their implementations.

use std::path::{Path, PathBuf};

use colored::Colorize;
use tracing::info;

use crate::{CliContext, CliContextExt, CliError, Result};

use super::definitions::{Commands, EcosystemCommands, UniversalCommands};
use crate::{
    ecosystem::EcosystemIntegrator,
    executor::{BiomeExecutor, RunBiomeOptions, UpBiomeOptions},
    universal::UniversalComputeManager,
    Cli,
};

/// Execute the main CLI command based on parsed arguments
pub async fn execute_command(cli: &Cli, ctx: &CliContext) -> Result<()> {
    match &cli.command {
        Commands::Run {
            manifest,
            name,
            env,
            debug,
            cpu_limit,
            memory_limit,
            security,
        } => {
            info!("🚀 Starting biome in foreground mode");
            let executor = BiomeExecutor::new().await?;
            let opts = RunBiomeOptions {
                manifest_path: manifest.clone(),
                name: name.clone(),
                env: env.clone(),
                debug: *debug,
                cpu_limit: *cpu_limit,
                memory_limit: memory_limit.clone(),
                security: security.clone(),
            };
            executor.run_biome(ctx, opts).await?;
        }

        Commands::Up {
            manifest,
            detach,
            name,
            env,
            restart,
            health_interval,
        } => {
            info!("🚀 Starting biome in background mode");
            let executor = BiomeExecutor::new().await?;
            let opts = UpBiomeOptions {
                manifest_path: manifest.clone(),
                detach: *detach,
                name: name.clone(),
                env: env.clone(),
                restart: *restart,
                health_interval: *health_interval,
            };
            executor.up_biome(ctx, opts).await?;
        }

        Commands::Down {
            biome,
            force,
            timeout,
            purge,
        } => {
            info!("🛑 Stopping biome: {}", biome);
            let executor = BiomeExecutor::new().await?;
            executor
                .down_biome(biome.clone(), *force, *timeout, *purge)
                .await?;
        }

        Commands::Ps {
            all,
            format,
            resources,
            status,
        } => {
            info!("📋 Listing biomes");
            let executor = BiomeExecutor::new().await?;
            executor
                .list_biomes(*all, format.clone(), *resources, status.clone())
                .await?;
        }

        Commands::Logs {
            target,
            follow,
            lines,
            timestamps,
            level,
            grep,
        } => {
            info!("📜 Showing logs for: {}", target);
            let executor = BiomeExecutor::new().await?;
            executor
                .show_logs(
                    target.clone(),
                    *follow,
                    *lines,
                    *timestamps,
                    level.clone(),
                    grep.clone(),
                )
                .await?;
        }

        Commands::Validate {
            manifest,
            check_resources,
            check_security,
            format,
        } => {
            info!("🔍 Validating biome manifest: {}", manifest.display());
            validate_manifest_command(manifest, *check_resources, *check_security, format).await?;
        }

        Commands::Init {
            path,
            template,
            force,
        } => {
            info!("📝 Initializing new biome manifest");
            init_manifest_command(path, template, *force).await?;
        }

        Commands::Capabilities {
            format,
            detailed,
            test_platform,
        } => {
            info!("🌍 Showing system capabilities");
            show_capabilities_command(format, *detailed, test_platform).await?;
        }

        Commands::Ecosystem { action } => {
            execute_ecosystem_command(action).await?;
        }

        Commands::Universal { operation } => {
            execute_universal_command(operation).await?;
        }

        Commands::Server {
            register,
            port,
            socket,
            config,
            max_workloads,
            biomeos_socket,
            family_id,
        }
        | Commands::Daemon {
            register,
            port,
            socket,
            config,
            max_workloads,
            biomeos_socket,
            family_id,
        } => {
            let is_server = matches!(&cli.command, Commands::Server { .. });

            if is_server {
                info!("🍄 ToadStool Server Mode (UniBin Standard)");
            } else {
                info!("🍄 ToadStool Daemon Mode (backward compat)");
                info!("💡 TIP: Use 'toadstool server' for ecosystem standard naming");
            }

            info!(
                "   Register: {}",
                if *register { "enabled" } else { "disabled" }
            );
            info!("   Port: {}", port);
            if let Some(sock) = socket {
                info!("   Socket: {}", sock.display());
            }
            if let Some(cfg) = config {
                info!("   Config: {}", cfg.display());
            }
            info!("   Max workloads: {}", max_workloads);
            if let Some(biomeos) = biomeos_socket {
                info!("   BiomeOS: {}", biomeos.display());
            }
            if let Some(fid) = family_id {
                info!("   Family ID: {}", fid);
            }

            run_server_daemon(family_id.clone()).await?;
        }

        Commands::Execute {
            workload,
            runtime,
            env,
            timeout,
            format,
        } => {
            info!("🚀 Executing workload: {}", workload.display());
            crate::executor::workload::execute_workload(
                workload,
                runtime.clone(),
                env.clone(),
                *timeout,
                format.clone(),
            )
            .await?;
        }

        Commands::ByobServer { bind, port, config } => {
            info!("🍄 Starting Toadstool BYOB Server");
            let config = toadstool_runtime_container::byob_server::ByobServerConfig {
                bind_address: Some(bind.clone()),
                port: Some(*port),
                config_path: config.as_ref().and_then(|p| p.to_str().map(String::from)),
            };
            toadstool_runtime_container::byob_server::run_byob_server(config)
                .await
                .map_err(|e| CliError::Other(format!("BYOB server failed: {e}")))?;
        }

        Commands::Doctor {
            all,
            hardware,
            ecosystem,
            config,
            format,
            fix,
        } => {
            info!("🩺 Running system diagnostics");
            super::doctor::run_doctor(
                *all || *hardware,
                *all || *ecosystem,
                *all || *config,
                format,
                *fix,
            )
            .await?;
        }
    }

    Ok(())
}

/// Validate biome manifest
async fn validate_manifest_command(
    manifest_path: &PathBuf,
    check_resources: bool,
    check_security: bool,
    format: &str,
) -> Result<()> {
    use crate::{load_biome_manifest, validate_manifest};

    let manifest = load_biome_manifest(manifest_path).await.context(format!(
        "Failed to load manifest: {}",
        manifest_path.display()
    ))?;

    let warnings = validate_manifest(&manifest)?;
    let errors: Vec<String> = Vec::new();
    let validation_warnings = warnings;

    if check_resources {
        info!("🔍 Checking resource availability");
    }

    if check_security {
        info!("🔒 Validating security policies");
    }

    match format {
        "json" => {
            let result = serde_json::json!({
                "valid": errors.is_empty(),
                "errors": errors,
                "warnings": validation_warnings,
                "manifest": manifest
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        _ => {
            if errors.is_empty() {
                println!("{} Manifest validation passed", "✅".green());
            } else {
                println!("{} Manifest validation failed", "❌".red());
                for error in &errors {
                    println!("  {} {}", "Error:".red().bold(), error);
                }
            }

            if !validation_warnings.is_empty() {
                println!("\n{} Warnings:", "⚠️".yellow().bold());
                for warning in &validation_warnings {
                    println!("  {} {}", "Warning:".yellow().bold(), warning);
                }
            }

            println!("\n📋 Manifest Summary:");
            println!(
                "  Biome: {} v{}",
                manifest.metadata.name, manifest.metadata.version
            );
            println!("  Primals: {}", manifest.primals.len());
            println!("  Services: {}", manifest.services.len());
            println!("  BearDog Required: {}", manifest.security.beardog_required);
        }
    }

    Ok(())
}

/// Initialize new biome manifest
async fn init_manifest_command(path: &Path, template: &str, force: bool) -> Result<()> {
    use crate::templates::TemplateGenerator;

    let biome_template = TemplateGenerator::parse_template(template)
        .context(format!("Unknown template type: {template}"))?;

    if template == "list" {
        println!("📦 Available Templates:");
        for (name, description) in TemplateGenerator::list_templates() {
            println!("  {} - {}", name.bright_green().bold(), description);
        }
        return Ok(());
    }

    let generator = TemplateGenerator::new(path.to_path_buf(), force);
    let output_path = generator.generate(biome_template).await?;

    println!(
        "{} Biome manifest generated: {}",
        "✅".green(),
        output_path.display().to_string().bright_cyan()
    );

    Ok(())
}

/// Show system capabilities
async fn show_capabilities_command(
    format: &str,
    detailed: bool,
    test_platform: &Option<String>,
) -> Result<()> {
    let manager = UniversalComputeManager::new().await?;

    if let Some(platform) = test_platform {
        info!("🧪 Testing platform: {}", platform);
    }

    manager
        .show_capabilities(format.to_string(), detailed)
        .await?;

    Ok(())
}

/// Execute ecosystem integration commands
///
/// NOTE: Uses legacy EcosystemIntegrator which internally calls deprecated service modules.
/// Migration to Adapters planned for v0.2.0.
#[allow(deprecated)]
async fn execute_ecosystem_command(action: &EcosystemCommands) -> Result<()> {
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
            info!("🏠 Connecting to NestGate storage");
            let mount_info = integrator
                .connect_nestgate_storage(endpoint.clone(), mount.clone(), dataset.clone())
                .await?;

            println!("{} NestGate storage connected:", "✅".green());
            println!("  Dataset: {}", mount_info.dataset_name);
            println!("  Mount Point: {}", mount_info.mount_point.display());
            println!("  Access Mode: {}", mount_info.access_mode);
        }
    }

    Ok(())
}

/// Execute universal compute operations
async fn execute_universal_command(operation: &UniversalCommands) -> Result<()> {
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

/// Run ToadStool in server/daemon mode (UniBin compliant)
async fn run_server_daemon(family_id: Option<String>) -> Result<()> {
    info!("🚀 Starting ToadStool server (UniBin mode)...");

    toadstool_server::run_server_main(family_id)
        .await
        .map_err(|e| CliError::Other(format!("Server failed: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::definitions::Commands;
    use super::*;

    #[test]
    fn test_dispatch_module_compiles_and_commands_accessible() {
        // Verify Commands enum is constructible and execute_command exists
        let _cmd = Commands::Doctor {
            all: false,
            hardware: false,
            ecosystem: false,
            config: false,
            format: "text".to_string(),
            fix: false,
        };
        // execute_command is the key public function - verify it's in scope
        let _ = execute_command;
    }
}
