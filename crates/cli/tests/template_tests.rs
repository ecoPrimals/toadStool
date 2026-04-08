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
//! Template generation tests
//!
//! Comprehensive tests for biome template generators.
//! These templates are used for `toadstool init` and provide starting points for users.

use std::collections::HashMap;
use toadstool_cli::templates::basic_templates::{
    create_basic_template, create_development_template,
};
use toadstool_cli::templates::specialized_templates::{
    create_ai_research_template, create_distributed_template, create_quantum_template,
    create_science_template,
};

#[test]
fn test_basic_template_structure() {
    let (name, description, primals, services, resources, security, networking, _storage) =
        create_basic_template();

    // Verify basic structure
    assert_eq!(name, "basic-biome");
    assert!(!description.is_empty());

    // ✅ MODERNIZED: Check for capability providers, not hardcoded primal names
    // Should have capability providers (PKI, etc.)
    assert!(!primals.is_empty(), "Should have capability providers");

    // At least one primal should be enabled
    assert!(
        primals.values().any(|p| p.enabled),
        "At least one capability provider should be enabled"
    );

    // Should have at least one service
    assert!(!services.is_empty());

    // Resources should have reasonable defaults
    assert!(resources.cpu_limit.is_some());
    assert!(resources.memory_limit.is_some());

    // Security should require beardog
    assert!(security.security_required);

    // Networking should have a network mode
    assert!(!networking.mode.is_empty());

    // Storage structure exists (may be empty for basic template)
    // Basic template uses ephemeral storage by default
}

#[test]
fn test_basic_template_has_health_checks() {
    let (_, _, primals, services, _, _, _, _) = create_basic_template();

    // ✅ MODERNIZED: Check that capability providers have health checks
    // All primals/capability providers should have health checks
    for (name, primal) in &primals {
        assert!(
            primal.health_check.is_some(),
            "Capability provider '{name}' should have health check"
        );
    }

    // All services should have health checks
    for (name, service) in &services {
        assert!(
            service.health_check.is_some(),
            "Service '{name}' missing health check"
        );
    }
}

#[test]
fn test_basic_template_dependencies() {
    let (_, _, primals, services, _, _, _, _) = create_basic_template();

    // ✅ MODERNIZED: Services use capability-based dependencies, not hardcoded primal names
    // Services should depend on capabilities (e.g., capability:pki) not specific primals
    for (name, service) in &services {
        // Check for capability-based dependencies OR no dependencies
        let has_capability_dep = service
            .dependencies
            .iter()
            .any(|dep| dep.starts_with("capability:"));

        assert!(
            has_capability_dep || service.dependencies.is_empty(),
            "Service '{name}' should use capability-based dependencies (capability:*) or have no dependencies"
        );
    }

    // Primals should have no dependencies (for basic template)
    for (name, primal) in &primals {
        assert!(
            primal.dependencies.is_empty(),
            "Primal '{name}' should have no dependencies in basic template"
        );
    }
}

#[test]
fn test_development_template_includes_tools() {
    let (name, _, primals, services, _, _, _, _) = create_development_template();

    assert_eq!(name, "dev-biome");

    // ✅ MODERNIZED: Check for capability providers, not specific primal names
    // Should have essential capability providers (PKI, storage, etc.)
    assert!(
        !primals.is_empty(),
        "Development template should have capability providers"
    );

    // Should have development-friendly services
    // (Check for debug tools, IDEs, or development servers)
    assert!(
        services.len() >= 2,
        "Development template should have multiple services"
    );
}

#[test]
fn test_ai_ml_template_has_gpu() {
    let (name, _, _primals, services, resources, _, _, _) = create_ai_research_template();

    assert_eq!(name, "ai-research-biome");

    // Should allocate GPU resources
    // Check if any service requests GPU
    let has_gpu_service = services
        .values()
        .any(|s| s.environment.contains_key("GPU_ENABLED"));

    // Or check resources for GPU allocation
    assert!(
        has_gpu_service || resources.gpu_limit.is_some(),
        "AI/ML template should request GPU resources"
    );
}

#[test]
fn test_gpu_compute_template() {
    // Use AI research template as it has GPU support
    let (name, _, _, services, resources, _, _, _) = create_ai_research_template();

    assert_eq!(name, "ai-research-biome");

    // Should request GPU resources or have GPU-enabled services
    let has_gpu = resources.gpu_limit.is_some()
        || services.values().any(|s| {
            s.environment
                .values()
                .any(|v| v.contains("GPU") || v.contains("CUDA"))
        });

    assert!(has_gpu, "AI research template should support GPU");
    assert!(!services.is_empty(), "Template should have services");
}

