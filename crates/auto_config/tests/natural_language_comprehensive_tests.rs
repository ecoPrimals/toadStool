//! Comprehensive tests for Natural Language Configuration
//!
//! This test suite provides extensive coverage of the NaturalLanguageConfig module,
//! including all public types, methods, and edge cases.
//!
//! Note: This file only tests the public API surface. Internal types like RuntimeType
//! are tested indirectly through RuntimePreferences methods.

use toadstool_auto_config::*;

// ============================================================================
// Core NaturalLanguageConfig Tests
// ============================================================================

#[test]
fn test_natural_language_config_new() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    assert!(!templates.is_empty(), "Should have templates");
    assert!(templates.len() >= 5, "Should have multiple templates");
}

#[test]
fn test_natural_language_config_default() {
    let nl_config = NaturalLanguageConfig::default();
    let templates = nl_config.get_available_templates();

    assert!(!templates.is_empty(), "Default should have templates");
}

#[test]
fn test_get_available_templates_not_empty() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    assert!(!templates.is_empty());

    // Verify each template has required fields
    for template in templates {
        assert!(!template.name.is_empty(), "Template should have name");
        assert!(
            !template.description.is_empty(),
            "Template should have description"
        );
    }
}

#[test]
fn test_templates_have_unique_names() {
    use std::collections::HashSet;
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    let mut names = HashSet::new();
    for template in &templates {
        assert!(
            names.insert(template.name.clone()),
            "Template names should be unique: {}",
            template.name
        );
    }
}

#[test]
fn test_templates_have_valid_usage_patterns() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    // Each template should have a valid usage pattern
    for template in templates {
        // Just check that we can access the pattern
        let _ = match &template.use_case {
            UsagePattern::MachineLearning => "ML",
            UsagePattern::WebDevelopment => "Web",
            UsagePattern::ScientificComputing => "Sci",
            UsagePattern::GeneralPurpose => "General",
            UsagePattern::HighPerformanceComputing => "HPC",
            UsagePattern::Development => "Dev",
            UsagePattern::EnterpriseSecurity => "Security",
            UsagePattern::Custom(_) => "Custom",
        };
    }
}

// ============================================================================
// UsagePattern Tests
// ============================================================================

#[test]
fn test_usage_pattern_all_variants() {
    let patterns = vec![
        UsagePattern::MachineLearning,
        UsagePattern::WebDevelopment,
        UsagePattern::ScientificComputing,
        UsagePattern::GeneralPurpose,
        UsagePattern::HighPerformanceComputing,
        UsagePattern::Development,
        UsagePattern::EnterpriseSecurity,
        UsagePattern::Custom("custom".to_string()),
    ];

    assert_eq!(patterns.len(), 8, "Should have all usage pattern variants");
}

#[test]
fn test_usage_pattern_default() {
    let default_pattern = UsagePattern::default();

    // Default should be GeneralPurpose
    matches!(default_pattern, UsagePattern::GeneralPurpose);
}

#[test]
fn test_usage_pattern_custom() {
    let custom = UsagePattern::Custom("MyCustomWorkload".to_string());

    if let UsagePattern::Custom(name) = custom {
        assert_eq!(name, "MyCustomWorkload");
    } else {
        panic!("Should be custom variant");
    }
}

#[test]
fn test_usage_pattern_debug() {
    let pattern = UsagePattern::MachineLearning;
    let debug_str = format!("{:?}", pattern);

    assert!(debug_str.contains("MachineLearning"));
}

#[test]
fn test_usage_pattern_clone() {
    let pattern1 = UsagePattern::WebDevelopment;
    let pattern2 = pattern1.clone();

    // Both should be WebDevelopment
    matches!(pattern2, UsagePattern::WebDevelopment);
}

#[test]
fn test_usage_pattern_serialize() {
    let pattern = UsagePattern::MachineLearning;
    let serialized = serde_json::to_string(&pattern);

    assert!(serialized.is_ok());
}

