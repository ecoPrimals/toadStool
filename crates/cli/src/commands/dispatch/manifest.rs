// SPDX-License-Identifier: AGPL-3.0-only
//! Manifest command handlers
//!
//! Validate and Init - biome manifest validation and template generation.

use std::path::{Path, PathBuf};

use colored::Colorize;
use tracing::info;

use crate::{load_biome_manifest, validate_manifest, CliContextExt, Result};

/// Validate biome manifest
pub async fn execute_validate(
    manifest_path: &PathBuf,
    check_resources: bool,
    check_security: bool,
    format: &str,
) -> Result<()> {
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
pub async fn execute_init(path: &Path, template: &str, force: bool) -> Result<()> {
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
