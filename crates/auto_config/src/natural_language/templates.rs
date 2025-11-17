//! Configuration templates for common use cases
//!
//! This module provides pre-configured templates for common workload types,
//! making it easy to generate appropriate configurations from intent analysis.

use std::collections::{HashMap, HashSet};

use super::types::*;

/// Create default configuration templates
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
pub fn get_template(
    templates: &HashMap<String, ConfigurationTemplate>,
    name: &str,
) -> ConfigurationTemplate {
    templates
        .get(name)
        .cloned()
        .unwrap_or_else(|| templates.get("general_purpose").unwrap().clone())
}