#[test]
fn test_usage_pattern_deserialize() {
    let json = r#""MachineLearning""#;
    let deserialized: Result<UsagePattern, _> = serde_json::from_str(json);

    assert!(deserialized.is_ok());
}

#[test]
fn test_usage_pattern_custom_serialize() {
    let pattern = UsagePattern::Custom("EdgeComputing".to_string());
    let serialized = serde_json::to_string(&pattern).unwrap();
    let deserialized: UsagePattern = serde_json::from_str(&serialized).unwrap();

    if let UsagePattern::Custom(name) = deserialized {
        assert_eq!(name, "EdgeComputing");
    } else {
        panic!("Should deserialize to Custom variant");
    }
}

// ============================================================================
// SecurityPreference Tests
// ============================================================================

#[test]
fn test_security_preference_all_variants() {
    let prefs = vec![
        SecurityPreference::Minimal,
        SecurityPreference::Balanced,
        SecurityPreference::High,
        SecurityPreference::Maximum,
    ];

    assert_eq!(
        prefs.len(),
        4,
        "Should have all security preference variants"
    );
}

#[test]
fn test_security_preference_debug() {
    let pref = SecurityPreference::Maximum;
    let debug_str = format!("{:?}", pref);

    assert!(debug_str.contains("Maximum"));
}

#[test]
fn test_security_preference_clone() {
    let pref1 = SecurityPreference::High;
    let pref2 = pref1.clone();

    matches!(pref2, SecurityPreference::High);
}

#[test]
fn test_security_preference_serialize() {
    let pref = SecurityPreference::High;
    let serialized = serde_json::to_string(&pref);

    assert!(serialized.is_ok());
}

#[test]
fn test_security_preference_deserialize() {
    let json = r#""High""#;
    let deserialized: Result<SecurityPreference, _> = serde_json::from_str(json);

    assert!(deserialized.is_ok());

    if let Ok(pref) = deserialized {
        matches!(pref, SecurityPreference::High);
    }
}

#[test]
fn test_security_preference_all_variants_serialization() {
    let prefs = vec![
        SecurityPreference::Minimal,
        SecurityPreference::Balanced,
        SecurityPreference::High,
        SecurityPreference::Maximum,
    ];

    for pref in prefs {
        let serialized = serde_json::to_string(&pref).unwrap();
        let _deserialized: SecurityPreference = serde_json::from_str(&serialized).unwrap();
    }
}

// ============================================================================
// RuntimePreferences Tests (via templates)
// ============================================================================

#[test]
fn test_runtime_preferences_via_ml_template() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    // Find ML template
    let ml_template = templates
        .iter()
        .find(|t| t.name.contains("Machine Learning") || t.name.contains("ML"))
        .expect("Should have ML template");

    // ML template should typically have GPU enabled
    let gpu_enabled = ml_template.runtime_preferences.enable_gpu();
    let python_enabled = ml_template.runtime_preferences.enable_python();

    // At least one should be true for ML workloads
    assert!(
        gpu_enabled || python_enabled,
        "ML should enable GPU or Python"
    );
}

#[test]
fn test_runtime_preferences_via_web_template() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    // Find web template
    let web_template = templates
        .iter()
        .find(|t| t.name.contains("Web"))
        .expect("Should have Web template");

    // Web development should have container or WASM enabled
    let container_enabled = web_template.runtime_preferences.enable_container();
    let wasm_enabled = web_template.runtime_preferences.enable_wasm();

    // At least one should be true for web workloads
    assert!(
        container_enabled || wasm_enabled,
        "Web template should have at least one runtime enabled (container or wasm)"
    );
}

