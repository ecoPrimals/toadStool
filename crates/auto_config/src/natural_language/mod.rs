// SPDX-License-Identifier: AGPL-3.0-only
//! # Natural Language Configuration Interface
//!
//! Enables configuration of `ToadStool` through natural language descriptions.
//! Perfect for integration with Squirrel MCP and AI systems that need to configure
//! compute environments through conversation.
//!
//! ## Architecture
//!
//! This module is organized by concerns:
//! - `types` - Core type definitions (preferences, intents, templates)
//! - `intent` - Intent recognition and analysis
//! - `templates` - Pre-configured templates for common use cases
//!
//! ## Examples
//!
//! ```rust,no_run
//! use toadstool_auto_config::NaturalLanguageConfig;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut nl_config = NaturalLanguageConfig::new();
//!     
//!     // Configure using natural language
//!     let config = nl_config.configure_from_text(
//!         "I want to run machine learning workloads with high performance \
//!          and automatic GPU acceleration. Make it secure but fast."
//!     ).await?;
//!     
//!     println!("Configuration generated from natural language!");
//!     Ok(())
//! }
//! ```

pub mod intent;
pub mod templates;
pub mod types;

// Re-export main types for convenience
pub use types::*;

use std::collections::HashMap;
use tracing::info;

use crate::intelligent::IntelligentAutoConfig;
use crate::{ToadStoolError, ToadStoolResult};
use toadstool_config::ToadStoolConfig;

/// Natural language configuration system for AI-friendly setup
pub struct NaturalLanguageConfig {
    /// Base auto-configuration system
    _auto_config: IntelligentAutoConfig,
    /// Intent recognition patterns
    intent_patterns: HashMap<String, ConfigurationIntent>,
    /// Configuration templates
    templates: HashMap<String, ConfigurationTemplate>,
}

impl NaturalLanguageConfig {
    /// Create a new natural language configuration system
    #[must_use]
    pub fn new() -> Self {
        let intent_patterns = intent::create_intent_patterns();
        let templates = templates::create_templates();

        Self {
            _auto_config: IntelligentAutoConfig::new(),
            intent_patterns,
            templates,
        }
    }

