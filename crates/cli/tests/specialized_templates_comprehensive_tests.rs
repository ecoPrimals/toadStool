// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for specialized templates
//!
//! Goal: Push specialized_templates.rs coverage from 1.18% to 50%+

use toadstool_cli::templates::specialized_templates::{
    create_ai_research_template, create_distributed_template, create_genomics_template,
    create_quantum_template, create_science_template, create_sovereign_template,
    create_vision_template,
};

// ============================================================================
// Science Template Tests
// ============================================================================

#[test]
fn test_science_template_name() {
    let (name, description, _, _, _, _, _, _) = create_science_template();

    assert_eq!(name, "science-biome");
    assert!(!description.is_empty());
    assert!(
        description.to_lowercase().contains("scientific")
            || description.to_lowercase().contains("science")
    );
}

#[test]
fn test_science_template_has_storage_capability() {
    let (_, _, primals, _, _, _, _, _) = create_science_template();

    // Science needs storage capability for data management
    assert!(primals.contains_key("capability:storage"));
    assert!(primals.get("capability:storage").unwrap().enabled);
}

#[test]
fn test_science_template_has_jupyter() {
    let (_, _, _, services, _, _, _, _) = create_science_template();

    assert!(services.contains_key("jupyter"));
    let jupyter = services.get("jupyter").unwrap();
    assert_eq!(jupyter.replicas, Some(1));
}

#[test]
fn test_science_template_jupyter_resources() {
    let (_, _, _, services, _, _, _, _) = create_science_template();

    let jupyter = services.get("jupyter").unwrap();
    assert_eq!(jupyter.resources.cpu_limit, Some(8.0));
    assert!(jupyter.resources.memory_limit.is_some());
}

#[test]
fn test_science_template_has_postgres() {
    let (_, _, _, services, _, _, _, _) = create_science_template();

    assert!(services.contains_key("postgres"));
}

#[test]
fn test_science_template_resources() {
    let (_, _, _, _, resources, _, _, _) = create_science_template();

    // Science template needs substantial resources
    assert!(resources.cpu_limit.unwrap_or(0.0) >= 16.0);
    assert!(resources.memory_limit.is_some());
}

#[test]
fn test_science_template_storage() {
    let (_, _, _, _, _, _, _, storage) = create_science_template();

    // Should have NestGate integration for data
    assert!(storage.nestgate_integration.is_some());
}

// ============================================================================
// AI Research Template Tests
// ============================================================================

#[test]
fn test_ai_template_name() {
    let (name, description, _, _, _, _, _, _) = create_ai_research_template();

    assert_eq!(name, "ai-research-biome");
    assert!(description.to_lowercase().contains("ai") || description.to_lowercase().contains("ml"));
}

#[test]
fn test_ai_template_has_pytorch() {
    let (_, _, _, services, _, _, _, _) = create_ai_research_template();

    assert!(services.contains_key("pytorch"));
}

#[test]
fn test_ai_template_has_tensorboard() {
    let (_, _, _, services, _, _, _, _) = create_ai_research_template();

    assert!(services.contains_key("tensorboard"));
}

#[test]
fn test_ai_template_gpu_resources() {
    let (_, _, _, _, resources, _, _, _) = create_ai_research_template();

    // AI needs GPUs
    assert!(resources.gpu_limit.is_some());
    assert!(resources.gpu_limit.unwrap() > 0);
}

#[test]
fn test_ai_template_high_memory() {
    let (_, _, _, _, resources, _, _, _) = create_ai_research_template();

    // AI needs lots of memory
    assert!(resources.memory_limit.is_some());
    assert!(resources.cpu_limit.unwrap_or(0.0) >= 16.0);
}

// ============================================================================
// Quantum Template Tests
// ============================================================================

#[test]
fn test_quantum_template_name() {
    let (name, description, _, _, _, _, _, _) = create_quantum_template();

    assert_eq!(name, "quantum-biome");
    assert!(description.to_lowercase().contains("quantum"));
}

#[test]
fn test_quantum_template_has_qiskit() {
    let (_, _, _, services, _, _, _, _) = create_quantum_template();

    assert!(services.contains_key("qiskit"));
}

#[test]
fn test_quantum_template_resources() {
    let (_, _, _, _, resources, _, _, _) = create_quantum_template();

    // Quantum needs substantial compute
    assert!(resources.cpu_limit.is_some());
    assert!(resources.memory_limit.is_some());
}

// ============================================================================
// Genomics Template Tests
// ============================================================================

#[test]
fn test_genomics_template_name() {
    let (name, description, _, _, _, _, _, _) = create_genomics_template();

    assert_eq!(name, "genomics-biome");
    assert!(
        description.to_lowercase().contains("genomics")
            || description.to_lowercase().contains("bioinformatics")
    );
}