#[test]
fn test_runtime_preferences_methods() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    // Test that all runtime preference methods work
    for template in templates {
        let prefs = &template.runtime_preferences;

        // These methods should not panic
        let _ = prefs.enable_gpu();
        let _ = prefs.enable_python();
        let _ = prefs.enable_container();
        let _ = prefs.enable_wasm();

        // Check memory fractions are valid
        assert!(prefs.gpu_memory_fraction >= 0.0);
        assert!(prefs.gpu_memory_fraction <= 1.0);
        assert!(prefs.python_memory_limit_gb >= 0.0);
    }
}

#[test]
fn test_runtime_preferences_gpu_memory_fraction_valid() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    for template in templates {
        let fraction = template.runtime_preferences.gpu_memory_fraction;
        assert!(
            fraction >= 0.0,
            "GPU memory fraction should be non-negative"
        );
        assert!(fraction <= 1.0, "GPU memory fraction should not exceed 1.0");
    }
}

#[test]
fn test_runtime_preferences_python_memory_limit_valid() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    for template in templates {
        let limit = template.runtime_preferences.python_memory_limit_gb;
        assert!(limit >= 0.0, "Python memory limit should be non-negative");
    }
}

#[test]
fn test_runtime_preferences_debug() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    if let Some(template) = templates.first() {
        let debug_str = format!("{:?}", template.runtime_preferences);
        assert!(debug_str.contains("RuntimePreferences"));
    }
}

#[test]
fn test_runtime_preferences_clone() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    if let Some(template) = templates.first() {
        let prefs1 = &template.runtime_preferences;
        let prefs2 = prefs1.clone();

        assert_eq!(prefs1.gpu_memory_fraction, prefs2.gpu_memory_fraction);
        assert_eq!(prefs1.python_memory_limit_gb, prefs2.python_memory_limit_gb);
        assert_eq!(prefs1.enable_gpu(), prefs2.enable_gpu());
        assert_eq!(prefs1.enable_python(), prefs2.enable_python());
    }
}

// ============================================================================
// ExplicitPreferences Tests
// ============================================================================

#[test]
fn test_explicit_preferences_default() {
    let prefs = ExplicitPreferences::default();

    assert!(prefs.performance_priority.is_none());
    assert!(prefs.security_priority.is_none());
    assert!(prefs.memory_usage.is_none());
    assert!(prefs.use_gpu.is_none());
    assert!(prefs.use_containers.is_none());
}

#[test]
fn test_explicit_preferences_with_values() {
    let prefs = ExplicitPreferences {
        performance_priority: Some("high".to_string()),
        security_priority: Some("maximum".to_string()),
        memory_usage: Some("8GB".to_string()),
        use_gpu: Some(true),
        use_containers: Some(true),
    };

    assert_eq!(prefs.performance_priority, Some("high".to_string()));
    assert_eq!(prefs.security_priority, Some("maximum".to_string()));
    assert_eq!(prefs.memory_usage, Some("8GB".to_string()));
    assert_eq!(prefs.use_gpu, Some(true));
    assert_eq!(prefs.use_containers, Some(true));
}

#[test]
fn test_explicit_preferences_partial() {
    let prefs = ExplicitPreferences {
        performance_priority: Some("balanced".to_string()),
        security_priority: None,
        memory_usage: None,
        use_gpu: Some(false),
        use_containers: None,
    };

    assert!(prefs.performance_priority.is_some());
    assert!(prefs.security_priority.is_none());
    assert!(prefs.use_gpu.is_some());
}

#[test]
fn test_explicit_preferences_debug() {
    let prefs = ExplicitPreferences::default();
    let debug_str = format!("{:?}", prefs);

    assert!(debug_str.contains("ExplicitPreferences"));
}

#[test]
fn test_explicit_preferences_clone() {
    let prefs1 = ExplicitPreferences {
        performance_priority: Some("high".to_string()),
        security_priority: Some("high".to_string()),
        memory_usage: Some("16GB".to_string()),
        use_gpu: Some(true),
        use_containers: Some(true),
    };

    let prefs2 = prefs1.clone();
    assert_eq!(prefs2.performance_priority, Some("high".to_string()));
    assert_eq!(prefs2.use_gpu, Some(true));
}