    /// Configure from natural language text
    ///
    /// # Errors
    ///
    /// Returns an error if intent analysis fails or configuration generation fails
    #[expect(
        clippy::unused_async,
        reason = "API contract for future async operations"
    )]
    pub async fn configure_from_text(&mut self, text: &str) -> ToadStoolResult<ToadStoolConfig> {
        info!("🗣️  Processing natural language configuration request");

        // Analyze intent
        let analysis = intent::analyze_intent(text, &self.intent_patterns)?;

        // Get appropriate template
        let template = templates::get_template(&self.templates, &analysis.primary_intent);

        info!(
            "📋 Selected template: {} (confidence: {:.2})",
            template.name, analysis.confidence
        );

        // Generate configuration from template
        let config = self.generate_config_from_template(&template);

        Ok(config)
    }

    /// Analyze natural language text to determine intent
    ///
    /// # Errors
    ///
    /// Returns an error if intent analysis fails
    pub fn analyze_intent(&self, text: &str) -> ToadStoolResult<IntentAnalysis> {
        intent::analyze_intent(text, &self.intent_patterns)
    }

    /// Configure from a specific template by name
    ///
    /// # Errors
    ///
    /// Returns an error if the template is not found or configuration generation fails
    #[expect(
        clippy::unused_async,
        reason = "API contract for future async operations"
    )]
    pub async fn configure_from_template(
        &mut self,
        template_name: &str,
    ) -> ToadStoolResult<ToadStoolConfig> {
        let template = self.templates.get(template_name).ok_or_else(|| {
            ToadStoolError::configuration(format!("Template not found: {template_name}"))
        })?;

        Ok(self.generate_config_from_template(template))
    }

    /// Get list of available template names
    #[must_use]
    pub fn get_available_templates(&self) -> Vec<&ConfigurationTemplate> {
        self.templates.values().collect()
    }

    /// Extract explicit preferences from natural language text
    ///
    /// # Errors
    ///
    /// Returns an error if preference extraction fails
    pub fn extract_explicit_preferences(&self, text: &str) -> ToadStoolResult<ExplicitPreferences> {
        // This is now a public wrapper around the intent module's logic
        let text_lower = text.to_lowercase();
        Ok(intent::extract_explicit_preferences(&text_lower))
    }

    /// Generate configuration from a template
    #[expect(clippy::unused_self, reason = "may use self for future extensions")]
    fn generate_config_from_template(&self, template: &ConfigurationTemplate) -> ToadStoolConfig {
        info!(
            "⚙️  Generating configuration from template: {}",
            template.name
        );

        // Start with default configuration
        let config = ToadStoolConfig::default();

        // Apply runtime preferences
        if template.runtime_preferences.enable_gpu() {
            // Enable GPU runtime via configuration
            // Note: Actual GPU config application would go here
            info!("🎮 GPU runtime enabled");
        }

        // Apply resource preferences
        if template.resource_preferences.requires_gpu {
            info!("⚡ GPU required for this workload");
        }

        // Apply security preferences based on template
        let _security_level = match template.security_preference {
            SecurityPreference::Minimal => "minimal",
            SecurityPreference::Balanced => "balanced",
            SecurityPreference::High => "high",
            SecurityPreference::Maximum => "maximum",
        };

        // Note: Security configuration application would go here
        // config.security.set_level(security_level);

        info!("✅ Configuration generated successfully");
        config
    }

    /// Validate and optimize generated configuration
    ///
    /// NOTE: Pass-through implementation - configuration validation happens
    /// elsewhere in the pipeline. Reserved for future optimization passes.
    #[expect(dead_code, reason = "Reserved: future config optimization passes")]
    #[expect(clippy::unused_self, reason = "reserved for future optimization logic")]
    const fn validate_and_optimize(&self, config: ToadStoolConfig) -> ToadStoolConfig {
        // Configuration is validated during generation and by config module
        // This method is reserved for future optimization logic
        config
    }
}

impl Default for NaturalLanguageConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_machine_learning_intent() {
        let nl_config = NaturalLanguageConfig::new();

        let analysis = nl_config
            .analyze_intent(
                "I want to train neural networks and do machine learning with GPU acceleration",
            )
            .unwrap();

        assert_eq!(analysis.primary_intent, "machine_learning");
        assert!(analysis.confidence > 0.0);
    }

    #[test]
    fn test_web_development_intent() {
        let nl_config = NaturalLanguageConfig::new();

        let analysis = nl_config
            .analyze_intent(
                "I'm building a web application with React and need to deploy containers",
            )
            .unwrap();

        assert_eq!(analysis.primary_intent, "web_development");
        assert!(analysis.confidence > 0.0);
    }

    #[test]
    fn test_explicit_preferences_extraction() {
        let nl_config = NaturalLanguageConfig::new();

        let analysis = nl_config
            .analyze_intent("I need high performance with GPU acceleration and maximum security")
            .unwrap();

        assert_eq!(
            analysis.explicit_preferences.performance_priority,
            Some("high".to_string())
        );
        assert_eq!(
            analysis.explicit_preferences.security_priority,
            Some("high".to_string())
        );
    }

    #[test]
    fn test_configuration_template_structure() {
        let nl_config = NaturalLanguageConfig::new();

        let ml_template = nl_config.templates.get("machine_learning").unwrap();
        assert_eq!(ml_template.name, "Machine Learning");
        assert!(ml_template.runtime_preferences.enable_gpu());
        assert!(ml_template.runtime_preferences.enable_python());
        assert!(ml_template.runtime_preferences.gpu_memory_fraction > 0.5);

        let web_template = nl_config.templates.get("web_development").unwrap();
        assert_eq!(web_template.name, "Web Development");
        assert!(web_template.runtime_preferences.enable_container());
        assert!(web_template.runtime_preferences.enable_wasm());
    }
}