#[test]
fn test_genomics_template_has_bioconductor() {
    let (_, _, _, services, _, _, _, _) = create_genomics_template();

    assert!(services.contains_key("bioconductor"));
}

#[test]
fn test_genomics_template_security() {
    let (_, _, _, _, _, security, _, _) = create_genomics_template();

    // Genomics handles sensitive data
    assert!(security.beardog_required);
    assert!(!security.isolation_level.is_empty());
}

#[test]
fn test_genomics_template_storage() {
    let (_, _, _, _, _, _, _, storage) = create_genomics_template();

    // Should have storage for datasets or NestGate
    assert!(storage.nestgate_integration.is_some() || !storage.datasets.is_empty());
}

// ============================================================================
// Vision Template Tests
// ============================================================================

#[test]
fn test_vision_template_name() {
    let (name, description, _, _, _, _, _, _) = create_vision_template();

    assert_eq!(name, "vision-biome");
    assert!(description.to_lowercase().contains("vision"));
}

#[test]
fn test_vision_template_has_opencv() {
    let (_, _, _, services, _, _, _, _) = create_vision_template();

    assert!(services.contains_key("opencv"));
}

#[test]
fn test_vision_template_gpu() {
    let (_, _, _, _, resources, _, _, _) = create_vision_template();

    // Vision processing needs GPU
    assert!(resources.gpu_limit.is_some());
}

// ============================================================================
// Distributed Template Tests
// ============================================================================

#[test]
fn test_distributed_template_name() {
    let (name, description, _, _, _, _, _, _) = create_distributed_template();

    assert_eq!(name, "distributed-biome");
    assert!(description.to_lowercase().contains("distributed"));
}

#[test]
fn test_distributed_template_has_discovery_capability() {
    let (_, _, primals, _, _, _, _, _) = create_distributed_template();

    assert!(primals.contains_key("capability:discovery"));
}

#[test]
fn test_distributed_template_has_storage_capability() {
    let (_, _, primals, _, _, _, _, _) = create_distributed_template();

    assert!(primals.contains_key("capability:storage"));
}

#[test]
fn test_distributed_template_has_workers() {
    let (_, _, _, services, _, _, _, _) = create_distributed_template();

    assert!(services.contains_key("worker"));

    let worker = services.get("worker").unwrap();
    // Multiple workers for distributed computing
    assert!(worker.replicas.is_some());
    assert!(worker.replicas.unwrap() > 1);
}

#[test]
fn test_distributed_template_resources() {
    let (_, _, _, _, resources, _, _, _) = create_distributed_template();

    // Distributed needs lots of resources
    assert!(resources.cpu_limit.unwrap_or(0.0) >= 100.0);
}

#[test]
fn test_distributed_template_networking() {
    let (_, _, _, _, _, _, networking, _) = create_distributed_template();

    // Should have mesh or cluster networking
    assert!(!networking.mode.is_empty());
}

#[test]
fn test_distributed_template_storage() {
    let (_, _, _, _, _, _, _, storage) = create_distributed_template();

    // Distributed systems need shared storage
    assert!(storage.nestgate_integration.is_some());
}

// ============================================================================
// Sovereign Template Tests
// ============================================================================

#[test]
fn test_sovereign_template_name() {
    let (name, description, _, _, _, _, _, _) = create_sovereign_template();

    assert_eq!(name, "sovereign-biome");
    assert!(
        description.to_lowercase().contains("sovereign")
            || description.to_lowercase().contains("security")
    );
}

#[test]
fn test_sovereign_template_maximum_security() {
    let (_, _, _, _, _, security, _, _) = create_sovereign_template();

    assert_eq!(security.isolation_level, "maximum");
    assert!(security.beardog_required);
}

#[test]
fn test_sovereign_template_crypto_policies() {
    let (_, _, _, _, _, security, _, _) = create_sovereign_template();

    assert!(!security.crypto_policies.is_empty());
    // Should have post-quantum crypto
    assert!(security
        .crypto_policies
        .iter()
        .any(|p| p.contains("quantum")));
}

#[test]
fn test_sovereign_template_air_gapped() {
    let (_, _, _, _, _, security, networking, _) = create_sovereign_template();

    // Air-gapped means no external networks
    assert_eq!(networking.mode, "none");
    assert!(security.allowed_networks.contains(&"none".to_string()));
}

#[test]
fn test_sovereign_template_forbidden_syscalls() {
    let (_, _, _, _, _, security, _, _) = create_sovereign_template();

    // Should restrict dangerous syscalls
    assert!(!security.forbidden_syscalls.is_empty());
}

