//! # toadStool - Universal Runtime for ecoPrimals
//!
//! Docker-free, sovereignty-focused container orchestration with WASM-first execution.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tokio;
use tracing::{error, info, warn};

mod manifest;
mod scheduler;
mod runtimes;
mod resources;
mod federation;
mod security;
mod cli;

use crate::cli::{CliHandler, CliError};
use crate::manifest::BiomeManifest;
use crate::scheduler::WorkloadScheduler;

/// toadStool - Universal runtime for ecoPrimals biomes
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(name = "toadstool")]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Suppress logging output
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Set logging level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info", global = true)]
    log_level: String,

    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a biome from manifest
    Run {
        /// Path to biome.yaml manifest
        manifest: PathBuf,
        
        /// Run in foreground (default)
        #[arg(long)]
        foreground: bool,
    },
    
    /// Start biome in background
    Up {
        /// Path to biome.yaml manifest
        manifest: PathBuf,
        
        /// Run in detached mode
        #[arg(short, long)]
        detached: bool,
        
        /// Override biome name
        #[arg(long)]
        name: Option<String>,
    },
    
    /// List running biomes
    Ps {
        /// Output format: table, json, yaml
        #[arg(long, default_value = "table")]
        format: String,
        
        /// Show all biomes (including stopped)
        #[arg(short, long)]
        all: bool,
        
        /// Filter by biome name pattern
        #[arg(long)]
        filter: Option<String>,
    },
    
    /// View biome logs
    Logs {
        /// Biome name
        biome_name: String,
        
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
        
        /// Number of lines to show from end
        #[arg(long, default_value = "100")]
        tail: u32,
        
        /// Show timestamps
        #[arg(long)]
        timestamps: bool,
    },
    
    /// Stop a running biome
    Stop {
        /// Biome name
        biome_name: String,
        
        /// Force stop (SIGKILL)
        #[arg(long)]
        force: bool,
        
        /// Timeout in seconds
        #[arg(long, default_value = "30")]
        timeout: u64,
    },
    
    /// Federation management
    Federation {
        #[command(subcommand)]
        action: FederationCommands,
    },
    
    /// System information and health
    Info {
        /// Show detailed system information
        #[arg(long)]
        detailed: bool,
        
        /// Output format: table, json, yaml
        #[arg(long, default_value = "table")]
        format: String,
    },
    
    /// Validate a biome manifest
    Validate {
        /// Path to biome.yaml manifest
        manifest: PathBuf,
        
        /// Strict validation mode
        #[arg(long)]
        strict: bool,
    },
}

#[derive(Subcommand)]
enum FederationCommands {
    /// Show federation status
    Status {
        /// Output format: table, json, yaml
        #[arg(long, default_value = "table")]
        format: String,
    },
    
    /// List federation peers
    Peers {
        /// Show offline peers
        #[arg(long)]
        all: bool,
    },
    
    /// Join a federation
    Join {
        /// Peer address to join
        peer: String,
        
        /// Trust policy to use
        #[arg(long, default_value = "beardog_verified")]
        trust_policy: String,
    },
    
    /// Leave federation
    Leave {
        /// Force leave without graceful shutdown
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Initialize logging
    init_logging(&cli)?;
    
    info!("toadStool v{} starting...", env!("CARGO_PKG_VERSION"));
    
    // Initialize CLI handler
    let mut cli_handler = CliHandler::new(cli.config.clone()).await?;
    
    // Handle graceful shutdown
    let shutdown_signal = setup_shutdown_handler();
    
    // Execute command
    let result = match cli.command {
        Commands::Run { manifest, foreground } => {
            cli_handler.run_biome(manifest, foreground, shutdown_signal).await
        }
        
        Commands::Up { manifest, detached, name } => {
            cli_handler.start_biome(manifest, detached, name).await
        }
        
        Commands::Ps { format, all, filter } => {
            cli_handler.list_biomes(format, all, filter).await
        }
        
        Commands::Logs { biome_name, follow, tail, timestamps } => {
            cli_handler.show_logs(biome_name, follow, tail, timestamps, shutdown_signal).await
        }
        
        Commands::Stop { biome_name, force, timeout } => {
            cli_handler.stop_biome(biome_name, force, timeout).await
        }
        
        Commands::Federation { action } => {
            match action {
                FederationCommands::Status { format } => {
                    cli_handler.federation_status(format).await
                }
                FederationCommands::Peers { all } => {
                    cli_handler.federation_peers(all).await
                }
                FederationCommands::Join { peer, trust_policy } => {
                    cli_handler.federation_join(peer, trust_policy).await
                }
                FederationCommands::Leave { force } => {
                    cli_handler.federation_leave(force).await
                }
            }
        }
        
        Commands::Info { detailed, format } => {
            cli_handler.system_info(detailed, format).await
        }
        
        Commands::Validate { manifest, strict } => {
            cli_handler.validate_manifest(manifest, strict).await
        }
    };
    
    match result {
        Ok(_) => {
            info!("Command completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Command failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn init_logging(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    
    let log_level = if cli.quiet {
        "error"
    } else if cli.verbose {
        "debug"
    } else {
        &cli.log_level
    };
    
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));
    
    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    Ok(())
}

fn setup_shutdown_handler() -> tokio::sync::broadcast::Receiver<()> {
    let (tx, rx) = tokio::sync::broadcast::channel(1);
    
    tokio::spawn(async move {
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("Failed to install SIGINT handler");
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
        
        tokio::select! {
            _ = sigint.recv() => {
                info!("Received SIGINT, shutting down gracefully...");
            }
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down gracefully...");
            }
        }
        
        let _ = tx.send(());
    });
    
    rx
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert()
    }

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from(&["toadstool", "ps"]).unwrap();
        assert!(matches!(cli.command, Commands::Ps { .. }));
    }

    #[test]
    fn test_federation_commands() {
        let cli = Cli::try_parse_from(&["toadstool", "federation", "status"]).unwrap();
        assert!(matches!(cli.command, Commands::Federation { .. }));
    }
} 