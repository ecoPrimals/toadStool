//! # Natural Language Configuration Interface
//!
//! Enables configuration of ToadStool through natural language descriptions,
//! using natural language descriptions. Perfect for integration with Squirrel MCP
//! and AI systems that need to configure compute environments through conversation.
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

use std::collections::HashMap;
use std::time::Duration;

use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::{ToadStoolResult, ToadStoolError};
use crate::intelligent::IntelligentAutoConfig;
use toadstool_config::ToadStoolConfig;

/// Natural language configuration system for AI-friendly setup
pub struct NaturalLanguageConfig {
    /// Base auto-configuration system
    auto_config: IntelligentAutoConfig,
    /// Intent recognition patterns
    intent_patterns: HashMap<String, ConfigurationIntent>,
    /// Configuration templates
    templates: HashMap<String, ConfigurationTemplate>,
}

impl NaturalLanguageConfig {
    /// Create a new natural language configuration system
    pub fn new() -> Self {
        let mut intent_patterns = HashMap::new();
        let mut templates = HashMap::new();

        // Machine Learning Intent
        intent_patterns.insert("machine_learning".to_string(), ConfigurationIntent {
            keywords: vec![
                "machine learning".to_string(), "ml".to_string(), "ai".to_string(), "neural network".to_string(), "model training".to_string(),
                "tensorflow".to_string(), "pytorch".to_string(), "jupyter".to_string(), "data science".to_string(), "gpu acceleration".to_string()
            ],
            priority_features: vec!["gpu_runtime".to_string(), "high_memory".to_string(), "python_runtime".to_string()],
            performance_preference: PerformancePreference::MaximumPerformance,
            security_preference: SecurityPreference::Balanced,
        });

        // Web Development Intent
        intent_patterns.insert("web_development".to_string(), ConfigurationIntent {
            keywords: vec![
                "web development".to_string(), "web app".to_string(), "api".to_string(), "server".to_string(), "frontend".to_string(), "backend".to_string(),
                "nodejs".to_string(), "javascript".to_string(), "react".to_string(), "vue".to_string(), "express".to_string(), "fastify".to_string()
            ],
            priority_features: vec!["container_runtime".to_string(), "network_optimization".to_string(), "auto_scaling".to_string()],
            performance_preference: PerformancePreference::Balanced,
            security_preference: SecurityPreference::High,
        });

        // Data Processing Intent
        intent_patterns.insert("data_processing".to_string(), ConfigurationIntent {
            keywords: vec![
                "data processing".to_string(), "etl".to_string(), "data pipeline".to_string(), "batch processing".to_string(),
                "streaming".to_string(), "kafka".to_string(), "spark".to_string(), "pandas".to_string(), "large datasets".to_string()
            ],
            priority_features: vec!["high_memory".to_string(), "storage_optimization".to_string(), "parallel_processing".to_string()],
            performance_preference: PerformancePreference::HighPerformance,
            security_preference: SecurityPreference::Balanced,
        });

        // Gaming Intent
        intent_patterns.insert("gaming".to_string(), ConfigurationIntent {
            keywords: vec![
                "gaming".to_string(), "game".to_string(), "graphics".to_string(), "rendering".to_string(), "3d".to_string(), "opengl".to_string(), "vulkan".to_string(),
                "unity".to_string(), "unreal".to_string(), "fps".to_string(), "real-time".to_string(), "low latency".to_string()
            ],
            priority_features: vec!["gpu_runtime".to_string(), "low_latency".to_string(), "real_time_priority".to_string()],
            performance_preference: PerformancePreference::MaximumPerformance,
            security_preference: SecurityPreference::Balanced,
        });

        // Enterprise Intent
        intent_patterns.insert("enterprise".to_string(), ConfigurationIntent {
            keywords: vec![
                "enterprise".to_string(), "security".to_string(), "compliance".to_string(), "audit".to_string(), "encryption".to_string(),
                "production".to_string(), "business".to_string(), "corporate".to_string(), "secure".to_string(), "regulated".to_string()
            ],
            priority_features: vec!["maximum_security".to_string(), "audit_logging".to_string(), "sandboxing".to_string()],
            performance_preference: PerformancePreference::Balanced,
            security_preference: SecurityPreference::Maximum,
        });

        // Development Intent
        intent_patterns.insert("development".to_string(), ConfigurationIntent {
            keywords: vec![
                "development".to_string(), "dev".to_string(), "testing".to_string(), "debugging".to_string(), "prototyping".to_string(),
                "local".to_string(), "debug".to_string(), "test".to_string(), "experimental".to_string(), "playground".to_string()
            ],
            priority_features: vec!["fast_startup".to_string(), "development_tools".to_string(), "debugging".to_string()],
            performance_preference: PerformancePreference::Balanced,
            security_preference: SecurityPreference::Minimal,
        });

        // Education Intent
        intent_patterns.insert("education".to_string(), ConfigurationIntent {
            keywords: vec![
                "education".to_string(), "learning".to_string(), "student".to_string(), "tutorial".to_string(), "beginner".to_string(),
                "course".to_string(), "class".to_string(), "academic".to_string(), "university".to_string(), "school".to_string()
            ],
            priority_features: vec!["simple_setup".to_string(), "good_defaults".to_string(), "educational_examples".to_string()],
            performance_preference: PerformancePreference::Balanced,
            security_preference: SecurityPreference::Balanced,
        });

        // Configuration Templates
        templates.insert("machine_learning".to_string(), ConfigurationTemplate {
            name: "Machine Learning Optimized".to_string(),
            description: "Optimized for ML workloads with GPU acceleration".to_string(),
            use_case: UsagePattern::MachineLearning,
            security_preference: SecurityPreference::Balanced,
            runtime_preferences: RuntimePreferences {
                enable_gpu: true,
                enable_python: true,
                enable_container: true,
                enable_wasm: false,
                gpu_memory_fraction: 0.9,
                python_memory_limit_gb: 16.0,
            },
            resource_preferences: ResourcePreferences {
                memory_allocation_strategy: "aggressive".to_string(),
                cpu_priority: "high".to_string(),
                storage_optimization: "speed".to_string(),
            },
            explicit_preferences: ExplicitPreferences {
                performance_priority: Some("maximum".to_string()),
                security_priority: Some("balanced".to_string()),
                memory_usage: Some("high".to_string()),
                use_gpu: Some(true),
                use_containers: Some(true),
            },
        });

        templates.insert("web_development".to_string(), ConfigurationTemplate {
            name: "Web Development".to_string(),
            description: "Optimized for web application development".to_string(),
            use_case: UsagePattern::WebDevelopment,
            security_preference: SecurityPreference::High,
            runtime_preferences: RuntimePreferences {
                enable_gpu: false,
                enable_python: false,
                enable_container: true,
                enable_wasm: true,
                gpu_memory_fraction: 0.0,
                python_memory_limit_gb: 4.0,
            },
            resource_preferences: ResourcePreferences {
                memory_allocation_strategy: "balanced".to_string(),
                cpu_priority: "normal".to_string(),
                storage_optimization: "balanced".to_string(),
            },
            explicit_preferences: ExplicitPreferences {
                performance_priority: Some("balanced".to_string()),
                security_priority: Some("high".to_string()),
                memory_usage: Some("normal".to_string()),
                use_gpu: Some(false),
                use_containers: Some(true),
            },
        });

        templates.insert("enterprise_security".to_string(), ConfigurationTemplate {
            name: "Enterprise Security".to_string(),
            description: "Maximum security for enterprise workloads".to_string(),
            use_case: UsagePattern::EnterpriseSecurity,
            security_preference: SecurityPreference::Maximum,
            runtime_preferences: RuntimePreferences {
                enable_gpu: false,
                enable_python: false,
                enable_container: true,
                enable_wasm: true,
                gpu_memory_fraction: 0.0,
                python_memory_limit_gb: 8.0,
            },
            resource_preferences: ResourcePreferences {
                memory_allocation_strategy: "conservative".to_string(),
                cpu_priority: "normal".to_string(),
                storage_optimization: "security".to_string(),
            },
            explicit_preferences: ExplicitPreferences {
                performance_priority: Some("security".to_string()),
                security_priority: Some("maximum".to_string()),
                memory_usage: Some("conservative".to_string()),
                use_gpu: Some(false),
                use_containers: Some(true),
            },
        });

        Self {
            auto_config: IntelligentAutoConfig::new(),
            intent_patterns,
            templates,
        }
    }

