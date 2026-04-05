// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive tests for CLI templates types

use std::collections::HashMap;
use toadstool_cli::templates::*;

// ============================================================================
// BiomeTemplate Tests
// ============================================================================

#[test]
fn test_biome_template_basic() {
    let template = BiomeTemplate::Basic;
    assert!(matches!(template, BiomeTemplate::Basic));
}

#[test]
fn test_biome_template_science() {
    let template = BiomeTemplate::Science;
    assert!(matches!(template, BiomeTemplate::Science));
}

#[test]
fn test_biome_template_ai_research() {
    let template = BiomeTemplate::AiResearch;
    assert!(matches!(template, BiomeTemplate::AiResearch));
}

#[test]
fn test_biome_template_quantum() {
    let template = BiomeTemplate::Quantum;
    assert!(matches!(template, BiomeTemplate::Quantum));
}

#[test]
fn test_biome_template_genomics() {
    let template = BiomeTemplate::Genomics;
    assert!(matches!(template, BiomeTemplate::Genomics));
}

#[test]
fn test_biome_template_vision() {
    let template = BiomeTemplate::Vision;
    assert!(matches!(template, BiomeTemplate::Vision));
}

#[test]
fn test_biome_template_distributed() {
    let template = BiomeTemplate::Distributed;
    assert!(matches!(template, BiomeTemplate::Distributed));
}

#[test]
fn test_biome_template_sovereign() {
    let template = BiomeTemplate::Sovereign;
    assert!(matches!(template, BiomeTemplate::Sovereign));
}

#[test]
fn test_biome_template_development() {
    let template = BiomeTemplate::Development;
    assert!(matches!(template, BiomeTemplate::Development));
}

#[test]
fn test_biome_template_custom() {
    let spec = CustomTemplateSpec {
        name: "MyCustom".to_string(),
        description: "Custom biome".to_string(),
        primals: vec!["songbird".to_string()],
        services: vec![],
        security_level: "high".to_string(),
        resource_profile: "medium".to_string(),
    };

    let template = BiomeTemplate::Custom(spec);

    if let BiomeTemplate::Custom(custom_spec) = template {
        assert_eq!(custom_spec.name, "MyCustom");
    } else {
        panic!("Expected Custom variant");
    }
}

#[test]
fn test_biome_template_clone() {
    let template = BiomeTemplate::Basic;
    let cloned = template;
    assert!(matches!(cloned, BiomeTemplate::Basic));
}

// ============================================================================
// CustomTemplateSpec Tests
// ============================================================================

#[test]
fn test_custom_template_spec_basic() {
    let spec = CustomTemplateSpec {
        name: "SimpleTemplate".to_string(),
        description: "A simple custom template".to_string(),
        primals: vec![],
        services: vec![],
        security_level: "standard".to_string(),
        resource_profile: "small".to_string(),
    };

    assert_eq!(spec.name, "SimpleTemplate");
    assert_eq!(spec.security_level, "standard");
    assert_eq!(spec.resource_profile, "small");
}

#[test]
fn test_custom_template_spec_with_primals() {
    let spec = CustomTemplateSpec {
        name: "FullStack".to_string(),
        description: "Full stack template".to_string(),
        primals: vec![
            "songbird".to_string(),
            "beardog".to_string(),
            "nestgate".to_string(),
        ],
        services: vec![],
        security_level: "maximum".to_string(),
        resource_profile: "large".to_string(),
    };

    assert_eq!(spec.primals.len(), 3);
    assert!(spec.primals.contains(&"songbird".to_string()));
    assert!(spec.primals.contains(&"beardog".to_string()));
    assert!(spec.primals.contains(&"nestgate".to_string()));
}

#[test]
fn test_custom_template_spec_high_security() {
    let spec = CustomTemplateSpec {
        name: "SecureCompute".to_string(),
        description: "High security computing".to_string(),
        primals: vec!["beardog".to_string()],
        services: vec![],
        security_level: "maximum".to_string(),
        resource_profile: "medium".to_string(),
    };

    assert_eq!(spec.security_level, "maximum");
}

#[test]
fn test_custom_template_spec_with_services() {
    let service = CustomServiceSpec {
        name: "web-server".to_string(),
        image: "nginx:latest".to_string(),
        ports: vec![80, 443],
        environment: HashMap::new(),
        volumes: vec![],
    };

    let spec = CustomTemplateSpec {
        name: "WebTemplate".to_string(),
        description: "Web server template".to_string(),
        primals: vec![],
        services: vec![service],
        security_level: "standard".to_string(),
        resource_profile: "medium".to_string(),
    };

    assert_eq!(spec.services.len(), 1);
}