#[test]
fn test_sovereign_template_secure_storage() {
    let (_, _, _, _, _, _, _, storage) = create_sovereign_template();

    // Must have NestGate for secure storage
    assert!(storage.nestgate_integration.is_some());
    // Must have backup policy
    assert!(storage.backup_policy.is_some());
}

// ============================================================================
// Cross-Template Validation Tests
// ============================================================================

#[test]
fn test_all_specialized_templates_have_beardog() {
    let templates = vec![
        create_science_template(),
        create_ai_research_template(),
        create_quantum_template(),
        create_genomics_template(),
        create_vision_template(),
        create_distributed_template(),
        create_sovereign_template(),
    ];

    for (_, _, primals, _, _, security, _, _) in templates {
        // EVOLVED: Check for capability-based PKI provider (not hardcoded "beardog")
        let has_pki = primals.keys().any(|k| {
            k == "pki-provider" || k == "beardog" || k.contains("pki") || k.contains("security")
        });
        assert!(has_pki, "Template must have PKI capability provider");
        assert!(
            security.beardog_required,
            "Template must require PKI/security"
        );
    }
}

#[test]
fn test_all_templates_have_unique_names() {
    let names = vec![
        create_science_template().0,
        create_ai_research_template().0,
        create_quantum_template().0,
        create_genomics_template().0,
        create_vision_template().0,
        create_distributed_template().0,
        create_sovereign_template().0,
    ];

    // Check all names are unique
    for i in 0..names.len() {
        for j in (i + 1)..names.len() {
            assert_ne!(names[i], names[j], "Template names must be unique");
        }
    }
}

#[test]
fn test_all_templates_have_descriptions() {
    let templates = vec![
        create_science_template(),
        create_ai_research_template(),
        create_quantum_template(),
        create_genomics_template(),
        create_vision_template(),
        create_distributed_template(),
        create_sovereign_template(),
    ];

    for (_, description, _, _, _, _, _, _) in templates {
        assert!(!description.is_empty(), "Template must have description");
        assert!(description.len() > 10, "Description should be meaningful");
    }
}

#[test]
fn test_all_templates_have_resources() {
    let templates = vec![
        create_science_template(),
        create_ai_research_template(),
        create_quantum_template(),
        create_genomics_template(),
        create_vision_template(),
        create_distributed_template(),
        create_sovereign_template(),
    ];

    for (_, _, _, _, resources, _, _, _) in templates {
        assert!(
            resources.cpu_limit.is_some(),
            "Template must specify CPU limit"
        );
        assert!(
            resources.memory_limit.is_some(),
            "Template must specify memory limit"
        );
    }
}

#[test]
fn test_all_templates_have_security_config() {
    let templates = vec![
        create_science_template(),
        create_ai_research_template(),
        create_quantum_template(),
        create_genomics_template(),
        create_vision_template(),
        create_distributed_template(),
        create_sovereign_template(),
    ];

    for (_, _, _, _, _, security, _, _) in templates {
        assert!(
            !security.isolation_level.is_empty(),
            "Must have isolation level"
        );
        assert!(!security.trust_level.is_empty(), "Must have trust level");
    }
}

#[test]
fn test_all_templates_have_networking() {
    let templates = vec![
        create_science_template(),
        create_ai_research_template(),
        create_quantum_template(),
        create_genomics_template(),
        create_vision_template(),
        create_distributed_template(),
        create_sovereign_template(),
    ];

    for (_, _, _, _, _, _, networking, _) in templates {
        assert!(!networking.mode.is_empty(), "Must have networking mode");
    }
}

#[test]
fn test_gpu_templates_specify_gpu_limit() {
    // AI and Vision templates should have GPU limits
    let ai_resources = create_ai_research_template().4;
    let vision_resources = create_vision_template().4;

    assert!(ai_resources.gpu_limit.is_some(), "AI template needs GPU");
    assert!(
        vision_resources.gpu_limit.is_some(),
        "Vision template needs GPU"
    );
}

#[test]
fn test_high_security_templates_have_policies() {
    let genomics_security = create_genomics_template().5;
    let sovereign_security = create_sovereign_template().5;

    // High-security templates need crypto policies
    assert!(!genomics_security.crypto_policies.is_empty());
    assert!(!sovereign_security.crypto_policies.is_empty());
}

#[test]
fn test_data_intensive_templates_have_storage() {
    let science_storage = create_science_template().7;
    let genomics_storage = create_genomics_template().7;
    let distributed_storage = create_distributed_template().7;

    // Data-intensive templates should have storage config
    assert!(science_storage.nestgate_integration.is_some() || !science_storage.datasets.is_empty());
    assert!(
        genomics_storage.nestgate_integration.is_some() || !genomics_storage.datasets.is_empty()
    );
    assert!(distributed_storage.nestgate_integration.is_some());
}