    /// Configure ToadStool from natural language description
    pub async fn configure_from_text(&mut self, text: &str) -> ToadStoolResult<ToadStoolConfig> {
        info!("🤖 Processing natural language configuration request...");
        debug!("Input text: {}", text);

        // Step 1: Analyze the text and extract intent
        let intent_analysis = self.analyze_intent(text).await?;
        info!("🎯 Detected intent: {} (confidence: {:.2})", 
              intent_analysis.primary_intent, intent_analysis.confidence);

        // Step 2: Create base configuration using auto-discovery
        let mut base_config = IntelligentAutoConfig::auto_configure().await?;

        // Step 3: Apply intent-specific modifications
        let modified_config = self.apply_intent_modifications(
            base_config,
            &intent_analysis
        ).await?;

        // Step 4: Validate and optimize the configuration
        let final_config = self.validate_and_optimize(modified_config).await?;

        info!("✅ Natural language configuration complete!");
        Ok(final_config)
    }

    /// Analyze text to extract configuration intent
    async fn analyze_intent(&self, text: &str) -> ToadStoolResult<IntentAnalysis> {
        let text_lower = text.to_lowercase();
        let mut intent_scores = HashMap::new();

        // Score each intent based on keyword matches
        for (intent_name, intent_pattern) in &self.intent_patterns {
            let mut score = 0.0;
            let mut matched_keywords = Vec::new();

            for keyword in &intent_pattern.keywords {
                if text_lower.contains(keyword) {
                    score += 1.0;
                    matched_keywords.push(keyword.clone());
                    
                    // Boost score for exact phrase matches
                    if text_lower.contains(&format!(" {} ", keyword)) {
                        score += 0.5;
                    }
                }
            }

            // Normalize score by number of keywords
            if !intent_pattern.keywords.is_empty() {
                score = score / intent_pattern.keywords.len() as f64;
            }

            if score > 0.0 {
                intent_scores.insert(intent_name.clone(), (score, matched_keywords));
                debug!("Intent '{}' scored {:.2} with keywords: {:?}", 
                       intent_name, score, matched_keywords);
            }
        }

        // Find the highest scoring intent
        let (primary_intent, (confidence, keywords)) = intent_scores
            .iter()
            .max_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, v)| (k.clone(), v.clone()))
            .unwrap_or_else(|| ("development".to_string(), (0.1, vec![])));

        // Extract explicit preferences from text
        let preferences = self.extract_explicit_preferences(&text_lower).await?;

        Ok(IntentAnalysis {
            primary_intent: primary_intent,
            confidence: confidence,
            matched_keywords: keywords.into_iter().map(|s| s.to_string()).collect(),
            secondary_intents: intent_scores.into_iter()
                .filter(|(k, _)| k != &primary_intent)
                .map(|(k, (score, _))| (k, score))
                .collect(),
            explicit_preferences: preferences,
        })
    }

    /// Extract explicit preferences from text
    async fn extract_explicit_preferences(&self, text: &str) -> ToadStoolResult<ExplicitPreferences> {
        let mut preferences = ExplicitPreferences::default();

        // Performance preferences
        if text.contains("fast") || text.contains("high performance") || text.contains("speed") {
            preferences.performance_priority = Some("high".to_string());
        }
        if text.contains("slow") || text.contains("power saver") || text.contains("battery") {
            preferences.performance_priority = Some("low".to_string());
        }

        // Security preferences
        if text.contains("secure") || text.contains("security") || text.contains("safe") {
            preferences.security_priority = Some("high".to_string());
        }
        if text.contains("insecure") || text.contains("no security") || text.contains("open") {
            preferences.security_priority = Some("low".to_string());
        }

        // Memory preferences
        if text.contains("high memory") || text.contains("lots of memory") || text.contains("memory intensive") {
            preferences.memory_usage = Some("high".to_string());
        }
        if text.contains("low memory") || text.contains("memory efficient") || text.contains("lightweight") {
            preferences.memory_usage = Some("low".to_string());
        }

        // GPU preferences
        if text.contains("gpu") || text.contains("graphics") || text.contains("cuda") || text.contains("opencl") {
            preferences.use_gpu = Some(true);
        }
        if text.contains("no gpu") || text.contains("cpu only") {
            preferences.use_gpu = Some(false);
        }

        // Container preferences
        if text.contains("container") || text.contains("docker") || text.contains("isolated") {
            preferences.use_containers = Some(true);
        }
        if text.contains("native") || text.contains("no container") || text.contains("direct") {
            preferences.use_containers = Some(false);
        }

        debug!("Extracted explicit preferences: {:?}", preferences);
        Ok(preferences)
    }

    /// Apply intent-specific modifications to base configuration
    async fn apply_intent_modifications(
        &self,
        mut config: ToadStoolConfig,
        intent_analysis: &IntentAnalysis,
    ) -> ToadStoolResult<ToadStoolConfig> {
        info!("🔧 Applying intent-specific modifications for: {}", intent_analysis.primary_intent);

        // Get the intent pattern and template
        let intent_pattern = self.intent_patterns.get(&intent_analysis.primary_intent)
            .ok_or_else(|| ToadStoolError::configuration(
                format!("Unknown intent: {}", intent_analysis.primary_intent)
            ))?;

        let template = self.templates.get(&intent_analysis.primary_intent);

        // Apply runtime modifications
        if let Some(template) = template {
            self.apply_runtime_preferences(&mut config, &template.runtime_preferences).await?;
            self.apply_resource_preferences(&mut config, &template.resource_preferences).await?;
            self.apply_security_preferences(&mut config, &template.security_preference).await?;
        }

        // Apply explicit user preferences (override template)
        self.apply_explicit_preferences(&mut config, &intent_analysis.explicit_preferences).await?;

        // Apply performance preference from intent
        self.apply_performance_preference(&mut config, &intent_pattern.performance_preference).await?;

        debug!("Applied modifications for intent: {}", intent_analysis.primary_intent);
        Ok(config)
    }

    /// Apply runtime preferences to configuration
    async fn apply_runtime_preferences(
        &self,
        config: &mut ToadStoolConfig,
        preferences: &RuntimePreferences,
    ) -> ToadStoolResult<()> {
        // Enable/disable GPU runtime
        config.runtime.enable_gpu = preferences.enable_gpu;
        if preferences.enable_gpu {
            config.runtime.gpu_memory_limit = Some((preferences.gpu_memory_fraction * 24.0 * 1024.0) as u64);
        }

        // Enable/disable Python runtime
        config.runtime.enable_python = preferences.enable_python;
        if preferences.enable_python {
            config.runtime.python_memory_limit = (preferences.python_memory_limit_gb * 1024.0) as u64;
        }

        // Enable/disable container runtime
        config.runtime.enable_container = preferences.enable_container;

        // Enable/disable WASM runtime
        config.runtime.enable_wasm = preferences.enable_wasm;

        debug!("Applied runtime preferences");
        Ok(())
    }

    /// Apply resource preferences to configuration
    async fn apply_resource_preferences(
        &self,
        config: &mut ToadStoolConfig,
        preferences: &ResourcePreferences,
    ) -> ToadStoolResult<()> {
        // Apply memory allocation strategy
        config.resources.cpu_allocation_strategy = preferences.memory_allocation_strategy.clone();

        // Apply storage optimization
        if preferences.storage_optimization == "cache_datasets" {
            config.resources.cache_storage_limit_gb *= 2.0; // Double cache for datasets
        } else if preferences.storage_optimization == "fast_builds" {
            config.resources.temp_storage_limit_gb *= 1.5; // More temp space for builds
        }

        debug!("Applied resource preferences");
        Ok(())
    }

    /// Apply security preferences to configuration
    async fn apply_security_preferences(
        &self,
        config: &mut ToadStoolConfig,
        preferences: &SecurityPreference,
    ) -> ToadStoolResult<()> {
        // Apply sandbox level
        config.security.enable_sandbox = match preferences {
            SecurityPreference::Minimal => false,
            SecurityPreference::Balanced => true,
            SecurityPreference::High => true,
            SecurityPreference::Maximum => true,
        };

        // Apply network isolation
        config.security.enable_network_isolation = true; // Always true for all security levels

        // Apply crypto verification
        config.security.enable_crypto_verification = true; // Always true for all security levels

        debug!("Applied security preferences");
        Ok(())
    }

    /// Apply explicit user preferences from natural language
    async fn apply_explicit_preferences(
        &self,
        config: &mut ToadStoolConfig,
        preferences: &ExplicitPreferences,
    ) -> ToadStoolResult<()> {
        // Apply performance priority
        if let Some(performance) = &preferences.performance_priority {
            match performance.as_str() {
                "high" => {
                    config.runtime.native_workers = (config.runtime.native_workers * 2).max(8);
                    config.resources.max_concurrent_executions *= 2;
                }
                "low" => {
                    config.runtime.native_workers = (config.runtime.native_workers / 2).max(1);
                    config.resources.max_concurrent_executions = (config.resources.max_concurrent_executions / 2).max(1);
                }
                _ => {}
            }
        }

        // Apply security priority
        if let Some(security) = &preferences.security_priority {
            match security.as_str() {
                "high" => {
                    config.security.enable_sandbox = true;
                    config.security.enable_network_isolation = true;
                    config.security.require_signed_workloads = true;
                }
                "low" => {
                    config.security.enable_sandbox = false;
                    config.security.enable_network_isolation = false;
                    config.security.require_signed_workloads = false;
                }
                _ => {}
            }
        }

        // Apply memory usage
        if let Some(memory) = &preferences.memory_usage {
            match memory.as_str() {
                "high" => {
                    config.resources.max_memory_percent = (config.resources.max_memory_percent * 1.2).min(90.0);
                }
                "low" => {
                    config.resources.max_memory_percent = (config.resources.max_memory_percent * 0.7).max(30.0);
                }
                _ => {}
            }
        }

        // Apply GPU preference
        if let Some(use_gpu) = preferences.use_gpu {
            config.runtime.enable_gpu = use_gpu;
        }

        // Apply container preference
        if let Some(use_containers) = preferences.use_containers {
            config.runtime.enable_container = use_containers;
        }

        debug!("Applied explicit user preferences");
        Ok(())
    }

    /// Apply performance preference from intent
    async fn apply_performance_preference(
        &self,
        config: &mut ToadStoolConfig,
        preference: &PerformancePreference,
    ) -> ToadStoolResult<()> {
        match preference {
            PerformancePreference::MaximumPerformance => {
                config.runtime.native_workers = (config.runtime.native_workers * 2).max(8);
                config.resources.max_concurrent_executions *= 2;
                config.resources.max_memory_percent = (config.resources.max_memory_percent * 1.1).min(85.0);
            }
            PerformancePreference::HighPerformance => {
                config.runtime.native_workers = (config.runtime.native_workers * 1.5) as u32;
                config.resources.max_concurrent_executions = (config.resources.max_concurrent_executions as f32 * 1.5) as u32;
            }
            PerformancePreference::Balanced => {
                // Keep defaults
            }
            PerformancePreference::PowerSaver => {
                config.runtime.native_workers = (config.runtime.native_workers / 2).max(1);
                config.resources.max_concurrent_executions = (config.resources.max_concurrent_executions / 2).max(1);
                config.resources.max_memory_percent = (config.resources.max_memory_percent * 0.8).max(40.0);
            }
        }

        debug!("Applied performance preference: {:?}", preference);
        Ok(())
    }

    /// Validate and optimize the final configuration
    async fn validate_and_optimize(
        &self,
        mut config: ToadStoolConfig,
    ) -> ToadStoolResult<ToadStoolConfig> {
        // Ensure at least one runtime is enabled
        if !config.runtime.enable_native && !config.runtime.enable_wasm && 
           !config.runtime.enable_container && !config.runtime.enable_gpu && 
           !config.runtime.enable_python {
            warn!("No runtimes enabled, enabling native runtime as fallback");
            config.runtime.enable_native = true;
            config.runtime.native_workers = 2;
        }

        // Ensure reasonable resource limits
        if config.resources.max_memory_percent < 10.0 {
            config.resources.max_memory_percent = 10.0;
        }
        if config.resources.max_memory_percent > 90.0 {
            config.resources.max_memory_percent = 90.0;
        }

        if config.resources.max_concurrent_executions == 0 {
            config.resources.max_concurrent_executions = 1;
        }

        // Validate security settings compatibility
        if config.security.enable_network_isolation && config.ecosystem.enable_songbird {
            warn!("Network isolation conflicts with ecosystem services, adjusting...");
            config.security.enable_network_isolation = false;
        }

        debug!("Configuration validated and optimized");
        Ok(config)
    }

    /// Get available configuration templates
    pub fn get_available_templates(&self) -> Vec<&ConfigurationTemplate> {
        self.templates.values().collect()
    }

    /// Configure from a specific template
    pub async fn configure_from_template(&mut self, template_name: &str) -> ToadStoolResult<ToadStoolConfig> {
        let template = self.templates.get(template_name)
            .ok_or_else(|| ToadStoolError::configuration(
                format!("Template not found: {}", template_name)
            ))?;

        info!("🎨 Configuring from template: {}", template.name);

        // Generate base configuration
        let mut config = IntelligentAutoConfig::auto_configure().await?;

        // Apply template preferences
        self.apply_runtime_preferences(&mut config, &template.runtime_preferences).await?;
        self.apply_resource_preferences(&mut config, &template.resource_preferences).await?;
        self.apply_security_preferences(&mut config, &template.security_preference).await?;

        // Validate and optimize
        let final_config = self.validate_and_optimize(config).await?;

        info!("✅ Template configuration complete: {}", template.name);
        Ok(final_config)
    }
}

