// SPDX-License-Identifier: AGPL-3.0-or-later
//! Root CLI struct, execution context, and manifest loading helpers.

use clap::Parser;
use std::path::PathBuf;
use tokio::fs;

use crate::biome_model::BiomeManifest;
use crate::error::{CliContextExt, Result};

/// `ToadStool` - Universal Compute Platform for Sovereign Science
#[derive(Parser)]
#[command(name = "toadstool")]
#[command(about = "🍄 Universal Compute Platform - The backbone of SOVEREIGN SCIENCE")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = "ToadStool Development Team")]
#[command(long_about = "
ToadStool is the universal runtime environment for the ecoPrimals ecosystem.
It bootstraps, manages, and isolates complete biomeOS instances from declarative
manifest files (biome.yaml).

🎯 SOVEREIGN SCIENCE: Your compute, your data, your control
🚀 UNIVERSAL COMPUTE: If it has a chip and memory, ToadStool runs on it
🔒 ZERO TRUST: Cryptographic security by default
")]
pub struct Cli {
    /// Top-level subcommand (ecosystem, universal, daemon, etc.)
    #[command(subcommand)]
    pub command: crate::commands::Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Working directory
    #[arg(short = 'C', long, global = true)]
    pub directory: Option<PathBuf>,
}

/// CLI execution context
pub struct CliContext {
    /// Path to config file (if -c/--config specified)
    pub config_path: Option<PathBuf>,
    /// Working directory (-C/--directory or cwd)
    pub working_dir: PathBuf,
    /// Whether verbose output is enabled (-v/--verbose)
    pub verbose: bool,
}

impl CliContext {
    /// Create context from parsed CLI args (config path, working dir, verbose)
    pub fn new(cli: &Cli) -> Result<Self> {
        let working_dir = if let Some(dir) = &cli.directory {
            dir.clone()
        } else {
            std::env::current_dir()?
        };

        Ok(Self {
            config_path: cli.config.clone(),
            working_dir,
            verbose: cli.verbose,
        })
    }
}

/// Load biome manifest from file
///
/// Supports both TOML (preferred, ecoBin compliant) and YAML formats.
/// Format is detected by file extension:
/// - `.toml` → TOML parser (pure Rust, no C dependencies)
/// - `.yaml`, `.yml` → YAML parser (legacy support)
/// - Other → Try TOML first, fall back to YAML
pub async fn load_biome_manifest(path: &PathBuf) -> Result<BiomeManifest> {
    let content = fs::read_to_string(path)
        .await
        .context(format!("Failed to read manifest file: {}", path.display()))?;

    // Determine format from extension (TOML preferred for ecoBin compliance)
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let manifest: BiomeManifest = match extension.to_lowercase().as_str() {
        "toml" => toml::from_str(&content)
            .context(format!("Failed to parse TOML manifest: {}", path.display()))?,
        "yaml" | "yml" => serde_yaml_ng::from_str(&content)
            .context(format!("Failed to parse YAML manifest: {}", path.display()))?,
        _ => {
            // Unknown extension: try TOML first (ecoBin preferred), then YAML
            toml::from_str(&content).or_else(|_| {
                serde_yaml_ng::from_str(&content)
                    .context(format!("Failed to parse manifest: {}", path.display()))
            })?
        }
    };

    Ok(manifest)
}

/// Validate biome manifest
pub fn validate_manifest(manifest: &BiomeManifest) -> Result<Vec<String>> {
    let mut warnings = Vec::new();

    // Check for required crypto/security capability provider
    if !manifest.has_primal_with_capability("crypto") && manifest.security.security_required {
        warnings.push("A security service is required but not configured".to_string());
    }

    // Validate service dependencies
    for (service_name, service) in &manifest.services {
        for dep in &service.dependencies {
            if !manifest.services.contains_key(dep) && !manifest.primals.contains_key(dep) {
                warnings.push(format!(
                    "Service '{service_name}' depends on undefined service '{dep}'"
                ));
            }
        }
    }

    // Check resource limits
    if manifest.resources.cpu_limit.is_none() {
        warnings.push("No CPU limit specified - consider setting resource limits".to_string());
    }

    Ok(warnings)
}
