// SPDX-License-Identifier: AGPL-3.0-or-later
//! TemplateGenerator implementation

use super::{BiomeTemplate, TemplateGenerator};
use crate::{BiomeManifest, BiomeMetadata};
use crate::{CliContextExt, Result};
use std::path::PathBuf;
use tokio::fs;
use tracing::info;

impl TemplateGenerator {
    /// Create a template generator for the given output directory
    #[must_use]
    pub const fn new(output_dir: PathBuf, force_overwrite: bool) -> Self {
        Self {
            output_dir,
            force_overwrite,
        }
    }

    /// Generate biome manifest from template
    pub async fn generate(&self, template: BiomeTemplate) -> Result<PathBuf> {
        let manifest = self.create_manifest(&template)?;
        let output_path = self.output_dir.join("biome.yaml");

        // Check if file exists and handle overwrite
        if output_path.exists() && !self.force_overwrite {
            return Err(crate::CliError::Other(
                "biome.yaml already exists. Use --force to overwrite.".to_string(),
            ));
        }

        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Generate YAML content
        let yaml_content = super::rendering::manifest_to_yaml(&manifest)?;

        // Write to file
        fs::write(&output_path, yaml_content)
            .await
            .context(format!(
                "Failed to write biome.yaml to {}",
                output_path.display()
            ))?;

        info!("✅ Generated biome.yaml: {}", output_path.display());
        super::rendering::print_template_info(&template);

        Ok(output_path)
    }

    /// List available templates
    #[must_use]
    pub fn list_templates() -> Vec<(String, String)> {
        vec![
            (
                "basic".to_string(),
                "Essential services for general computing".to_string(),
            ),
            (
                "science".to_string(),
                "Scientific computing with data analysis tools".to_string(),
            ),
            (
                "ai-research".to_string(),
                "AI/ML training and inference environment".to_string(),
            ),
            (
                "quantum".to_string(),
                "Quantum computing research platform".to_string(),
            ),
            (
                "genomics".to_string(),
                "Bioinformatics and genomics analysis".to_string(),
            ),
            (
                "vision".to_string(),
                "Computer vision and imaging processing".to_string(),
            ),
            (
                "distributed".to_string(),
                "Multi-node distributed computing cluster".to_string(),
            ),
            (
                "sovereign".to_string(),
                "Maximum security sovereign computing".to_string(),
            ),
            (
                "development".to_string(),
                "Development and testing environment".to_string(),
            ),
        ]
    }

    /// Parse template type from string
    pub fn parse_template(template_str: &str) -> Result<BiomeTemplate> {
        match template_str.to_lowercase().as_str() {
            "basic" => Ok(BiomeTemplate::Basic),
            "science" => Ok(BiomeTemplate::Science),
            "ai-research" | "ai" | "ml" => Ok(BiomeTemplate::AiResearch),
            "quantum" => Ok(BiomeTemplate::Quantum),
            "genomics" | "bio" | "bioinformatics" => Ok(BiomeTemplate::Genomics),
            "vision" | "cv" | "imaging" => Ok(BiomeTemplate::Vision),
            "distributed" | "cluster" => Ok(BiomeTemplate::Distributed),
            "sovereign" | "security" => Ok(BiomeTemplate::Sovereign),
            "development" | "dev" | "test" => Ok(BiomeTemplate::Development),
            _ => Err(crate::CliError::Other(format!(
                "Unknown template type: {template_str}"
            ))),
        }
    }

    fn create_manifest(&self, template: &BiomeTemplate) -> Result<BiomeManifest> {
        let now = std::time::SystemTime::now();

        let (name, description, primals, services, resources, security, networking, storage) =
            match template {
                BiomeTemplate::Basic => super::basic_templates::create_basic_template(),
                BiomeTemplate::Science => super::specialized_templates::create_science_template(),
                BiomeTemplate::AiResearch => {
                    super::specialized_templates::create_ai_research_template()
                }
                BiomeTemplate::Quantum => super::specialized_templates::create_quantum_template(),
                BiomeTemplate::Genomics => super::specialized_templates::create_genomics_template(),
                BiomeTemplate::Vision => super::specialized_templates::create_vision_template(),
                BiomeTemplate::Distributed => {
                    super::specialized_templates::create_distributed_template()
                }
                BiomeTemplate::Sovereign => {
                    super::specialized_templates::create_sovereign_template()
                }
                BiomeTemplate::Development => super::basic_templates::create_development_template(),
                BiomeTemplate::Custom(spec) => {
                    super::specialized_templates::create_custom_template(spec)
                }
            };

        Ok(BiomeManifest {
            metadata: BiomeMetadata {
                name,
                version: "1.0.0".to_string(),
                description: Some(description),
                author: Some("ToadStool Universal Compute".to_string()),
                created: now,
                updated: now,
                tags: super::rendering::get_template_tags(template),
            },
            primals,
            services,
            resources,
            security,
            networking,
            storage,
        })
    }
}
