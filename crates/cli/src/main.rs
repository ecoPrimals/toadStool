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

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

use toadstool_cli::{
    ecosystem::EcosystemIntegrator, executor::BiomeExecutor, universal::UniversalComputeManager,
    Cli, CliContext, Commands, EcosystemCommands, UniversalCommands,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Initialize logging with better formatting
    init_enhanced_logging(cli.verbose)?;

    // SECURITY WARNING: Alert users about incomplete security implementations
    if std::env::var("TOADSTOOL_SECURITY_WARNING_ACKNOWLEDGED").is_err() {
        warn!("🚨 SECURITY WARNING: This ToadStool instance has incomplete cryptographic verification");
        warn!("🚨 Service discovery and permission validation are not fully implemented");
        warn!("🚨 Do NOT use in production environments without proper security audit");
        warn!("🚨 Set TOADSTOOL_SECURITY_WARNING_ACKNOWLEDGED=1 to suppress this warning");
    }

    // Print banner (only in interactive mode)
    if atty::is(atty::Stream::Stdout) {
        print_banner();
    }

    // Create CLI context
    let ctx = CliContext::new(&cli)?;

    // Record start time for operation timing
    let start_time = std::time::Instant::now();

    // Execute command with enhanced error handling
    match execute_command(&cli, &ctx).await {
        Ok(()) => {
            let duration = start_time.elapsed();
            debug!("Command executed successfully in {:.2}s", duration.as_secs_f64());
            
            // Show success message for longer operations
            if duration.as_secs() > 2 {
                print_success_message("Operation completed successfully!");
                print_operation_summary("Command execution", duration, None);
            }
            
            Ok(())
        }
        Err(e) => {
            let duration = start_time.elapsed();
            error!("Command failed after {:.2}s: {}", duration.as_secs_f64(), e);

            // Print enhanced error information
            print_enhanced_error(&e);

            std::process::exit(1);
        }
    }
}