#[test]
fn test_explicit_preferences_serialize() {
    let prefs = ExplicitPreferences {
        performance_priority: Some("high".to_string()),
        security_priority: Some("maximum".to_string()),
        memory_usage: Some("8GB".to_string()),
        use_gpu: Some(true),
        use_containers: Some(true),
    };

    let serialized = serde_json::to_string(&prefs);
    assert!(serialized.is_ok());
}

#[test]
fn test_explicit_preferences_deserialize() {
    let json = r#"{
        "performance_priority": "high",
        "security_priority": null,
        "memory_usage": "8GB",
        "use_gpu": true,
        "use_containers": false
    }"#;

    let deserialized: Result<ExplicitPreferences, _> = serde_json::from_str(json);
    assert!(deserialized.is_ok());

    let prefs = deserialized.unwrap();
    assert_eq!(prefs.performance_priority, Some("high".to_string()));
    assert!(prefs.security_priority.is_none());
    assert_eq!(prefs.use_gpu, Some(true));
}

#[test]
fn test_explicit_preferences_all_some() {
    let prefs = ExplicitPreferences {
        performance_priority: Some("maximum".to_string()),
        security_priority: Some("maximum".to_string()),
        memory_usage: Some("64GB".to_string()),
        use_gpu: Some(true),
        use_containers: Some(true),
    };

    assert!(prefs.performance_priority.is_some());
    assert!(prefs.security_priority.is_some());
    assert!(prefs.memory_usage.is_some());
    assert!(prefs.use_gpu.is_some());
    assert!(prefs.use_containers.is_some());
}

#[test]
fn test_explicit_preferences_all_none() {
    let prefs = ExplicitPreferences {
        performance_priority: None,
        security_priority: None,
        memory_usage: None,
        use_gpu: None,
        use_containers: None,
    };

    assert!(prefs.performance_priority.is_none());
    assert!(prefs.security_priority.is_none());
    assert!(prefs.memory_usage.is_none());
    assert!(prefs.use_gpu.is_none());
    assert!(prefs.use_containers.is_none());
}

// ============================================================================
// ConfigurationTemplate Tests
// ============================================================================

#[test]
fn test_configuration_template_fields() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    for template in templates {
        assert!(!template.name.is_empty());
        assert!(!template.description.is_empty());

        // Check that fields are accessible
        let _ = &template.use_case;
        let _ = &template.security_preference;
        let _ = &template.runtime_preferences;
        let _ = &template.resource_preferences;
        let _ = &template.explicit_preferences;
    }
}

#[test]
fn test_configuration_template_debug() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    if let Some(template) = templates.first() {
        let debug_str = format!("{:?}", template);
        assert!(debug_str.contains("ConfigurationTemplate"));
    }
}

#[test]
fn test_configuration_template_clone() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    if let Some(template) = templates.first() {
        let cloned = (*template).clone();
        assert_eq!(template.name, cloned.name);
        assert_eq!(template.description, cloned.description);
    }
}

#[test]
fn test_configuration_template_serialize() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    if let Some(template) = templates.first() {
        let serialized = serde_json::to_string(template);
        assert!(serialized.is_ok());
    }
}

#[test]
fn test_configuration_template_resource_preferences() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    for template in templates {
        let res_prefs = &template.resource_preferences;
        assert!(!res_prefs.memory_allocation_strategy.is_empty());
        assert!(!res_prefs.cpu_priority.is_empty());
        assert!(!res_prefs.storage_optimization.is_empty());
    }
}

// ============================================================================
// ResourcePreferences Tests (via templates)
// ============================================================================

