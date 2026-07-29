// SPDX-License-Identifier: AGPL-3.0-or-later
//! Command routing and dispatch
//!
//! Routes parsed CLI commands to their implementations.

use tracing::info;

use crate::{CliContext, Result};

use super::definitions::Commands;
use crate::Cli;
use crate::executor::RunBiomeOptions;

mod biome;
mod ecosystem;
mod manifest;
mod server;
mod universal;

#[cfg(test)]
mod tests;

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
            let opts = RunBiomeOptions {
                manifest_path: manifest.clone(),
                name: name.clone(),
                env: env.clone(),
                debug: *debug,
                cpu_limit: *cpu_limit,
                memory_limit: memory_limit.clone(),
                security: security.clone(),
            };
            biome::execute_run(ctx, opts).await?;
        }

        Commands::Up {
            manifest,
            detach,
            name,
            env,
            restart,
            health_interval,
        } => {
            biome::execute_up(
                ctx,
                manifest.clone(),
                *detach,
                name.clone(),
                env.clone(),
                *restart,
                *health_interval,
            )
            .await?;
        }

        Commands::Down {
            biome,
            force,
            timeout,
            purge,
        } => {
            biome::execute_down(biome.as_str(), *force, *timeout, *purge).await?;
        }

        Commands::Ps {
            all,
            format,
            resources,
            status,
        } => {
            biome::execute_ps(*all, format.as_str(), *resources, status.as_deref()).await?;
        }

        Commands::Logs {
            target,
            follow,
            lines,
            timestamps,
            level,
            grep,
        } => {
            biome::execute_logs(
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
            manifest::execute_validate(manifest, *check_resources, *check_security, format).await?;
        }

        Commands::Init {
            path,
            template,
            force,
        } => {
            info!("📝 Initializing new biome manifest");
            manifest::execute_init(path, template, *force).await?;
        }

        Commands::Capabilities {
            format,
            detailed,
            test_platform,
        } => {
            info!("🌍 Showing system capabilities");
            universal::execute_capabilities(format, *detailed, test_platform).await?;
        }

        Commands::Ecosystem { action } => {
            ecosystem::execute(action).await?;
        }

        Commands::Universal { operation } => {
            universal::execute(operation).await?;
        }

        Commands::Transport { action } => {
            super::transport::execute_transport_command(action).await?;
        }

        Commands::Device { action } => {
            info!("Device lifecycle management");
            super::device::execute_device_command(action.clone()).await?;
        }

        Commands::Mode { action } => {
            info!("🖥️ GPU mode switching");
            super::mode::execute_mode_command(ctx, action.clone()).await?;
        }

        Commands::KernelHealth { format, repair } => {
            super::kernel_health::execute_kernel_health(format, *repair).await?;
        }

        Commands::Server {
            register,
            bind,
            port,
            socket,
            config,
            max_workloads,
            biomeos_socket,
            family_id,
            headless,
        }
        | Commands::Daemon {
            register,
            bind,
            port,
            socket,
            config,
            max_workloads,
            biomeos_socket,
            family_id,
            headless,
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
            if let Some(b) = bind {
                info!("   Bind: {}", b);
            }
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
            if *headless {
                info!("   Headless: enabled (skipping hardware probes)");
            }

            server::run_server_daemon(
                family_id.clone(),
                bind.clone(),
                Some(*port),
                socket.clone(),
                biomeos_socket.clone(),
                *headless,
            )
            .await?;
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
            let port = port.unwrap_or_else(toadstool_config::ports::daemon_port);
            server::run_byob_server(bind.clone(), port, config.clone()).await?;
        }

        Commands::Doctor {
            all,
            hardware,
            ecosystem,
            check_config,
            format,
            fix,
        } => {
            info!("🩺 Running system diagnostics");
            super::doctor::run_doctor(
                *all || *hardware,
                *all || *ecosystem,
                *all || *check_config,
                format,
                *fix,
            )
            .await?;
        }
    }

    Ok(())
}