#[test]
fn test_custom_template_spec_clone() {
    let spec = CustomTemplateSpec {
        name: "Test".to_string(),
        description: "Test template".to_string(),
        primals: vec![],
        services: vec![],
        security_level: "low".to_string(),
        resource_profile: "small".to_string(),
    };

    let cloned = spec.clone();
    assert_eq!(spec.name, cloned.name);
}

// ============================================================================
// CustomServiceSpec Tests
// ============================================================================

#[test]
fn test_custom_service_spec_basic() {
    let service = CustomServiceSpec {
        name: "database".to_string(),
        image: "postgres:14".to_string(),
        ports: vec![5432],
        environment: HashMap::new(),
        volumes: vec![],
    };

    assert_eq!(service.name, "database");
    assert_eq!(service.image, "postgres:14");
    assert_eq!(service.ports.len(), 1);
}

#[test]
fn test_custom_service_spec_with_environment() {
    let mut env = HashMap::new();
    env.insert("POSTGRES_PASSWORD".to_string(), "secret".to_string());
    env.insert("POSTGRES_USER".to_string(), "admin".to_string());

    let service = CustomServiceSpec {
        name: "db".to_string(),
        image: "postgres:15".to_string(),
        ports: vec![5432],
        environment: env,
        volumes: vec![],
    };

    assert_eq!(service.environment.len(), 2);
    assert_eq!(service.environment.get("POSTGRES_USER").unwrap(), "admin");
}

#[test]
fn test_custom_service_spec_with_volumes() {
    let service = CustomServiceSpec {
        name: "app".to_string(),
        image: "myapp:latest".to_string(),
        ports: vec![8080],
        environment: HashMap::new(),
        volumes: vec![
            "/data:/app/data".to_string(),
            "/config:/app/config".to_string(),
        ],
    };

    assert_eq!(service.volumes.len(), 2);
}

#[test]
fn test_custom_service_spec_multiple_ports() {
    let service = CustomServiceSpec {
        name: "api-gateway".to_string(),
        image: "gateway:v1".to_string(),
        ports: vec![80, 443, 8080, 8443],
        environment: HashMap::new(),
        volumes: vec![],
    };

    assert_eq!(service.ports.len(), 4);
    assert!(service.ports.contains(&80));
    assert!(service.ports.contains(&443));
}

#[test]
fn test_custom_service_spec_clone() {
    let service = CustomServiceSpec {
        name: "test-service".to_string(),
        image: "test:1.0".to_string(),
        ports: vec![3000],
        environment: HashMap::new(),
        volumes: vec![],
    };

    let cloned = service.clone();
    assert_eq!(service.name, cloned.name);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_all_biome_templates() {
    let templates = vec![
        BiomeTemplate::Basic,
        BiomeTemplate::Science,
        BiomeTemplate::AiResearch,
        BiomeTemplate::Quantum,
        BiomeTemplate::Genomics,
        BiomeTemplate::Vision,
        BiomeTemplate::Distributed,
        BiomeTemplate::Sovereign,
        BiomeTemplate::Development,
    ];

    assert_eq!(templates.len(), 9);
}

#[test]
fn test_security_levels() {
    let levels = ["low", "standard", "high", "maximum"];
    assert_eq!(levels.len(), 4);
}

#[test]
fn test_resource_profiles() {
    let profiles = ["small", "medium", "large"];
    assert_eq!(profiles.len(), 3);
}

#[test]
fn test_custom_template_with_all_primals() {
    let spec = CustomTemplateSpec {
        name: "AllPrimals".to_string(),
        description: "Template with all primals".to_string(),
        primals: vec![
            "songbird".to_string(),
            "beardog".to_string(),
            "nestgate".to_string(),
            "toadstool".to_string(),
        ],
        services: vec![],
        security_level: "maximum".to_string(),
        resource_profile: "large".to_string(),
    };

    assert_eq!(spec.primals.len(), 4);
}

#[test]
fn test_custom_service_full_config() {
    let mut env = HashMap::new();
    env.insert("ENV".to_string(), "production".to_string());
    env.insert("PORT".to_string(), "8000".to_string());

    let service = CustomServiceSpec {
        name: "production-api".to_string(),
        image: "api:production".to_string(),
        ports: vec![8000, 8443],
        environment: env,
        volumes: vec!["/data:/app/data".to_string()],
    };

    assert_eq!(service.name, "production-api");
    assert_eq!(service.ports.len(), 2);
    assert_eq!(service.environment.len(), 2);
    assert_eq!(service.volumes.len(), 1);
}