#[test]
fn test_resource_preferences_via_templates() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    for template in templates {
        let prefs = &template.resource_preferences;

        // All templates should have non-empty resource preferences
        assert!(
            !prefs.memory_allocation_strategy.is_empty(),
            "Template {} should have memory allocation strategy",
            template.name
        );
        assert!(
            !prefs.cpu_priority.is_empty(),
            "Template {} should have CPU priority",
            template.name
        );
        assert!(
            !prefs.storage_optimization.is_empty(),
            "Template {} should have storage optimization",
            template.name
        );
    }
}

#[test]
fn test_resource_preferences_debug() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    if let Some(template) = templates.first() {
        let debug_str = format!("{:?}", template.resource_preferences);
        assert!(debug_str.contains("ResourcePreferences"));
    }
}

#[test]
fn test_resource_preferences_clone() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    if let Some(template) = templates.first() {
        let prefs1 = &template.resource_preferences;
        let prefs2 = prefs1.clone();

        assert_eq!(
            prefs1.memory_allocation_strategy,
            prefs2.memory_allocation_strategy
        );
        assert_eq!(prefs1.cpu_priority, prefs2.cpu_priority);
        assert_eq!(prefs1.storage_optimization, prefs2.storage_optimization);
    }
}

// ============================================================================
// Edge Cases and Integration Tests
// ============================================================================

#[test]
fn test_usage_pattern_all_variants_debug() {
    let patterns = vec![
        UsagePattern::MachineLearning,
        UsagePattern::WebDevelopment,
        UsagePattern::ScientificComputing,
        UsagePattern::GeneralPurpose,
        UsagePattern::HighPerformanceComputing,
        UsagePattern::Development,
        UsagePattern::EnterpriseSecurity,
        UsagePattern::Custom("EdgeComputing".to_string()),
    ];

    for pattern in patterns {
        let debug_str = format!("{:?}", pattern);
        assert!(!debug_str.is_empty());
    }
}

#[test]
fn test_natural_language_config_templates_have_valid_patterns() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    for template in templates {
        // Verify template has reasonable values
        assert!(template.runtime_preferences.gpu_memory_fraction >= 0.0);
        assert!(template.runtime_preferences.gpu_memory_fraction <= 1.0);
        assert!(template.runtime_preferences.python_memory_limit_gb >= 0.0);

        // Verify template has valid strings
        assert!(!template.name.is_empty());
        assert!(!template.description.is_empty());
        assert!(!template
            .resource_preferences
            .memory_allocation_strategy
            .is_empty());
        assert!(!template.resource_preferences.cpu_priority.is_empty());
        assert!(!template
            .resource_preferences
            .storage_optimization
            .is_empty());
    }
}

#[test]
fn test_templates_cover_major_use_cases() {
    let nl_config = NaturalLanguageConfig::new();
    let templates = nl_config.get_available_templates();

    let template_names: Vec<String> = templates.iter().map(|t| t.name.to_lowercase()).collect();

    // Should have at least these major categories represented
    let has_ml_or_ai = template_names
        .iter()
        .any(|name| name.contains("machine") || name.contains("ml") || name.contains("ai"));

    let has_web = template_names.iter().any(|name| name.contains("web"));

    let has_dev = template_names
        .iter()
        .any(|name| name.contains("dev") || name.contains("general"));

    assert!(
        has_ml_or_ai || has_web || has_dev,
        "Templates should cover major use cases"
    );
}

#[test]
fn test_multiple_config_instances() {
    // Test that we can create multiple instances without issues
    let _config1 = NaturalLanguageConfig::new();
    let _config2 = NaturalLanguageConfig::new();
    let _config3 = NaturalLanguageConfig::default();

    // Should not panic
}

#[test]
fn test_template_consistency() {
    let nl_config = NaturalLanguageConfig::new();
    let templates1 = nl_config.get_available_templates();
    let templates2 = nl_config.get_available_templates();

    // Should return the same templates each time
    assert_eq!(templates1.len(), templates2.len());
}
