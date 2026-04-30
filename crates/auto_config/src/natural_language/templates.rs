// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configuration templates for common use cases
//!
//! This module provides pre-configured templates for common workload types,
//! making it easy to generate appropriate configurations from intent analysis.

use std::collections::{HashMap, HashSet};

use super::types::{
    ConfigurationTemplate, ExplicitPreferences, ResourcePreferences, RuntimePreferences,
    RuntimeType, SecurityPreference, UsagePattern,
};

/// Create default configuration templates
#[must_use]
pub fn create_templates() -> HashMap<String, ConfigurationTemplate> {
    let mut templates = HashMap::new();

    // Machine Learning Template
    templates.insert(
        "machine_learning".to_string(),
        ConfigurationTemplate {
            name: "Machine Learning".to_string(),
            description: "Optimized for ML training and inference with GPU acceleration"
                .to_string(),
            use_case: UsagePattern::MachineLearning,
            security_preference: SecurityPreference::Balanced,
            runtime_preferences: RuntimePreferences {
                enabled_runtimes: {
                    let mut set = HashSet::new();
                    set.insert(RuntimeType::Gpu);
                    set.insert(RuntimeType::Python);
                    set.insert(RuntimeType::Container);
                    set
                },
                gpu_memory_fraction: 0.8,
                python_memory_limit_gb: 16.0,
            },
            resource_preferences: ResourcePreferences {
                cpu_intensive: true,
                memory_intensive: true,
                requires_gpu: true,
                memory_allocation_strategy: "dynamic".to_string(),
                cpu_priority: "high".to_string(),
                storage_optimization: "performance".to_string(),
            },
            explicit_preferences: ExplicitPreferences {
                performance_priority: Some("high".to_string()),
                use_gpu: Some(true),
                ..Default::default()
            },
        },
    );

    // Web Development Template
    templates.insert(
        "web_development".to_string(),
        ConfigurationTemplate {
            name: "Web Development".to_string(),
            description: "Fast startup and hot reload for web applications".to_string(),
            use_case: UsagePattern::WebDevelopment,
            security_preference: SecurityPreference::High,
            runtime_preferences: RuntimePreferences {
                enabled_runtimes: {
                    let mut set = HashSet::new();
                    set.insert(RuntimeType::Container);
                    set.insert(RuntimeType::Wasm);
                    set
                },
                gpu_memory_fraction: 0.0,
                python_memory_limit_gb: 2.0,
            },
            resource_preferences: ResourcePreferences {
                cpu_intensive: false,
                memory_intensive: false,
                requires_gpu: false,
                memory_allocation_strategy: "balanced".to_string(),
                cpu_priority: "medium".to_string(),
                storage_optimization: "balanced".to_string(),
            },
            explicit_preferences: ExplicitPreferences {
                performance_priority: Some("balanced".to_string()),
                security_priority: Some("high".to_string()),
                use_containers: Some(true),
                ..Default::default()
            },
        },
    );

    // Data Processing Template
    templates.insert(
        "data_processing".to_string(),
        ConfigurationTemplate {
            name: "Data Processing".to_string(),
            description: "High throughput for ETL and batch processing".to_string(),
            use_case: UsagePattern::ScientificComputing,
            security_preference: SecurityPreference::Balanced,
            runtime_preferences: RuntimePreferences {
                enabled_runtimes: {
                    let mut set = HashSet::new();
                    set.insert(RuntimeType::Container);
                    set.insert(RuntimeType::Python);
                    set
                },
                gpu_memory_fraction: 0.0,
                python_memory_limit_gb: 32.0,
            },
            resource_preferences: ResourcePreferences {
                cpu_intensive: true,
                memory_intensive: true,
                requires_gpu: false,
                memory_allocation_strategy: "aggressive".to_string(),
                cpu_priority: "high".to_string(),
                storage_optimization: "throughput".to_string(),
            },
            explicit_preferences: ExplicitPreferences {
                performance_priority: Some("high".to_string()),
                memory_usage: Some("high".to_string()),
                ..Default::default()
            },
        },
    );

    // Gaming/Graphics Template
    templates.insert(
        "gaming".to_string(),
        ConfigurationTemplate {
            name: "Gaming & Graphics".to_string(),
            description: "Maximum GPU performance for gaming and graphics rendering".to_string(),
            use_case: UsagePattern::Custom("Gaming & Graphics Rendering".to_string()),
            security_preference: SecurityPreference::Minimal,
            runtime_preferences: RuntimePreferences {
                enabled_runtimes: {
                    let mut set = HashSet::new();
                    set.insert(RuntimeType::Gpu);
                    set.insert(RuntimeType::Container);
                    set
                },
                gpu_memory_fraction: 0.95,
                python_memory_limit_gb: 8.0,
            },
            resource_preferences: ResourcePreferences {
                cpu_intensive: true,
                memory_intensive: true,
                requires_gpu: true,
                memory_allocation_strategy: "aggressive".to_string(),
                cpu_priority: "high".to_string(),
                storage_optimization: "performance".to_string(),
            },
            explicit_preferences: ExplicitPreferences {
                performance_priority: Some("maximum".to_string()),
                use_gpu: Some(true),
                ..Default::default()
            },
        },
    );

    // General Purpose Template
    templates.insert(
        "general_purpose".to_string(),
        ConfigurationTemplate {
            name: "General Purpose".to_string(),
            description: "Balanced configuration for general computing".to_string(),
            use_case: UsagePattern::GeneralPurpose,
            security_preference: SecurityPreference::Balanced,
            runtime_preferences: RuntimePreferences {
                enabled_runtimes: {
                    let mut set = HashSet::new();
                    set.insert(RuntimeType::Container);
                    set.insert(RuntimeType::Wasm);
                    set
                },
                gpu_memory_fraction: 0.0,
                python_memory_limit_gb: 4.0,
            },
            resource_preferences: ResourcePreferences {
                cpu_intensive: false,
                memory_intensive: false,
                requires_gpu: false,
                memory_allocation_strategy: "balanced".to_string(),
                cpu_priority: "medium".to_string(),
                storage_optimization: "balanced".to_string(),
            },
            explicit_preferences: ExplicitPreferences::default(),
        },
    );

    templates
}