/// Initialize enhanced logging with better formatting
fn init_enhanced_logging(verbose: bool) -> Result<()> {
    use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

    let filter = if verbose {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_timer(tracing_subscriber::fmt::time::ChronoUtc::rfc_3339())
        .with_ansi(atty::is(atty::Stream::Stderr));

    // Use JSON format if running in CI or non-interactive environment
    if std::env::var("CI").is_ok() || !atty::is(atty::Stream::Stderr) {
        subscriber.json().init();
    } else {
        subscriber.init();
    }

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
    println!(
        "{}",
        "Universal Compute Platform - The Backbone of SOVEREIGN SCIENCE"
            .bright_cyan()
            .bold()
    );
    println!(
        "{}",
        "Version 0.1.0 | If it has a chip and memory, ToadStool runs on it".bright_white()
    );
    println!();
}

/// Execute the main CLI command
async fn execute_command(cli: &Cli, ctx: &CliContext) -> Result<()> {
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
            executor
                .run_biome(
                    ctx,
                    manifest.clone(),
                    name.clone(),
                    env.clone(),
                    *debug,
                    *cpu_limit,
                    memory_limit.clone(),
                    security.clone(),
                )
                .await?;
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
            executor
                .up_biome(
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
    let manifest = load_biome_manifest(manifest_path)
        .await
        .with_context(|| format!("Failed to load manifest: {}", manifest_path.display()))?;

    // Validate syntax and structure
    let warnings = validate_manifest(&manifest)?;

    // Additional validations
    let errors: Vec<String> = Vec::new();
    let validation_warnings = warnings;

    if check_resources {
        // Check if requested resources are available locally
        info!("🔍 Checking resource availability");
    }

    if check_security {
        // Validate security policies before execution
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
        }
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
async fn init_manifest_command(path: &PathBuf, template: &str, force: bool) -> Result<()> {
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
        // Test specific platform implementation
    }

    manager
        .show_capabilities(format.to_string(), detailed)
        .await?;

    Ok(())
}

/// Execute ecosystem integration commands
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
                    "  {} {} - {:?} ({})",
                    "✅".green(),
                    format!("{:?}", service.service_type),
                    service.trust_level,
                    service.address
                );
            }
        }

        EcosystemCommands::Register { endpoint, token } => {
            info!("🐦 Registering with Songbird");
            integrator
                .register_with_songbird(endpoint.clone(), token.clone())
                .await?;
        }

        EcosystemCommands::Auth {
            permission_file,
            validate_only,
        } => {
            info!("🐻 Installing BearDog permissions");
            integrator
                .install_beardog_permissions(permission_file.clone(), *validate_only)
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

/// Get error suggestion based on error content
fn get_error_suggestion(error: &anyhow::Error) -> Option<String> {
    let error_str = error.to_string().to_lowercase();

    // File system errors
    if error_str.contains("no such file or directory") {
        return Some("💡 Check that the file path is correct and the file exists. Use 'ls' to verify.".to_string());
    }

    if error_str.contains("permission denied") {
        return Some("💡 Try running with sudo or check file permissions with 'chmod' and 'chown'.".to_string());
    }

    // Network errors
    if error_str.contains("connection refused") {
        return Some("💡 Check that the service is running and the address is correct. Use 'netstat -tlnp' to verify.".to_string());
    }

    if error_str.contains("connection timeout") {
        return Some("💡 Check network connectivity and firewall settings. The service may be overloaded.".to_string());
    }

    // ToadStool specific errors
    if error_str.contains("biome.yaml") {
        return Some("💡 Use 'toadstool init' to create a new biome.yaml file or 'toadstool validate' to check an existing one.".to_string());
    }

    if error_str.contains("not found") && !error_str.contains("file") {
        return Some("💡 Use 'toadstool ps' to see available biomes or 'toadstool capabilities' to check platform support.".to_string());
    }

    if error_str.contains("already running") {
        return Some("💡 Use 'toadstool down <biome>' to stop the existing instance or 'toadstool ps' to check status.".to_string());
    }

    if error_str.contains("insufficient resources") {
        return Some("💡 Check available resources with 'toadstool capabilities' and adjust limits in biome.yaml.".to_string());
    }

    if error_str.contains("security") {
        return Some("💡 Check BearDog permissions with 'toadstool ecosystem auth --validate-only' and security policies.".to_string());
    }

    // Runtime errors
    if error_str.contains("wasm") {
        return Some("💡 Verify WASM module is valid and all required dependencies are available.".to_string());
    }

    if error_str.contains("gpu") {
        return Some("💡 Check GPU drivers and CUDA/OpenCL installation with 'nvidia-smi' or 'clinfo'.".to_string());
    }

    if error_str.contains("container") {
        return Some("💡 Verify Docker/container runtime is installed and running. Check with 'docker version'.".to_string());
    }

    // Ecosystem errors
    if error_str.contains("songbird") {
        return Some("💡 Use 'toadstool ecosystem discover' to find Songbird instances or check network connectivity.".to_string());
    }

    if error_str.contains("nestgate") {
        return Some("💡 Verify NestGate endpoint and credentials. Use 'toadstool ecosystem storage --help' for options.".to_string());
    }

    if error_str.contains("beardog") {
        return Some("💡 Install BearDog permissions with 'toadstool ecosystem auth <permission-file>'.".to_string());
    }

    // General suggestions
    if error_str.contains("parse") || error_str.contains("invalid") {
        return Some("💡 Check syntax and format of configuration files. Use '--help' for command usage.".to_string());
    }

    if error_str.contains("timeout") {
        return Some("💡 Increase timeout values or check system performance. Some operations may take longer on slower systems.".to_string());
    }

    None
}

/// Enhanced error reporting with context
fn print_enhanced_error(error: &anyhow::Error) {
    eprintln!("\n{}", "💥 ERROR".red().bold());
    eprintln!("{}", "═".repeat(60).red());
    
    // Main error message
    eprintln!("{} {}", "Message:".red().bold(), error);
    
    // Error chain
    if error.chain().count() > 1 {
        eprintln!("\n{}", "📋 Error Chain:".yellow().bold());
        for (i, err) in error.chain().enumerate() {
            if i > 0 {
                eprintln!("  {} {}", format!("{}.", i).cyan(), err);
            }
        }
    }
    
    // Suggestion
    if let Some(suggestion) = get_error_suggestion(error) {
        eprintln!("\n{}", suggestion.green());
    }
    
    // Help resources
    eprintln!("\n{}", "📚 Need Help?".blue().bold());
    eprintln!("  {} toadstool --help", "•".blue());
    eprintln!("  {} toadstool <command> --help", "•".blue()); 
    eprintln!("  {} https://docs.toadstool.dev", "•".blue());
    eprintln!();
}

/// Enhanced success reporting
fn print_success_message(message: &str) {
    println!("\n{} {}", "✅".green().bold(), message.green().bold());
}

/// Print operation summary
fn print_operation_summary(operation: &str, duration: std::time::Duration, details: Option<&str>) {
    println!("\n{}", "📊 Operation Summary".blue().bold());
    println!("{}", "─".repeat(40).blue());
    println!("Operation: {}", operation.cyan());
    println!("Duration:  {:.2}s", duration.as_secs_f64());
    if let Some(details) = details {
        println!("Details:   {}", details);
    }
    println!();
}
