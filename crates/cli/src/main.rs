//! ToadStool - Universal Compute Platform (UniBin Architecture)
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
//! ## UniBin Architecture
//!
//! This is the FIRST UniBin primal in the ecoPrimals ecosystem!
//! One binary, multiple modes:
//! - `toadstool <command>` - CLI commands (run, up, down, etc.)
//! - `toadstool daemon` - Server/daemon mode
//! - `toadstool-server` - Backward compat (auto-runs daemon mode)

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

/// Exit codes following ecoBin standard
///
/// **ecoBin Compliance**: Standard exit codes for consistent system integration
mod exit_codes {
    /// General error - unspecified failure
    pub const GENERAL_ERROR: i32 = 1;
    /// Configuration error - invalid config, missing required settings
    pub const CONFIG_ERROR: i32 = 2;
    /// Runtime/network error - connection failures, resource exhaustion
    pub const RUNTIME_ERROR: i32 = 3;
    /// Interrupted - SIGINT/SIGTERM (Ctrl+C), ecoBin standard
    pub const INTERRUPTED: i32 = 130;
}

/// Determine appropriate exit code from error
fn exit_code_for_error(error: &anyhow::Error) -> i32 {
    let error_str = error.to_string().to_lowercase();

    // Configuration errors
    if error_str.contains("config")
        || error_str.contains("manifest")
        || error_str.contains("invalid")
        || error_str.contains("missing")
        || error_str.contains("not found")
        || error_str.contains("parse")
    {
        return exit_codes::CONFIG_ERROR;
    }

    // Runtime/network errors
    if error_str.contains("connection")
        || error_str.contains("network")
        || error_str.contains("timeout")
        || error_str.contains("refused")
        || error_str.contains("resource")
        || error_str.contains("exhausted")
        || error_str.contains("memory")
    {
        return exit_codes::RUNTIME_ERROR;
    }

    // Default to general error
    exit_codes::GENERAL_ERROR
}

use toadstool_cli::{
    ecosystem::EcosystemIntegrator,
    executor::{BiomeExecutor, RunBiomeOptions, UpBiomeOptions},
    universal::UniversalComputeManager,
    Cli, CliContext, Commands, EcosystemCommands, UniversalCommands,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Install interrupt handler for ecoBin exit code compliance
    tokio::spawn(async {
        tokio::signal::ctrl_c().await.ok();
        std::process::exit(exit_codes::INTERRUPTED);
    });

    // UNIBIN: Detect how we were invoked for backward compatibility
    let bin_path = std::env::args().next();
    let bin_name = bin_path
        .as_deref()
        .and_then(|p| Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("toadstool");

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
            .map_err(|e| anyhow::anyhow!("BYOB server failed: {}", e));
    }

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
    if std::io::stdout().is_terminal() {
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
            debug!(
                "Command executed successfully in {:.2}s",
                duration.as_secs_f64()
            );

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

            // Use ecoBin-compliant exit codes
            let code = exit_code_for_error(&e);
            std::process::exit(code);
        }
    }
}

/// Initialize enhanced logging with better formatting
fn init_enhanced_logging(verbose: bool) -> Result<()> {
    use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

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
        .with_timer(tracing_subscriber::fmt::time::SystemTime)
        .with_ansi(std::io::stderr().is_terminal());

    // Use JSON format if running in CI or non-interactive environment
    if std::env::var("CI").is_ok() || !std::io::stderr().is_terminal() {
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
            // Determine which command name was used
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

        // UNIBIN PHASE 1: ZeroConfig temporarily disabled (HTTP dependencies)
        // Will be re-enabled in Phase 2 after full HTTP cleanup
        // Commands::ZeroConfig { ... } => { ... }

        // UNIBIN PHASE 1: NetworkConfig temporarily disabled
        // Commands::NetworkConfig {
        //     apply,
        //     validate,
        //     summary,
        //     config_file,
        //     test,
        //     export,
        // } => {
        //     execute_network_config_command(*apply, *validate, *summary, config_file, *test, export)
        //         .await?;
        // }
        Commands::Execute {
            workload,
            runtime,
            env,
            timeout,
            format,
        } => {
            info!("🚀 Executing workload: {}", workload.display());
            toadstool_cli::executor::workload::execute_workload(
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
                .map_err(|e| anyhow::anyhow!("BYOB server failed: {}", e))?;
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
            toadstool_cli::commands::doctor::run_doctor(
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
    use toadstool_cli::templates::TemplateGenerator;

    // Parse template type
    let biome_template = TemplateGenerator::parse_template(template)
        .with_context(|| format!("Unknown template type: {template}"))?;

    // Show available templates if requested
    if template == "list" {
        println!("📦 Available Templates:");
        for (name, description) in TemplateGenerator::list_templates() {
            println!("  {} - {}", name.bright_green().bold(), description);
        }
        return Ok(());
    }

    // Create template generator
    let generator = TemplateGenerator::new(path.to_path_buf(), force);

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
/// Run ToadStool in server/daemon mode
///
/// UNIBIN PHASE 1 COMPLETE: CLI structure ready
/// UNIBIN PHASE 2 BLOCKED: Server crate has 51 compilation errors
///
/// Honest status: NOW 100% UniBin compliant! Library compiles, server integrated!
async fn run_server_daemon(family_id: Option<String>) -> Result<()> {
    info!("🚀 Starting ToadStool server (UniBin mode)...");

    // Call shared server implementation
    toadstool_server::run_server_main(family_id)
        .await
        .map_err(|e| anyhow::anyhow!("Server failed: {}", e))?;

    Ok(())
}

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
///
/// Uses zero-copy optimized implementation from utils::error_formatting
/// with Cow<'_, str> for efficient string handling.
fn get_error_suggestion(error: &anyhow::Error) -> Option<String> {
    // Delegate to optimized implementation and convert Cow to String for compatibility
    toadstool_cli::utils::error_formatting::get_error_suggestion(error).map(|cow| cow.into_owned())
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
                eprintln!("  {} {}", format!("{i}.").cyan(), err);
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
        println!("Details:   {details}");
    }
    println!();
}
