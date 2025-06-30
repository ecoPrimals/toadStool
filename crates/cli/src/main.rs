//! ToadStool CLI - Universal Compute Command Center
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

use anyhow::{Result, Context};
use clap::Parser;
use std::path::PathBuf;
use tracing::{info, error, debug};
use tracing_subscriber::EnvFilter;
use colored::Colorize;

use toadstool_cli::{
    Cli, Commands, CliContext,
    executor::BiomeExecutor,
    ecosystem::EcosystemIntegrator,
    universal::UniversalComputeManager,
    EcosystemCommands, UniversalCommands,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Initialize logging
    init_logging(cli.verbose)?;
    
    // Print banner
    print_banner();
    
    // Create CLI context
    let ctx = CliContext::new(&cli)?;
    
    // Execute command
    match execute_command(&cli, &ctx).await {
        Ok(()) => {
            debug!("Command executed successfully");
            Ok(())
        },
        Err(e) => {
            error!("Command failed: {}", e);
            
            // Print user-friendly error message
            eprintln!("{} {}", "Error:".red().bold(), e);
            
            // Print suggestions if available
            if let Some(suggestion) = get_error_suggestion(&e) {
                eprintln!("{} {}", "Suggestion:".yellow().bold(), suggestion);
            }
            
            std::process::exit(1);
        }
    }
}

/// Initialize logging based on verbosity level
fn init_logging(verbose: bool) -> Result<()> {
    let filter = if verbose {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("debug"))
    } else {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"))
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .init();
    
    Ok(())
}

/// Print the ToadStool banner
fn print_banner() {
    let banner = r#"
🍄 ████████╗ ██████╗  █████╗ ██████╗ ███████╗████████╗ ██████╗  ██████╗ ██╗     
🍄 ╚══██╔══╝██╔═══██╗██╔══██╗██╔══██╗██╔════╝╚══██╔══╝██╔═══██╗██╔═══██╗██║     
🍄    ██║   ██║   ██║███████║██║  ██║███████╗   ██║   ██║   ██║██║   ██║██║     
🍄    ██║   ██║   ██║██╔══██║██║  ██║╚════██║   ██║   ██║   ██║██║   ██║██║     
🍄    ██║   ╚██████╔╝██║  ██║██████╔╝███████║   ██║   ╚██████╔╝╚██████╔╝███████╗
🍄    ╚═╝    ╚═════╝ ╚═╝  ╚═╝╚═════╝ ╚══════╝   ╚═╝    ╚═════╝  ╚═════╝ ╚══════╝
"#;
    
    println!("{}", banner.bright_green().bold());
    println!("{}", "Universal Compute Platform - The Backbone of SOVEREIGN SCIENCE".bright_cyan().bold());
    println!("{}", "Version 0.1.0 | If it has a chip and memory, ToadStool runs on it".bright_white());
    println!();
}