/// Get a template by name, returning general purpose if not found
/// ✅ FIXED: Safe fallback chain - no `unwrap()` that could panic
#[must_use]
#[expect(
    clippy::implicit_hasher,
    reason = "standard HashMap sufficient for templates"
)]
pub fn get_template(
    templates: &HashMap<String, ConfigurationTemplate>,
    name: &str,
) -> ConfigurationTemplate {
    templates
        .get(name)
        .or_else(|| templates.get("general_purpose"))
        .cloned()
        .unwrap_or_else(|| {
            // Ultimate fallback if templates HashMap is somehow empty
            use crate::natural_language::types::{SecurityPreference, UsagePattern};
            use std::collections::HashSet;
            ConfigurationTemplate {
                name: "default".to_string(),
                description: "Default fallback template".to_string(),
                use_case: UsagePattern::GeneralPurpose,
                security_preference: SecurityPreference::Balanced,
                runtime_preferences: RuntimePreferences {
                    enabled_runtimes: HashSet::new(),
                    gpu_memory_fraction: 0.7,
                    python_memory_limit_gb: 4.0,
                },
                resource_preferences: ResourcePreferences {
                    cpu_intensive: false,
                    memory_intensive: false,
                    requires_gpu: false,
                    memory_allocation_strategy: "balanced".to_string(),
                    cpu_priority: "normal".to_string(),
                    storage_optimization: "balanced".to_string(),
                },
                explicit_preferences: ExplicitPreferences {
                    performance_priority: None,
                    security_priority: None,
                    memory_usage: None,
                    use_gpu: None,
                    use_containers: None,
                },
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_templates_has_all_keys() {
        let t = create_templates();
        assert!(t.contains_key("machine_learning"));
        assert!(t.contains_key("web_development"));
        assert!(t.contains_key("data_processing"));
        assert!(t.contains_key("gaming"));
        assert!(t.contains_key("general_purpose"));
        assert_eq!(t.len(), 5);
    }

    #[test]
    fn ml_template_requires_gpu() {
        let t = create_templates();
        let ml = &t["machine_learning"];
        assert!(ml.resource_preferences.requires_gpu);
        assert!(
            ml.runtime_preferences
                .enabled_runtimes
                .contains(&RuntimeType::Gpu)
        );
        assert!(ml.explicit_preferences.use_gpu == Some(true));
    }

    #[test]
    fn web_dev_template_no_gpu() {
        let t = create_templates();
        let web = &t["web_development"];
        assert!(!web.resource_preferences.requires_gpu);
        assert!(
            web.runtime_preferences
                .enabled_runtimes
                .contains(&RuntimeType::Container)
        );
    }

    #[test]
    fn data_processing_template_high_memory() {
        let t = create_templates();
        let dp = &t["data_processing"];
        assert!(dp.resource_preferences.memory_intensive);
        assert!(dp.runtime_preferences.python_memory_limit_gb > 16.0);
    }

    #[test]
    fn gaming_template_max_gpu() {
        let t = create_templates();
        let g = &t["gaming"];
        assert!(g.runtime_preferences.gpu_memory_fraction > 0.9);
        assert!(g.resource_preferences.requires_gpu);
    }

    #[test]
    fn get_template_existing() {
        let t = create_templates();
        let ml = get_template(&t, "machine_learning");
        assert_eq!(ml.name, "Machine Learning");
    }

    #[test]
    fn get_template_unknown_falls_back_to_general() {
        let t = create_templates();
        let fallback = get_template(&t, "nonexistent");
        assert_eq!(fallback.name, "General Purpose");
    }

    #[test]
    fn get_template_empty_map_uses_ultimate_fallback() {
        let empty: HashMap<String, ConfigurationTemplate> = HashMap::new();
        let fallback = get_template(&empty, "anything");
        assert_eq!(fallback.name, "default");
    }
}