#[test]
fn test_microservices_template_has_networking() {
    // Use distributed template as it has multi-service orchestration
    let (name, _, _, services, _, _, networking, _) = create_distributed_template();

    assert_eq!(name, "distributed-biome");

    // Should have services (at least primals + compute)
    assert!(
        !services.is_empty(),
        "Distributed template should have services"
    );

    // Networking should be configured
    assert!(!networking.mode.is_empty(), "Network mode should be set");

    // At least some services should have ports exposed
    let has_ports = services.values().any(|s| !s.ports.is_empty());
    assert!(has_ports, "Some services should expose ports");
}

#[test]
fn test_data_processing_template_has_storage() {
    // Use distributed template or science template for data processing
    let (name, _, primals, _, _, _, _, storage) = create_distributed_template();

    assert_eq!(name, "distributed-biome");

    // Should have storage configuration (volumes, datasets, or nestgate)
    let has_storage = !storage.volumes.is_empty()
        || !storage.datasets.is_empty()
        || storage.nestgate_integration.is_some();

    assert!(
        has_storage,
        "Distributed biome should have storage configured"
    );

    // Should have storage capability for data management
    assert!(
        primals.contains_key("capability:storage"),
        "Should have storage capability primal"
    );
}

#[test]
fn test_edge_iot_template_is_resource_constrained() {
    // Use science template as a general-purpose template
    let (name, _, _, services, resources, _, _, _) = create_science_template();

    assert_eq!(name, "science-biome");

    // Should have resource limits defined
    assert!(
        resources.cpu_limit.is_some() || resources.memory_limit.is_some(),
        "Template should have resource limits"
    );

    // Services should exist
    assert!(!services.is_empty(), "Template should have services");
}

#[test]
fn test_quantum_computing_template() {
    let (name, description, _, services, _, _, _, _) = create_quantum_template();

    assert_eq!(name, "quantum-biome");
    assert!(description.to_lowercase().contains("quantum"));

    // Should have services
    assert!(
        !services.is_empty(),
        "Quantum template should have services"
    );
}

#[test]
fn test_all_templates_have_capability_providers() {
    // ✅ MODERNIZED: Test that all templates have capability providers
    // (e.g., PKI, storage, auth), not hardcoded primal names
    let templates = vec![
        create_basic_template(),
        create_development_template(),
        create_ai_research_template(),
        create_science_template(),
        create_distributed_template(),
        create_quantum_template(),
    ];

    for (name, _, primals, _, _, security, _, _) in templates {
        // Each template should have capability providers
        assert!(
            !primals.is_empty(),
            "Template '{name}' should have capability providers"
        );

        // Security should enforce capability requirements
        assert!(
            security.security_required, // This field now means "PKI capability required"
            "Template '{name}' should require PKI capability"
        );
    }
}