impl Default for NaturalLanguageConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration intent extracted from natural language
#[derive(Debug, Clone)]
pub struct ConfigurationIntent {
    pub keywords: Vec<String>,
    pub priority_features: Vec<String>,
    pub performance_preference: PerformancePreference,
    pub security_preference: SecurityPreference,
}

/// Performance preference levels
#[derive(Debug, Clone)]
pub enum PerformancePreference {
    PowerSaver,
    Balanced,
    HighPerformance,
    MaximumPerformance,
}

/// Security preference levels for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityPreference {
    /// Minimal security for development
    Minimal,
    /// Balanced security for general use
    Balanced,
    /// High security for production
    High,
    /// Maximum security for sensitive workloads
    Maximum,
}

/// Usage patterns for different workload types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsagePattern {
    /// Machine learning and AI workloads
    MachineLearning,
    /// Web development and services
    WebDevelopment,
    /// Scientific computing
    ScientificComputing,
    /// General purpose computing
    GeneralPurpose,
    /// High performance computing
    HighPerformanceComputing,
    /// Development and testing
    Development,
    /// Custom usage pattern
    Custom(String),
}

impl Default for UsagePattern {
    fn default() -> Self {
        UsagePattern::GeneralPurpose
    }
}

/// Intent analysis results
#[derive(Debug, Clone)]
pub struct IntentAnalysis {
    pub primary_intent: String,
    pub confidence: f64,
    pub matched_keywords: Vec<String>,
    pub secondary_intents: Vec<(String, f64)>,
    pub explicit_preferences: ExplicitPreferences,
}

