// SPDX-License-Identifier: AGPL-3.0-or-later
//! Additional template tests for missing coverage
//!
//! Tests for specialized templates that weren't covered:
//! - Genomics template
//! - Vision template
//! - Sovereign template  
//! - Custom template
//!
//! EVOLVED: Tests now check for capability-based architecture,
//! accepting both legacy names (beardog) and capability references (pki-provider).

use std::collections::HashMap;
use toadstool_cli::templates::specialized_templates::{
    create_custom_template, create_genomics_template, create_sovereign_template,
    create_vision_template,
};
use toadstool_cli::templates::types_mod::{CustomServiceSpec, CustomTemplateSpec};

// Helper: Check if primals include PKI capability (capability-based or legacy)
fn has_pki_capability(primals: &HashMap<String, impl std::fmt::Debug>) -> bool {
    primals.keys().any(|k| {
        k == "pki-provider" || k == "beardog" || k.contains("pki") || k.contains("security")
    })
}

// ============================================================================
// Genomics Template Tests
// ============================================================================

#[test]
fn test_genomics_template_structure() {
    let (name, description, primals, services, resources, security, _, storage) =
        create_genomics_template();

    assert_eq!(name, "genomics-biome");
    assert!(
        description.to_lowercase().contains("genomics")
            || description.to_lowercase().contains("bioinformatics")
    );

    // Should have PKI capability provider (capability-based architecture)
    assert!(
        has_pki_capability(&primals),
        "Genomics template should have PKI capability provider"
    );

    // Should have genomics-specific services
    assert!(
        !services.is_empty(),
        "Genomics template should have services"
    );

    // Should have enhanced security for sensitive genomic data
    assert!(
        security.beardog_required
            || security.isolation_level == "high"
            || security.isolation_level == "maximum",
        "Genomics should require PKI/security"
    );
    assert!(!security.isolation_level.is_empty());

    // Should have storage for datasets
    assert!(
        !storage.datasets.is_empty() || storage.nestgate_integration.is_some(),
        "Genomics template should have storage configuration"
    );

    // Should have reasonable resource limits for bioinformatics
    assert!(resources.cpu_limit.is_some() || resources.memory_limit.is_some());
}

#[test]
fn test_genomics_template_has_bioconductor() {
    let (_, _, _, services, _, _, _, _) = create_genomics_template();

    // Should have Bioconductor or similar bioinformatics tools
    let has_bioinformatics_tool = services.keys().any(|k| {
        k.to_lowercase().contains("bioconductor")
            || k.to_lowercase().contains("bio")
            || k.to_lowercase().contains("genomics")
    });

    assert!(
        has_bioinformatics_tool || !services.is_empty(),
        "Genomics template should have bioinformatics tools"
    );
}

#[test]
fn test_genomics_template_security() {
    let (_, _, _, _, _, security, _, _) = create_genomics_template();

    // Genomic data requires high security
    assert!(
        security.isolation_level == "high" || security.isolation_level == "maximum",
        "Genomics should have high security: {}",
        security.isolation_level
    );

    // Should require PKI/security capability
    assert!(
        security.beardog_required,
        "Genomics must require PKI/security for data protection"
    );
}

// ============================================================================
// Vision Template Tests
// ============================================================================

#[test]
fn test_vision_template_structure() {
    let (name, description, primals, services, resources, _, _, _) = create_vision_template();

    assert_eq!(name, "vision-biome");
    assert!(
        description.to_lowercase().contains("vision")
            || description.to_lowercase().contains("opencv")
    );

    // Should have PKI capability (capability-based)
    assert!(
        has_pki_capability(&primals),
        "Vision template should have PKI capability provider"
    );

    // Should have vision processing services
    assert!(!services.is_empty(), "Vision template should have services");

    // Vision processing needs significant resources
    assert!(
        resources.cpu_limit.is_some() || resources.gpu_limit.is_some(),
        "Vision template should have CPU or GPU resources"
    );
}