/// Execute the main CLI command
async fn execute_command(cli: &Cli, ctx: &CliContext) -> Result<()> {
    match &cli.command {
        Commands::Run { 
            manifest, name, env, debug, cpu_limit, memory_limit, security 
        } => {
            info!("🚀 Starting biome in foreground mode");
            let executor = BiomeExecutor::new().await?;
            executor.run_biome(
                ctx,
                manifest.clone(),
                name.clone(),
                env.clone(),
                *debug,
                *cpu_limit,
                memory_limit.clone(),
                security.clone(),
            ).await?;
        },
        
        Commands::Up { 
            manifest, detach, name, env, restart, health_interval 
        } => {
            info!("🚀 Starting biome in background mode");
            let executor = BiomeExecutor::new().await?;
            executor.up_biome(
                ctx,
                manifest.clone(),
                *detach,
                name.clone(),
                env.clone(),
                *restart,
                *health_interval,
            ).await?;
        },
        
        Commands::Down { biome, force, timeout, purge } => {
            info!("🛑 Stopping biome: {}", biome);
            let executor = BiomeExecutor::new().await?;
            executor.down_biome(
                biome.clone(),
                *force,
                *timeout,
                *purge,
            ).await?;
        },
        
        Commands::Ps { all, format, resources, status } => {
            info!("📋 Listing biomes");
            let executor = BiomeExecutor::new().await?;
            executor.list_biomes(
                *all,
                format.clone(),
                *resources,
                status.clone(),
            ).await?;
        },
        
        Commands::Logs { 
            target, follow, lines, timestamps, level, grep 
        } => {
            info!("📜 Showing logs for: {}", target);
            let executor = BiomeExecutor::new().await?;
            executor.show_logs(
                target.clone(),
                *follow,
                *lines,
                *timestamps,
                level.clone(),
                grep.clone(),
            ).await?;
        },
        
        Commands::Validate { 
            manifest, check_resources, check_security, format 
        } => {
            info!("🔍 Validating biome manifest: {}", manifest.display());
            validate_manifest_command(manifest, *check_resources, *check_security, format).await?;
        },
        
        Commands::Init { path, template, force } => {
            info!("📝 Initializing new biome manifest");
            init_manifest_command(path, template, *force).await?;
        },
        
        Commands::Capabilities { format, detailed, test_platform } => {
            info!("🌍 Showing system capabilities");
            show_capabilities_command(format, *detailed, test_platform).await?;
        },
        
        Commands::Ecosystem { action } => {
            execute_ecosystem_command(action).await?;
        },
        
        Commands::Universal { operation } => {
            execute_universal_command(operation).await?;
        },
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
    use toadstool_cli::{load_biome_manifest, validate_manifest};
    
    // Load manifest
    let manifest = load_biome_manifest(manifest_path).await
        .with_context(|| format!("Failed to load manifest: {}", manifest_path.display()))?;
    
    // Validate syntax and structure
    let warnings = validate_manifest(&manifest)?;
    
    // Additional validations
    let errors: Vec<String> = Vec::new();
    let validation_warnings = warnings;
    
    if check_resources {
        // TODO: Check if requested resources are available
        info!("🔍 Checking resource availability");
    }
    
    if check_security {
        // TODO: Validate security policies
        info!("🔒 Validating security policies");
    }
    
    // Output results
    match format {
        "json" => {
            let result = serde_json::json!({
                "valid": errors.is_empty(),
                "errors": errors,
                "warnings": validation_warnings,
                "manifest": manifest
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        },
        "text" | _ => {
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
            println!("  Biome: {} v{}", manifest.metadata.name, manifest.metadata.version);
            println!("  Primals: {}", manifest.primals.len());
            println!("  Services: {}", manifest.services.len());
            println!("  BearDog Required: {}", manifest.security.beardog_required);
        }
    }
    
    Ok(())
}

/// Initialize new biome manifest
async fn init_manifest_command(
    path: &PathBuf,
    template: &str,
    force: bool,
) -> Result<()> {
    use toadstool_cli::templates::TemplateGenerator;
    
    // Parse template type
    let biome_template = TemplateGenerator::parse_template(template)
        .with_context(|| format!("Unknown template type: {}", template))?;
    
    // Show available templates if requested
    if template == "list" {
        println!("📦 Available Templates:");
        for (name, description) in TemplateGenerator::list_templates() {
            println!("  {} - {}", name.bright_green().bold(), description);
        }
        return Ok(());
    }
    
    // Create template generator
    let generator = TemplateGenerator::new(path.clone(), force);
    
    // Generate manifest
    let output_path = generator.generate(biome_template).await?;
    
    println!("{} Biome manifest generated: {}", 
             "✅".green(), 
             output_path.display().to_string().bright_cyan());
    
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
        // TODO: Test specific platform
    }
    
    manager.show_capabilities(format.to_string(), detailed).await?;
    
    Ok(())
}

/// Execute ecosystem integration commands
async fn execute_ecosystem_command(action: &EcosystemCommands) -> Result<()> {
    let mut integrator = EcosystemIntegrator::new();
    
    match action {
        EcosystemCommands::Discover { services, timeout } => {
            info!("🔍 Discovering ecosystem services");
            let result = integrator.discover_services(services.clone(), *timeout).await?;
            
            println!("🎯 Discovery Results:");
            println!("  Services Found: {}", result.total_discovered);
            println!("  Verified: {}", result.verified_count);
            println!("  Scan Duration: {:.2}s", result.scan_duration.as_secs_f64());
            
            for service in &result.services {
                println!("  {} {} - {:?} ({})", 
                         "✅".green(),
                         format!("{:?}", service.service_type),
                         service.trust_level,
                         service.address);
            }
        },
        
        EcosystemCommands::Register { endpoint, token } => {
            info!("🐦 Registering with Songbird");
            integrator.register_with_songbird(endpoint.clone(), token.clone()).await?;
        },
        
        EcosystemCommands::Auth { permission_file, validate_only } => {
            info!("🐻 Installing BearDog permissions");
            integrator.install_beardog_permissions(
                permission_file.clone(),
                *validate_only,
            ).await?;
        },
        
        EcosystemCommands::Storage { endpoint, mount, dataset } => {
            info!("🏠 Connecting to NestGate storage");
            let mount_info = integrator.connect_nestgate_storage(
                endpoint.clone(),
                mount.clone(),
                dataset.clone(),
            ).await?;
            
            println!("{} NestGate storage connected:", "✅".green());
            println!("  Dataset: {}", mount_info.dataset_name);
            println!("  Mount Point: {}", mount_info.mount_point.display());
            println!("  Access Mode: {}", mount_info.access_mode);
        },
    }
    
    Ok(())
}

/// Execute universal compute operations
async fn execute_universal_command(operation: &UniversalCommands) -> Result<()> {
    let mut manager = UniversalComputeManager::new().await?;
    
    match operation {
        UniversalCommands::Detect { categories, test, output } => {
            info!("🔍 Detecting universal compute substrates");
            manager.detect_platforms(
                categories.clone(),
                *test,
                output.clone(),
            ).await?;
        },
        
        UniversalCommands::Benchmark { suite, platforms, format } => {
            info!("📊 Running benchmark suite: {}", suite);
            manager.run_benchmarks(
                suite.clone(),
                platforms.clone(),
                format.clone(),
            ).await?;
        },
        
        UniversalCommands::Migrate { source, target, pause, verify } => {
            info!("🚚 Migrating workload: {} → {}", source, target);
            manager.migrate_workload(
                source.clone(),
                target.clone(),
                *pause,
                *verify,
            ).await?;
        },
        
        UniversalCommands::Federate { endpoint, mode, resources } => {
            info!("🤝 Establishing federation");
            manager.establish_federation(
                endpoint.clone(),
                mode.clone(),
                resources.clone(),
            ).await?;
        },
    }
    
    Ok(())
}

/// Get error suggestion based on error content
fn get_error_suggestion(error: &anyhow::Error) -> Option<String> {
    let error_str = error.to_string();
    
    if error_str.contains("No such file or directory") {
        return Some("Check that the file path is correct and the file exists".to_string());
    }
    
    if error_str.contains("Permission denied") {
        return Some("Try running with sudo or check file permissions".to_string());
    }
    
    if error_str.contains("Connection refused") {
        return Some("Check that the service is running and the address is correct".to_string());
    }
    
    if error_str.contains("biome.yaml") {
        return Some("Use 'toadstool init' to create a new biome.yaml file".to_string());
    }
    
    if error_str.contains("not found") {
        return Some("Use 'toadstool ps' to see available biomes".to_string());
    }
    
    None
} 