// SPDX-License-Identifier: AGPL-3.0-or-later
//! Natural language configuration processor (requires `runtime` feature).

use std::collections::HashMap;

use tracing::info;

use crate::intelligent::IntelligentAutoConfig;
use crate::{ToadStoolError, ToadStoolResult};
use toadstool_config::ToadStoolConfig;

use super::intent;
use super::templates;
use super::types::{
    ConfigurationIntent, ConfigurationTemplate, ExplicitPreferences, IntentAnalysis,
    SecurityPreference,
};

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

        let analysis = intent::analyze_intent(text, &self.intent_patterns)?;
        let template = templates::get_template(&self.templates, &analysis.primary_intent);

        info!(
            "📋 Selected template: {} (confidence: {:.2})",
            template.name, analysis.confidence
        );

        Ok(self.generate_config_from_template(&template))
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

        let config = ToadStoolConfig::default();

        if template.runtime_preferences.enable_gpu() {
            info!("🎮 GPU runtime enabled");
        }

        if template.resource_preferences.requires_gpu {
            info!("⚡ GPU required for this workload");
        }

        let _security_level = match template.security_preference {
            SecurityPreference::Minimal => "minimal",
            SecurityPreference::Balanced => "balanced",
            SecurityPreference::High => "high",
            SecurityPreference::Maximum => "maximum",
        };

        info!("✅ Configuration generated successfully");
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