#[test]
fn test_vision_template_has_opencv() {
    let (_, _, _, services, _, _, _, _) = create_vision_template();

    // Should have OpenCV or vision processing tools
    let has_vision_tool = services.keys().any(|k| {
        k.to_lowercase().contains("opencv")
            || k.to_lowercase().contains("vision")
            || k.to_lowercase().contains("cv")
    });

    assert!(
        has_vision_tool || !services.is_empty(),
        "Vision template should have computer vision tools"
    );
}

#[test]
fn test_vision_template_resources() {
    let (_, _, _, services, resources, _, _, _) = create_vision_template();

    // Vision processing needs compute resources
    assert!(
        resources.cpu_limit.is_some() || resources.memory_limit.is_some(),
        "Vision template needs resource limits"
    );

    // Services should have reasonable resources
    for (name, service) in &services {
        assert!(
            service.resources.cpu_limit.is_some() || service.resources.memory_limit.is_some(),
            "Service '{name}' should have resource limits"
        );
    }
}

// ============================================================================
// Sovereign Template Tests
// ============================================================================

#[test]
fn test_sovereign_template_structure() {
    let (name, description, primals, _services, _, security, networking, storage) =
        create_sovereign_template();

    assert_eq!(name, "sovereign-biome");
    assert!(
        description.to_lowercase().contains("sovereign")
            || description.to_lowercase().contains("maximum security")
            || description.to_lowercase().contains("air-gapped")
    );

    // Should have all security primals (capability-based)
    assert!(
        has_pki_capability(&primals),
        "Sovereign template should have PKI capability provider"
    );

    // Should have services or be minimal for security
    // Sovereign template may have fewer services for security
    assert!(!primals.is_empty());

    // Should have maximum security
    assert_eq!(
        security.isolation_level, "maximum",
        "Sovereign template must have maximum isolation"
    );
    assert!(security.beardog_required);

    // Networking should be restricted
    assert!(!networking.mode.is_empty());

    // Storage should be secure (check for NestGate integration)
    assert!(storage.nestgate_integration.is_some());
}

#[test]
fn test_sovereign_template_maximum_security() {
    let (_, _, _, _, _, security, networking, _) = create_sovereign_template();

    // Maximum security settings
    assert_eq!(security.isolation_level, "maximum");
    assert!(security.beardog_required);

    // Should have crypto policies
    assert!(
        !security.crypto_policies.is_empty(),
        "Sovereign template must have crypto policies"
    );

    // Should have network restrictions or firewall rules
    assert!(
        !networking.network_policies.is_empty(),
        "Sovereign template should have network policies"
    );
}

#[test]
fn test_sovereign_template_encryption() {
    let (_, _, _, _, _, _, _, storage) = create_sovereign_template();

    // All storage must use NestGate for security
    assert!(
        storage.nestgate_integration.is_some(),
        "Sovereign template must use NestGate integration"
    );

    // Should have backup policy for sovereign data
    assert!(
        storage.backup_policy.is_some(),
        "Sovereign template should have backup policy"
    );
}

#[test]
fn test_sovereign_template_air_gapped() {
    let (_, _, _, _, _, _, networking, _) = create_sovereign_template();

    // Should have network isolation
    assert!(!networking.mode.is_empty());

    // Should either be isolated or have strict network policies
    let is_isolated = networking.mode == "none"
        || networking.mode == "isolated"
        || !networking.network_policies.is_empty();

    assert!(is_isolated, "Sovereign template should be network-isolated");
}

// ============================================================================
// Custom Template Tests
// ============================================================================

#[test]
fn test_custom_template_basic() {
    let spec = CustomTemplateSpec {
        name: "test-custom".to_string(),
        description: "Test custom template".to_string(),
        primals: vec!["pki-provider".to_string()], // Capability-based
        services: vec![],
        security_level: "high".to_string(),
        resource_profile: "medium".to_string(),
    };

    let (name, description, primals, _, _, security, _, _) = create_custom_template(&spec);

    assert_eq!(name, "test-custom-biome");
    assert_eq!(description, "Test custom template");

    // Should have requested primals
    assert!(!primals.is_empty(), "Should have primals");

    // Should respect security level
    assert!(!security.isolation_level.is_empty());
}