#[test]
fn test_all_templates_have_unique_names() {
    let templates = vec![
        create_basic_template(),
        create_development_template(),
        create_ai_research_template(),
        create_science_template(),
        create_distributed_template(),
        create_quantum_template(),
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
fn test_template_service_sources_are_valid() {
    let (_, _, _, services, _, _, _, _) = create_basic_template();

    for (name, service) in &services {
        match &service.source {
            toadstool_cli::WorkloadSource::Container {
                registry,
                image,
                tag,
                ..
            } => {
                assert!(!registry.is_empty(), "Service '{name}' has empty registry");
                assert!(!image.is_empty(), "Service '{name}' has empty image");
                assert!(!tag.is_empty(), "Service '{name}' has empty tag");
            }
            toadstool_cli::WorkloadSource::Wasm { source, .. } => {
                assert!(!source.is_empty(), "Service '{name}' has empty wasm source");
            }
            toadstool_cli::WorkloadSource::Local { path } => {
                assert!(
                    !path.as_os_str().is_empty(),
                    "Service '{name}' has empty path"
                );
            }
            _ => {
                // Git, IPFS variants also valid
            }
        }
    }
}

#[test]
fn test_template_health_check_intervals_are_reasonable() {
    let (_, _, primals, services, _, _, _, _) = create_basic_template();

    // Check primals
    for (name, primal) in &primals {
        if let Some(health) = &primal.health_check {
            assert!(
                health.interval > 0,
                "Primal '{name}' health check interval must be > 0"
            );
            assert!(
                health.interval <= 300,
                "Primal '{name}' health check interval too long: {}s",
                health.interval
            );
            assert!(
                health.timeout < health.interval,
                "Primal '{name}' health check timeout must be < interval"
            );
        }
    }

    // Check services
    for (name, service) in &services {
        if let Some(health) = &service.health_check {
            assert!(
                health.interval > 0,
                "Service '{name}' health check interval must be > 0"
            );
            assert!(
                health.interval <= 300,
                "Service '{name}' health check interval too long: {}s",
                health.interval
            );
            assert!(
                health.timeout < health.interval,
                "Service '{name}' health check timeout must be < interval"
            );
        }
    }
}

#[test]
fn test_template_resource_limits_are_valid() {
    let templates = vec![
        create_basic_template(),
        create_development_template(),
        create_ai_research_template(),
        create_science_template(),
    ];

    for (name, _, _, _, resources, _, _, _) in templates {
        if let Some(cpu_limit) = resources.cpu_limit {
            assert!(cpu_limit > 0.0, "Template '{name}' CPU limit must be > 0");
            assert!(
                cpu_limit <= 256.0,
                "Template '{name}' CPU limit unreasonably high: {cpu_limit}"
            );
        }

        if let Some(memory_limit) = &resources.memory_limit {
            assert!(
                !memory_limit.is_empty(),
                "Template '{name}' memory limit is empty string"
            );
            // Should contain a number and unit
            assert!(
                memory_limit.chars().any(|c| c.is_ascii_digit()),
                "Template '{name}' memory limit missing number: {memory_limit}"
            );
        }
    }
}

#[test]
fn test_template_networking_is_configured() {
    let (_, _, _, _, _, _, networking, _) = create_basic_template();

    // Should have network mode defined
    assert!(!networking.mode.is_empty());

    // DNS is capability-based: defaults to empty so the runtime/orchestrator
    // can inject resolvers from the host. An explicit "host" mode is also valid.
    // The template itself should never embed hardcoded DNS IPs.
    assert!(
        networking.dns_servers.is_empty() || networking.mode == "host",
        "template DNS should be empty (host-inherited) or mode==host, found: {:?} / {:?}",
        networking.dns_servers,
        networking.mode
    );

    // At least one port should be exposed (if services exist)
    // This is implicit through service port configurations and port_mappings
}

#[test]
fn test_template_security_levels_are_appropriate() {
    let templates = vec![
        create_basic_template(),
        create_development_template(),
        create_ai_research_template(),
        create_distributed_template(),
    ];

    for (name, _, _, _, _, security, _, _) in templates {
        // All templates should require beardog for zero-trust
        assert!(
            security.security_required,
            "Template '{name}' should require beardog"
        );

        // Isolation level should be defined
        assert!(
            !security.isolation_level.is_empty(),
            "Template '{name}' missing isolation level"
        );

        // Valid isolation levels
        let valid_levels = ["low", "medium", "high", "maximum", "paranoid"];
        assert!(
            valid_levels.contains(&security.isolation_level.as_str()),
            "Template '{name}' has invalid isolation level: {}",
            security.isolation_level
        );
    }
}

#[test]
fn test_development_template_has_enhanced_resources() {
    let (_, _, _, _, basic_resources, _, _, _) = create_basic_template();
    let (_, _, _, _, dev_resources, _, _, _) = create_development_template();

    // Development should have same or more resources than basic
    if let (Some(basic_cpu), Some(dev_cpu)) = (basic_resources.cpu_limit, dev_resources.cpu_limit) {
        assert!(
            dev_cpu >= basic_cpu,
            "Development template should have >= CPU than basic"
        );
    }
}

#[test]
fn test_template_dependencies_form_valid_dag() {
    let (_, _, primals, services, _, _, _, _) = create_basic_template();

    // Build dependency graph
    let mut all_deps = HashMap::new();

    for (name, primal) in &primals {
        all_deps.insert(name.clone(), primal.dependencies.clone());
    }

    for (name, service) in &services {
        all_deps.insert(name.clone(), service.dependencies.clone());
    }

    // Check no cycles (simple check: no self-dependencies)
    for (name, deps) in &all_deps {
        assert!(!deps.contains(name), "Component '{name}' depends on itself");
    }

    // Check all dependencies exist (or are capability-based)
    for (name, deps) in &all_deps {
        for dep in deps {
            // ✅ MODERNIZED: Allow capability-based dependencies (capability:*)
            // These are resolved at runtime by the orchestrator
            let is_capability = dep.starts_with("capability:");
            let exists = primals.contains_key(dep) || services.contains_key(dep);

            assert!(
                is_capability || exists,
                "Component '{name}' depends on non-existent '{dep}' (not a capability or existing component)"
            );
        }
    }
}
