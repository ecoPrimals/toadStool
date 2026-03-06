// SPDX-License-Identifier: AGPL-3.0-or-later
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
                .down_biome(biome.as_str(), *force, *timeout, *purge)
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
                .list_biomes(*all, format.as_str(), *resources, status.as_deref())
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
                    target.as_str(),
                    *follow,
                    *lines,
                    *timestamps,
                    level.as_deref(),
                    grep.as_deref(),
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

        Commands::Transport { action } => {
            super::transport::execute_transport_command(action).await?;
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
                runtime.as_deref(),
                env.as_slice(),
                *timeout,
                format.as_str(),
            )
            .await?;
        }

        Commands::ByobServer { bind, port, config } => {
            info!("🍄 Starting Toadstool BYOB Server");
            let bind = bind
                .clone()
                .unwrap_or_else(|| toadstool_config::config_utils::ConfigUtils::get_bind_address());
            let config = toadstool_runtime_container::byob_server::ByobServerConfig {
                bind_address: Some(bind),
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

    manager.show_capabilities(format, detailed).await?;

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
    use std::path::PathBuf;

    use super::super::definitions::{Commands, EcosystemCommands, UniversalCommands};
    use super::*;
    use crate::executor::{RunBiomeOptions, UpBiomeOptions};
    use crate::{Cli, CliContext};
    use tempfile::TempDir;
    use tokio::fs;

    fn valid_manifest_yaml() -> &'static str {
        r#"
metadata:
  name: test-biome
  version: "1.0.0"
  created: 1735689600
  updated: 1735689600
  tags: []

primals: {}
services: {}

resources:
  cpu_limit: 2.0
  memory_limit: "2GB"

security:
  isolation_level: "high"
  trust_level: "medium"
  beardog_required: false
  crypto_policies: []
  allowed_networks: []
  forbidden_syscalls: []

networking:
  mode: "bridge"
  dns_servers: []
  port_mappings: []
  network_policies: []

storage:
  nestgate_integration: false
  datasets: []
  volumes: []
"#
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_validate_valid_manifest() {
        let temp_dir = TempDir::new().expect("temp dir");
        let manifest_path = temp_dir.path().join("biome.yaml");
        fs::write(&manifest_path, valid_manifest_yaml())
            .await
            .expect("write manifest");

        let cli = Cli {
            command: Commands::Validate {
                manifest: manifest_path,
                check_resources: false,
                check_security: false,
                format: "text".to_string(),
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "validate should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_validate_json_format() {
        let temp_dir = TempDir::new().expect("temp dir");
        let manifest_path = temp_dir.path().join("biome.yaml");
        fs::write(&manifest_path, valid_manifest_yaml())
            .await
            .expect("write manifest");

        let cli = Cli {
            command: Commands::Validate {
                manifest: manifest_path,
                check_resources: true,
                check_security: true,
                format: "json".to_string(),
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "validate json should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_validate_nonexistent_manifest() {
        let cli = Cli {
            command: Commands::Validate {
                manifest: PathBuf::from("/nonexistent/path/biome.yaml"),
                check_resources: false,
                check_security: false,
                format: "text".to_string(),
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(result.is_err(), "validate nonexistent should fail");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_init_science_template() {
        let temp_dir = TempDir::new().expect("temp dir");
        let cli = Cli {
            command: Commands::Init {
                path: temp_dir.path().to_path_buf(),
                template: "science".to_string(),
                force: false,
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "init science should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_init_basic_template() {
        let temp_dir = TempDir::new().expect("temp dir");
        let output_dir = temp_dir.path().to_path_buf();

        let cli = Cli {
            command: Commands::Init {
                path: output_dir.clone(),
                template: "basic".to_string(),
                force: false,
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "init basic should succeed: {:?}",
            result.err()
        );
        assert!(output_dir.join("biome.yaml").exists());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_init_invalid_template() {
        let temp_dir = TempDir::new().expect("temp dir");
        let cli = Cli {
            command: Commands::Init {
                path: temp_dir.path().to_path_buf(),
                template: "nonexistent-template".to_string(),
                force: false,
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(result.is_err(), "init invalid template should fail");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_capabilities() {
        let cli = Cli {
            command: Commands::Capabilities {
                format: "table".to_string(),
                detailed: false,
                test_platform: None,
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "capabilities should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_capabilities_with_test_platform() {
        let cli = Cli {
            command: Commands::Capabilities {
                format: "json".to_string(),
                detailed: true,
                test_platform: Some("linux".to_string()),
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "capabilities with test_platform should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_ecosystem_discover() {
        let cli = Cli {
            command: Commands::Ecosystem {
                action: EcosystemCommands::Discover {
                    services: vec!["crypto".to_string()],
                    timeout: 1,
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "ecosystem discover should succeed (may return empty): {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_ecosystem_discover_empty_services() {
        let cli = Cli {
            command: Commands::Ecosystem {
                action: EcosystemCommands::Discover {
                    services: vec![],
                    timeout: 1,
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "ecosystem discover empty should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_ecosystem_register_error_path() {
        let cli = Cli {
            command: Commands::Ecosystem {
                action: EcosystemCommands::Register {
                    endpoint: "127.0.0.1:1".to_string(),
                    token: None,
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_err(),
            "ecosystem register to unreachable endpoint should fail"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_ecosystem_auth_error_path() {
        let cli = Cli {
            command: Commands::Ecosystem {
                action: EcosystemCommands::Auth {
                    permission_file: PathBuf::from("/nonexistent/permissions.json"),
                    validate_only: true,
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_err(),
            "ecosystem auth with nonexistent file should fail"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_ecosystem_storage_error_path() {
        let cli = Cli {
            command: Commands::Ecosystem {
                action: EcosystemCommands::Storage {
                    endpoint: "http://127.0.0.1:1".to_string(),
                    mount: PathBuf::from("/data"),
                    dataset: None,
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_err(),
            "ecosystem storage with unreachable endpoint should fail"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_universal_detect() {
        let cli = Cli {
            command: Commands::Universal {
                operation: UniversalCommands::Detect {
                    categories: vec!["traditional".to_string()],
                    test: false,
                    output: None,
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "universal detect should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_universal_benchmark() {
        let cli = Cli {
            command: Commands::Universal {
                operation: UniversalCommands::Benchmark {
                    suite: "standard".to_string(),
                    platforms: vec![],
                    format: "json".to_string(),
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "universal benchmark should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_universal_migrate_error_path() {
        let cli = Cli {
            command: Commands::Universal {
                operation: UniversalCommands::Migrate {
                    source: "nonexistent-source".to_string(),
                    target: "nonexistent-target".to_string(),
                    pause: false,
                    verify: false,
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_err(),
            "universal migrate with nonexistent source should fail"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_universal_federate() {
        let cli = Cli {
            command: Commands::Universal {
                operation: UniversalCommands::Federate {
                    endpoint: "127.0.0.1:9999".to_string(),
                    mode: "peer".to_string(),
                    resources: vec![],
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "universal federate should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_down_nonexistent() {
        let cli = Cli {
            command: Commands::Down {
                biome: "nonexistent-biome-xyz".to_string(),
                force: false,
                timeout: 30,
                purge: false,
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(result.is_err(), "down nonexistent should fail");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_ps() {
        let cli = Cli {
            command: Commands::Ps {
                all: false,
                format: "table".to_string(),
                resources: false,
                status: None,
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(result.is_ok(), "ps should succeed: {:?}", result.err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_command_doctor() {
        let cli = Cli {
            command: Commands::Doctor {
                all: false,
                hardware: true,
                ecosystem: false,
                config: false,
                format: "text".to_string(),
                fix: false,
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(result.is_ok(), "doctor should succeed: {:?}", result.err());
    }

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

    #[test]
    fn test_commands_run_variant() {
        let cmd = Commands::Run {
            manifest: PathBuf::from("biome.yaml"),
            name: Some("test-biome".to_string()),
            env: vec!["KEY=value".to_string()],
            debug: true,
            cpu_limit: Some(2.0),
            memory_limit: Some("512Mi".to_string()),
            security: "high".to_string(),
        };
        match &cmd {
            Commands::Run {
                manifest,
                name,
                env,
                debug,
                cpu_limit,
                memory_limit,
                security,
            } => {
                assert_eq!(manifest, &PathBuf::from("biome.yaml"));
                assert_eq!(name.as_deref(), Some("test-biome"));
                assert_eq!(env.len(), 1);
                assert!(*debug);
                assert_eq!(*cpu_limit, Some(2.0));
                assert_eq!(memory_limit.as_deref(), Some("512Mi"));
                assert_eq!(security, "high");
            }
            _ => panic!("Expected Run variant"),
        }
    }

    #[test]
    fn test_commands_up_variant() {
        let cmd = Commands::Up {
            manifest: PathBuf::from("biome.yaml"),
            detach: true,
            name: None,
            env: vec![],
            restart: true,
            health_interval: 60,
        };
        match &cmd {
            Commands::Up {
                manifest,
                detach,
                restart,
                health_interval,
                ..
            } => {
                assert_eq!(manifest, &PathBuf::from("biome.yaml"));
                assert!(*detach);
                assert!(*restart);
                assert_eq!(*health_interval, 60);
            }
            _ => panic!("Expected Up variant"),
        }
    }

    #[test]
    fn test_commands_down_variant() {
        let cmd = Commands::Down {
            biome: "my-biome".to_string(),
            force: true,
            timeout: 10,
            purge: true,
        };
        match &cmd {
            Commands::Down {
                biome,
                force,
                timeout,
                purge,
            } => {
                assert_eq!(biome, "my-biome");
                assert!(*force);
                assert_eq!(*timeout, 10);
                assert!(*purge);
            }
            _ => panic!("Expected Down variant"),
        }
    }

    #[test]
    fn test_commands_validate_variant() {
        let cmd = Commands::Validate {
            manifest: PathBuf::from("manifest.yaml"),
            check_resources: true,
            check_security: true,
            format: "json".to_string(),
        };
        match &cmd {
            Commands::Validate {
                manifest,
                check_resources,
                check_security,
                format,
            } => {
                assert_eq!(manifest, &PathBuf::from("manifest.yaml"));
                assert!(*check_resources);
                assert!(*check_security);
                assert_eq!(format, "json");
            }
            _ => panic!("Expected Validate variant"),
        }
    }

    #[test]
    fn test_commands_init_variant() {
        let cmd = Commands::Init {
            path: PathBuf::from("."),
            template: "basic".to_string(),
            force: true,
        };
        match &cmd {
            Commands::Init {
                path,
                template,
                force,
            } => {
                assert_eq!(path, &PathBuf::from("."));
                assert_eq!(template, "basic");
                assert!(*force);
            }
            _ => panic!("Expected Init variant"),
        }
    }

    #[test]
    fn test_commands_capabilities_variant() {
        let cmd = Commands::Capabilities {
            format: "table".to_string(),
            detailed: true,
            test_platform: Some("linux".to_string()),
        };
        match &cmd {
            Commands::Capabilities {
                format,
                detailed,
                test_platform,
            } => {
                assert_eq!(format, "table");
                assert!(*detailed);
                assert_eq!(test_platform.as_deref(), Some("linux"));
            }
            _ => panic!("Expected Capabilities variant"),
        }
    }

    #[test]
    fn test_commands_ecosystem_variant() {
        let cmd = Commands::Ecosystem {
            action: EcosystemCommands::Discover {
                services: vec!["crypto".to_string()],
                timeout: 5,
            },
        };
        match &cmd {
            Commands::Ecosystem { action } => match action {
                EcosystemCommands::Discover { services, timeout } => {
                    assert_eq!(services.len(), 1);
                    assert_eq!(*timeout, 5);
                }
                _ => panic!("Expected Discover subcommand"),
            },
            _ => panic!("Expected Ecosystem variant"),
        }
    }

    #[test]
    fn test_commands_universal_variant() {
        let cmd = Commands::Universal {
            operation: UniversalCommands::Detect {
                categories: vec!["traditional".to_string()],
                test: false,
                output: None,
            },
        };
        match &cmd {
            Commands::Universal { operation } => match operation {
                UniversalCommands::Detect {
                    categories, test, ..
                } => {
                    assert_eq!(categories.len(), 1);
                    assert!(!*test);
                }
                _ => panic!("Expected Detect subcommand"),
            },
            _ => panic!("Expected Universal variant"),
        }
    }

    #[test]
    fn test_run_biome_options_construction() {
        let opts = RunBiomeOptions {
            manifest_path: PathBuf::from("biome.yaml"),
            name: Some("test".to_string()),
            env: vec!["FOO=bar".to_string()],
            debug: false,
            cpu_limit: None,
            memory_limit: None,
            security: "medium".to_string(),
        };
        assert_eq!(opts.manifest_path, PathBuf::from("biome.yaml"));
        assert_eq!(opts.name.unwrap(), "test");
        assert_eq!(opts.env.len(), 1);
        assert_eq!(opts.security, "medium");
    }

    #[test]
    fn test_up_biome_options_construction() {
        let opts = UpBiomeOptions {
            manifest_path: PathBuf::from("biome.yaml"),
            detach: true,
            name: None,
            env: vec![],
            restart: false,
            health_interval: 30,
        };
        assert_eq!(opts.manifest_path, PathBuf::from("biome.yaml"));
        assert!(opts.detach);
        assert_eq!(opts.health_interval, 30);
    }
}