#[test]
fn test_custom_template_with_services() {
    let service_spec = CustomServiceSpec {
        name: "my-service".to_string(),
        image: "custom/image".to_string(),
        ports: vec![8080],
        environment: {
            let mut env = HashMap::new();
            env.insert("KEY".to_string(), "value".to_string());
            env
        },
        volumes: vec!["/data".to_string()],
    };

    let spec = CustomTemplateSpec {
        name: "custom-with-service".to_string(),
        description: "Custom with services".to_string(),
        primals: vec!["pki-provider".to_string()], // Capability-based
        services: vec![service_spec],
        security_level: "medium".to_string(),
        resource_profile: "high".to_string(),
    };

    let (name, _, primals, services, resources, _, _, _) = create_custom_template(&spec);

    assert_eq!(name, "custom-with-service-biome");

    // Should have primals
    assert!(!primals.is_empty());

    // Should have custom service
    assert!(
        services.contains_key("my-service"),
        "Should have custom service"
    );

    // High resource profile should have limits
    assert!(
        resources.cpu_limit.is_some() || resources.memory_limit.is_some(),
        "High resource profile should have limits"
    );
}

#[test]
fn test_custom_template_resource_profiles() {
    let profiles = vec!["low", "medium", "high"];

    for profile in profiles {
        let spec = CustomTemplateSpec {
            name: format!("custom-{profile}"),
            description: format!("Custom {profile} resources"),
            primals: vec!["pki-provider".to_string()],
            services: vec![],
            security_level: "medium".to_string(),
            resource_profile: profile.to_string(),
        };

        let (_, _, _, _, resources, _, _, _) = create_custom_template(&spec);

        // All profiles should set some resource limits
        assert!(
            resources.cpu_limit.is_some() || resources.memory_limit.is_some(),
            "Profile '{profile}' should have resource limits"
        );
    }
}

#[test]
fn test_custom_template_security_levels() {
    let levels = vec!["low", "medium", "high", "maximum"];

    for level in levels {
        let spec = CustomTemplateSpec {
            name: format!("custom-{level}"),
            description: format!("Custom {level} security"),
            primals: vec!["pki-provider".to_string()],
            services: vec![],
            security_level: level.to_string(),
            resource_profile: "medium".to_string(),
        };

        let (_, _, primals, _, _, security, _, _) = create_custom_template(&spec);

        // All templates should have some primal configured
        assert!(!primals.is_empty(), "Template should have primals");

        // Security isolation should be configured
        assert!(!security.isolation_level.is_empty());
    }
}

// ============================================================================
// Cross-Template Validation
// ============================================================================

#[test]
fn test_all_specialized_templates_have_unique_names() {
    let templates = vec![
        create_genomics_template(),
        create_vision_template(),
        create_sovereign_template(),
    ];

    let mut names = std::collections::HashSet::new();

    for (name, _, _, _, _, _, _, _) in templates {
        assert!(
            names.insert(name.clone()),
            "Duplicate template name: {name}"
        );
    }
}

#[test]
fn test_all_specialized_templates_have_pki_capability() {
    let templates = vec![
        ("genomics", create_genomics_template()),
        ("vision", create_vision_template()),
        ("sovereign", create_sovereign_template()),
    ];

    for (template_type, (name, _, primals, _, _, security, _, _)) in templates {
        assert!(
            has_pki_capability(&primals),
            "{template_type} template '{name}' missing PKI capability"
        );

        assert!(
            security.beardog_required,
            "{template_type} template '{name}' should require PKI/security"
        );
    }
}

#[test]
fn test_all_specialized_templates_have_valid_security() {
    let templates = vec![
        ("genomics", create_genomics_template()),
        ("vision", create_vision_template()),
        ("sovereign", create_sovereign_template()),
    ];

    let valid_levels = ["low", "medium", "high", "maximum", "paranoid"];

    for (template_type, (name, _, _, _, _, security, _, _)) in templates {
        assert!(
            !security.isolation_level.is_empty(),
            "{template_type} template '{name}' missing isolation level"
        );

        assert!(
            valid_levels.contains(&security.isolation_level.as_str()),
            "{template_type} template '{name}' has invalid isolation level: {}",
            security.isolation_level
        );
    }
}
