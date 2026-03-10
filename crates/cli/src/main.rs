// SPDX-License-Identifier: AGPL-3.0-only
//! ToadStool - Universal Compute Platform (`UniBin` Architecture)
//!
//! 🍄 **WELCOME TO THE FUTURE OF SOVEREIGN SCIENCE** 🍄
//!
//! ToadStool is the universal runtime environment for the ecoPrimals ecosystem.
//! It bootstraps, manages, and isolates complete biomeOS instances from declarative
//! manifest files (biome.yaml).
//!
//! 🎯 **SOVEREIGN SCIENCE**: Your compute, your data, your control
//! 🚀 **UNIVERSAL COMPUTE**: If it has a chip and memory, ToadStool runs on it
//! 🔒 **ZERO TRUST**: BearDog cryptographic security by default
//!
//! ## `UniBin` Architecture
//!
//! This is the FIRST `UniBin` primal in the ecoPrimals ecosystem!
//! One binary, multiple modes:
//! - `toadstool <command>` - CLI commands (run, up, down, etc.)
//! - `toadstool daemon` - Server/daemon mode
//! - `toadstool-server` - Backward compat (auto-runs daemon mode)

use std::io::IsTerminal;
use std::path::Path;

use clap::Parser;
use toadstool_cli::commands::dispatch;
use toadstool_cli::setup::{self, exit_codes};
use toadstool_cli::{Cli, CliContext, CliError, Result};
use tracing::{debug, error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Install interrupt handler for ecoBin exit code compliance
    tokio::spawn(async {
        tokio::signal::ctrl_c().await.ok();
        std::process::exit(exit_codes::INTERRUPTED);
    });

    // UNIBIN: Detect how we were invoked for backward compatibility
    let bin_path = std::env::args().next();
    #[allow(deprecated)]
    let default_name = toadstool_common::interned_strings::primals::TOADSTOOL;
    let bin_name = bin_path
        .as_deref()
        .and_then(|p| Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or(default_name);

    // If invoked as "toadstool-server", run in daemon mode automatically
    if bin_name == "toadstool-server" {
        info!("🍄 ToadStool invoked as 'toadstool-server' (legacy mode)");
        info!("💡 TIP: Use 'toadstool daemon' for the modern UniBin interface");
        return run_server_daemon(None).await;
    }

    // If invoked as "toadstool-byob-server", run BYOB server (UniBin migration)
    if bin_name == "toadstool-byob-server" {
        info!("🍄 ToadStool invoked as 'toadstool-byob-server' (legacy mode)");
        info!("💡 TIP: Use 'toadstool byob-server' for the modern UniBin interface");
        let config = toadstool_runtime_container::byob_server::ByobServerConfig::default();
        return toadstool_runtime_container::byob_server::run_byob_server(config)
            .await
            .map_err(|e| CliError::Other(format!("BYOB server failed: {e}")));
    }

    // Parse command line arguments
    let cli = Cli::parse();

    // Initialize logging with better formatting
    setup::init_enhanced_logging(cli.verbose)?;

    // SECURITY WARNING: Alert users about incomplete security implementations
    if std::env::var("TOADSTOOL_SECURITY_WARNING_ACKNOWLEDGED").is_err() {
        warn!("🚨 SECURITY WARNING: This ToadStool instance has incomplete cryptographic verification");
        warn!("🚨 Service discovery and permission validation are not fully implemented");
        warn!("🚨 Do NOT use in production environments without proper security audit");
        warn!("🚨 Set TOADSTOOL_SECURITY_WARNING_ACKNOWLEDGED=1 to suppress this warning");
    }

    // Print banner (only in interactive mode)
    if std::io::stdout().is_terminal() {
        setup::print_banner();
    }

    // Create CLI context
    let ctx = CliContext::new(&cli)?;

    // Record start time for operation timing
    let start_time = std::time::Instant::now();

    // Execute command with enhanced error handling
    match dispatch::execute_command(&cli, &ctx).await {
        Ok(()) => {
            let duration = start_time.elapsed();
            debug!(
                "Command executed successfully in {:.2}s",
                duration.as_secs_f64()
            );

            if duration.as_secs() > 2 {
                setup::print_success_message("Operation completed successfully!");
                setup::print_operation_summary("Command execution", duration, None);
            }

            Ok(())
        }
        Err(e) => {
            let duration = start_time.elapsed();
            error!("Command failed after {:.2}s: {}", duration.as_secs_f64(), e);

            setup::print_enhanced_error(&e);

            let code = setup::exit_code_for_error(&e);
            std::process::exit(code);
        }
    }
}

/// Run server/daemon when invoked via legacy binary name (before CLI parse)
async fn run_server_daemon(family_id: Option<String>) -> Result<()> {
    info!("🚀 Starting ToadStool server (UniBin mode)...");
    toadstool_server::run_server_main(family_id)
        .await
        .map_err(|e| CliError::Other(format!("Server failed: {e}")))
}