/// Explicit preferences extracted from text
#[derive(Debug, Clone, Default)]
pub struct ExplicitPreferences {
    pub performance_priority: Option<String>,
    pub security_priority: Option<String>,
    pub memory_usage: Option<String>,
    pub use_gpu: Option<bool>,
    pub use_containers: Option<bool>,
}

/// Configuration template for specific use cases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationTemplate {
    pub name: String,
    pub description: String,
    pub use_case: UsagePattern,
    pub security_preference: SecurityPreference,
    pub runtime_preferences: RuntimePreferences,
    pub resource_preferences: ResourcePreferences,
    pub explicit_preferences: ExplicitPreferences,
}

/// Runtime preferences in template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimePreferences {
    pub enable_gpu: bool,
    pub enable_python: bool,
    pub enable_container: bool,
    pub enable_wasm: bool,
    pub gpu_memory_fraction: f64,
    pub python_memory_limit_gb: f64,
}

/// Resource preferences in template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePreferences {
    pub memory_allocation_strategy: String,
    pub cpu_priority: String,
    pub storage_optimization: String,
}

/// Security preferences in template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPreferences {
    pub sandbox_level: String,
    pub network_isolation: bool,
    pub crypto_verification: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_natural_language_config_creation() {
        let nl_config = NaturalLanguageConfig::new();
        
        assert!(!nl_config.intent_patterns.is_empty());
        assert!(!nl_config.templates.is_empty());
        
        // Check that key intents are present
        assert!(nl_config.intent_patterns.contains_key("machine_learning"));
        assert!(nl_config.intent_patterns.contains_key("web_development"));
        assert!(nl_config.intent_patterns.contains_key("enterprise_security"));
    }

    #[tokio::test]
    async fn test_intent_analysis_machine_learning() {
        let nl_config = NaturalLanguageConfig::new();
        
        let analysis = nl_config.analyze_intent(
            "I want to train neural networks and do machine learning with GPU acceleration"
        ).await.unwrap();
        
        assert_eq!(analysis.primary_intent, "machine_learning");
        assert!(analysis.confidence > 0.0);
        assert!(!analysis.matched_keywords.is_empty());
    }

    #[tokio::test]
    async fn test_intent_analysis_web_development() {
        let nl_config = NaturalLanguageConfig::new();
        
        let analysis = nl_config.analyze_intent(
            "I'm building a web application with React and need to deploy containers"
        ).await.unwrap();
        
        assert_eq!(analysis.primary_intent, "web_development");
        assert!(analysis.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_explicit_preferences_extraction() {
        let nl_config = NaturalLanguageConfig::new();
        
        let preferences = nl_config.extract_explicit_preferences(
            "I need high performance with GPU acceleration and maximum security"
        ).await.unwrap();
        
        assert_eq!(preferences.performance_priority, Some("high".to_string()));
        assert_eq!(preferences.security_priority, Some("high".to_string()));
        assert_eq!(preferences.use_gpu, Some(true));
    }

    #[test]
    fn test_configuration_template_structure() {
        let nl_config = NaturalLanguageConfig::new();
        
        let ml_template = nl_config.templates.get("machine_learning").unwrap();
        assert_eq!(ml_template.name, "Machine Learning Optimized");
        assert!(ml_template.runtime_preferences.enable_gpu);
        assert!(ml_template.runtime_preferences.enable_python);
        assert!(ml_template.runtime_preferences.gpu_memory_fraction > 0.5);
        
        let web_template = nl_config.templates.get("web_development").unwrap();
        assert_eq!(web_template.name, "Web Development");
        assert!(web_template.runtime_preferences.enable_container);
        assert!(web_template.runtime_preferences.enable_wasm);
        assert!(!web_template.runtime_preferences.enable_gpu);
    }

    #[test]
    fn test_get_available_templates() {
        let nl_config = NaturalLanguageConfig::new();
        let templates = nl_config.get_available_templates();
        
        assert!(!templates.is_empty());
        
        let template_names: Vec<&String> = templates.iter().map(|t| &t.name).collect();
        assert!(template_names.iter().any(|&name| name.contains("Machine Learning")));
        assert!(template_names.iter().any(|&name| name.contains("Web Development")));
    }

    #[test]
    fn test_performance_preference_enum() {
        // Test that all performance preference variants exist
        let _power_saver = PerformancePreference::PowerSaver;
        let _balanced = PerformancePreference::Balanced;
        let _high_performance = PerformancePreference::HighPerformance;
        let _maximum_performance = PerformancePreference::MaximumPerformance;
    }

    #[test]
    fn test_security_preference_enum() {
        // Test that all security preference variants exist
        let _minimal = SecurityPreference::Minimal;
        let _balanced = SecurityPreference::Balanced;
        let _high = SecurityPreference::High;
        let _maximum = SecurityPreference::Maximum;
    }

    #[tokio::test]
    async fn test_intent_patterns_coverage() {
        let nl_config = NaturalLanguageConfig::new();
        
        // Test various input texts to ensure good coverage
        let test_cases = vec![
            ("train a machine learning model", "machine_learning"),
            ("build a web API with containers", "web_development"), 
            ("process large datasets", "data_processing"),
            ("need maximum security for enterprise", "enterprise_security"),
            ("learning to code", "education"),
            ("development environment setup", "development"),
        ];
        
        for (text, expected_intent) in test_cases {
            let analysis = nl_config.analyze_intent(text).await.unwrap();
            assert_eq!(analysis.primary_intent, expected_intent, 
                      "Failed for text: '{}'", text);
        }
    }
}
