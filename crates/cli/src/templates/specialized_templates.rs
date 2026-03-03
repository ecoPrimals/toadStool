// SPDX-License-Identifier: AGPL-3.0-or-later
//! Specialized biome templates (Science, AI, Quantum, Genomics, Vision, Distributed, Sovereign, Custom)
//!
//! This module contains all specialized template implementations for different
//! scientific computing and research workflows:
//!
//! - `create_science_template()`: Jupyter, PostgreSQL, data analysis tools
//! - `create_ai_research_template()`: PyTorch, TensorFlow, GPU acceleration
//! - `create_quantum_template()`: Qiskit, quantum computing simulators
//! - `create_genomics_template()`: Bioconductor, enhanced security for genomic data
//! - `create_vision_template()`: OpenCV, computer vision processing
//! - `create_distributed_template()`: Songbird orchestration, multi-node clusters
//! - `create_sovereign_template()`: Maximum security, air-gapped configuration
//! - `create_custom_template()`: User-specified custom configurations
//!
//! Extracted from `generator_impl.rs` (Nov 7, 2025) as part of the refactoring
//! to keep files under 1000 lines.
//!
//! ⚠️ **MIGRATION NOTICE**: Uses deprecated hardcoded ports during transition to capability-based discovery.

#![allow(deprecated)] // Module uses deprecated fields during migration

use std::collections::HashMap;
use toadstool_config::env_config::EnvironmentConfig;

use super::basic_templates::TemplateComponents;
use super::types_mod::CustomTemplateSpec;
use crate::{
    DatasetConfig, HealthCheck, PrimalConfig, ServiceConfig, ServicePort, ServiceResources,
    WorkloadSource,
};

/// Create science template with data analysis tools
pub fn create_science_template() -> TemplateComponents {
    use super::constants::{registries, service_names, template_names, versions};

    let name = template_names::SCIENCE.to_string();
    let description = "Scientific computing biome with Jupyter and data analysis tools".to_string();

    let (_, _, mut primals, mut services, mut resources, security, networking, mut storage) =
        super::basic_templates::create_basic_template();

    // Add NestGate for data management
    primals.insert(
        service_names::NESTGATE.to_string(),
        PrimalConfig {
            version: versions::LATEST.to_string(),
            source: WorkloadSource::Container {
                registry: registries::SOVEREIGN_SCIENCE.to_string(),
                image: service_names::NESTGATE.to_string(),
                tag: versions::LATEST.to_string(),
                digest: None,
            },
            enabled: true,
            config: HashMap::new(),
            dependencies: vec!["capability:pki".to_string()],
            health_check: Some(HealthCheck {
                command: vec!["nestgate".to_string(), "health".to_string()],
                interval: 30,
                timeout: 10,
                retries: 3,
                start_period: 60,
            }),
        },
    );

    // Jupyter notebook service
    services.insert(
        service_names::JUPYTER.to_string(),
        ServiceConfig {
            version: versions::LATEST.to_string(),
            source: WorkloadSource::Container {
                registry: registries::DOCKER_HUB.to_string(),
                image: super::constants::images::JUPYTER_SCIPY.to_string(),
                tag: versions::LATEST.to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: ServiceResources {
                cpu_limit: Some(8.0),
                memory_limit: Some(super::constants::resource_sizes::GB_16.to_string()),
                storage_limit: Some(super::constants::resource_sizes::GB_100.to_string()),
            },
            environment: vec![(
                super::constants::env_vars::JUPYTER_ENABLE_LAB.to_string(),
                "yes".to_string(),
            )]
            .into_iter()
            .collect(),
            ports: vec![{
                let config = EnvironmentConfig::from_env();
                ServicePort {
                    container_port: config.network.songbird_port,
                    host_port: Some(config.network.songbird_port),
                    protocol: "tcp".to_string(),
                }
            }],
            volumes: vec![],
            // Dependencies: Capability-based (orchestrator will resolve to available providers)
            // Modern orchestrators support capability discovery - if orchestrator is legacy,
            // it can map capabilities back to known service names
            dependencies: vec![
                "capability:pki".to_string(),     // PKI/cert management (e.g., beardog)
                "capability:storage".to_string(), // Persistent storage (e.g., nestgate)
            ],
            health_check: Some({
                let config = EnvironmentConfig::from_env();
                HealthCheck {
                    command: vec![
                        "curl".to_string(),
                        "-f".to_string(),
                        format!(
                            "http://{}:{}",
                            config.network.bind_address, config.network.songbird_port
                        ),
                    ],
                    interval: 30,
                    timeout: 10,
                    retries: 3,
                    start_period: 60,
                }
            }),
        },
    );

    // PostgreSQL for data storage
    services.insert(
        "postgres".to_string(),
        ServiceConfig {
            version: "15".to_string(),
            source: WorkloadSource::Container {
                registry: "docker.io".to_string(),
                image: "postgres".to_string(),
                tag: "15-alpine".to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: ServiceResources {
                cpu_limit: Some(4.0),
                memory_limit: Some("8GB".to_string()),
                storage_limit: Some("200GB".to_string()),
            },
            environment: vec![
                (
                    "POSTGRES_PASSWORD".to_string(),
                    "${DB_PASSWORD:-}".to_string(),
                ),
                ("POSTGRES_DB".to_string(), "research".to_string()),
            ]
            .into_iter()
            .collect(),
            ports: vec![ServicePort {
                container_port: 5432,
                host_port: Some(5432),
                protocol: "tcp".to_string(),
            }],
            volumes: vec![],
            dependencies: vec!["capability:pki".to_string()], // PKI capability (runtime discovery)
            health_check: Some(HealthCheck {
                command: vec![
                    "pg_isready".to_string(),
                    "-U".to_string(),
                    "postgres".to_string(),
                ],
                interval: 10,
                timeout: 5,
                retries: 5,
                start_period: 30,
            }),
        },
    );

    // Enhanced resources for scientific computing
    resources.cpu_limit = Some(16.0);
    resources.memory_limit = Some("32GB".to_string());
    resources.storage_limit = Some("500GB".to_string());

    // Enable NestGate storage integration
    storage.nestgate_integration = Some("latest".to_string());
    storage.datasets = vec![
        DatasetConfig {
            name: "research-data".to_string(),
            size: Some("500GB".to_string()),
            compression: Some("zstd".to_string()),
            encryption: true,
        },
        DatasetConfig {
            name: "reference-data".to_string(),
            size: Some("1TB".to_string()),
            compression: Some("zstd".to_string()),
            encryption: false,
        },
    ];

    (
        name,
        description,
        primals,
        services,
        resources,
        security,
        networking,
        storage,
    )
}

/// Create AI research template with GPU support
pub fn create_ai_research_template() -> TemplateComponents {
    use super::constants::{registries, service_names, template_names, versions};

    let name = template_names::AI_RESEARCH.to_string();
    let description =
        "AI/ML research biome with PyTorch, TensorFlow, and GPU acceleration".to_string();

    let (_, _, primals, mut services, mut resources, security, networking, mut storage) =
        create_science_template();

    // PyTorch environment
    services.insert(
        "pytorch".to_string(),
        ServiceConfig {
            version: versions::LATEST.to_string(),
            source: WorkloadSource::Container {
                registry: registries::DOCKER_HUB.to_string(),
                image: "pytorch/pytorch".to_string(),
                tag: versions::LATEST.to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: ServiceResources {
                cpu_limit: Some(16.0),
                memory_limit: Some(super::constants::resource_sizes::GB_64.to_string()),
                storage_limit: Some(super::constants::resource_sizes::GB_500.to_string()),
            },
            environment: HashMap::new(),
            ports: vec![{
                let config = EnvironmentConfig::from_env();
                ServicePort {
                    container_port: config.network.songbird_port,
                    host_port: Some(config.network.songbird_port),
                    protocol: "tcp".to_string(),
                }
            }],
            volumes: vec![],
            dependencies: vec![service_names::BEARDOG.to_string()],
            health_check: None,
        },
    );

    // TensorBoard for experiment tracking
    services.insert(
        "tensorboard".to_string(),
        ServiceConfig {
            version: "latest".to_string(),
            source: WorkloadSource::Container {
                registry: "docker.io".to_string(),
                image: "tensorflow/tensorflow".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: ServiceResources {
                cpu_limit: Some(4.0),
                memory_limit: Some("8GB".to_string()),
                storage_limit: Some("100GB".to_string()),
            },
            environment: HashMap::new(),
            ports: vec![ServicePort {
                container_port: 6006,
                host_port: Some(6006),
                protocol: "tcp".to_string(),
            }],
            volumes: vec![],
            dependencies: vec!["capability:pki".to_string()], // PKI capability (runtime discovery)
            health_check: None,
        },
    );

    // Massive resources for AI/ML
    resources.cpu_limit = Some(32.0);
    resources.memory_limit = Some("128GB".to_string());
    resources.storage_limit = Some("2TB".to_string());
    resources.gpu_limit = Some(4);

    // Model storage
    storage.datasets.push(DatasetConfig {
        name: "models".to_string(),
        size: Some("2TB".to_string()),
        compression: Some("zstd".to_string()),
        encryption: true,
    });

    (
        name,
        description,
        primals,
        services,
        resources,
        security,
        networking,
        storage,
    )
}

/// Create quantum computing template
pub fn create_quantum_template() -> TemplateComponents {
    use super::constants::{registries, service_names, template_names, versions};

    let name = template_names::QUANTUM.to_string();
    let description = "Quantum computing research with Qiskit and simulators".to_string();

    let (_, _, primals, mut services, mut resources, security, networking, storage) =
        create_science_template();

    // Qiskit environment
    services.insert(
        "qiskit".to_string(),
        ServiceConfig {
            version: versions::LATEST.to_string(),
            source: WorkloadSource::Container {
                registry: registries::DOCKER_HUB.to_string(),
                image: "qiskit/qiskit".to_string(),
                tag: versions::LATEST.to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: ServiceResources {
                cpu_limit: Some(32.0),
                memory_limit: Some("128GB".to_string()),
                storage_limit: Some(super::constants::resource_sizes::GB_500.to_string()),
            },
            environment: HashMap::new(),
            ports: vec![{
                let config = EnvironmentConfig::from_env();
                ServicePort {
                    container_port: config.network.songbird_port,
                    host_port: Some(config.network.songbird_port),
                    protocol: "tcp".to_string(),
                }
            }],
            volumes: vec![],
            dependencies: vec![service_names::BEARDOG.to_string()],
            health_check: None,
        },
    );

    // Extreme resources for quantum simulation
    resources.cpu_limit = Some(64.0);
    resources.memory_limit = Some("256GB".to_string());
    resources.storage_limit = Some(super::constants::resource_sizes::TB_1.to_string());

    (
        name,
        description,
        primals,
        services,
        resources,
        security,
        networking,
        storage,
    )
}

/// Create genomics/bioinformatics template
pub fn create_genomics_template() -> TemplateComponents {
    use super::constants::{registries, service_names, template_names, versions};

    let name = template_names::GENOMICS.to_string();
    let description = "Bioinformatics and genomics analysis with enhanced security".to_string();

    let (_, _, primals, mut services, mut resources, mut security, networking, mut storage) =
        create_science_template();

    // Bioconductor R environment
    services.insert(
        "bioconductor".to_string(),
        ServiceConfig {
            version: versions::LATEST.to_string(),
            source: WorkloadSource::Container {
                registry: registries::DOCKER_HUB.to_string(),
                image: "bioconductor/bioconductor_docker".to_string(),
                tag: versions::LATEST.to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: ServiceResources {
                cpu_limit: Some(16.0),
                memory_limit: Some(super::constants::resource_sizes::GB_64.to_string()),
                storage_limit: Some(super::constants::resource_sizes::TB_1.to_string()),
            },
            environment: HashMap::new(),
            ports: vec![{
                let config = EnvironmentConfig::from_env();
                ServicePort {
                    container_port: config.network.songbird_port,
                    host_port: Some(config.network.songbird_port),
                    protocol: "tcp".to_string(),
                }
            }],
            volumes: vec![],
            dependencies: vec![
                service_names::BEARDOG.to_string(),
                service_names::NESTGATE.to_string(),
            ],
            health_check: None,
        },
    );

    // Enhanced security for genomic data (HIPAA-compliant)
    security.isolation_level = "maximum".to_string();
    security.crypto_policies = vec!["post-quantum".to_string(), "aes-256-gcm".to_string()];

    // Large resources for genomics
    resources.cpu_limit = Some(32.0);
    resources.memory_limit = Some("128GB".to_string());
    resources.storage_limit = Some("2TB".to_string());

    // Encrypted datasets for genomic data
    storage.datasets = vec![
        DatasetConfig {
            name: "reference-genomes".to_string(),
            size: Some(super::constants::resource_sizes::GB_500.to_string()),
            compression: Some("zstd".to_string()),
            encryption: true,
        },
        DatasetConfig {
            name: "sequencing-data".to_string(),
            size: Some("10TB".to_string()),
            compression: Some("zstd".to_string()),
            encryption: true,
        },
    ];

    (
        name,
        description,
        primals,
        services,
        resources,
        security,
        networking,
        storage,
    )
}

/// Create computer vision template
pub fn create_vision_template() -> TemplateComponents {
    use super::constants::{registries, service_names, template_names, versions};

    let name = template_names::VISION.to_string();
    let description = "Computer vision and image processing with GPU acceleration".to_string();

    let (_, _, primals, mut services, mut resources, security, networking, storage) =
        create_science_template();

    // OpenCV environment with CUDA
    services.insert(
        "opencv".to_string(),
        ServiceConfig {
            version: versions::LATEST.to_string(),
            source: WorkloadSource::Container {
                registry: registries::DOCKER_HUB.to_string(),
                image: "opencv/opencv".to_string(),
                tag: versions::LATEST.to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: ServiceResources {
                cpu_limit: Some(8.0),
                memory_limit: Some(super::constants::resource_sizes::GB_32.to_string()),
                storage_limit: Some(super::constants::resource_sizes::GB_500.to_string()),
            },
            environment: HashMap::new(),
            ports: vec![{
                let config = EnvironmentConfig::from_env();
                ServicePort {
                    container_port: config.network.songbird_port,
                    host_port: Some(config.network.songbird_port),
                    protocol: "tcp".to_string(),
                }
            }],
            volumes: vec![],
            dependencies: vec![service_names::BEARDOG.to_string()],
            health_check: None,
        },
    );

    // GPU resources for vision processing
    resources.cpu_limit = Some(16.0);
    resources.memory_limit = Some(super::constants::resource_sizes::GB_64.to_string());
    resources.storage_limit = Some(super::constants::resource_sizes::TB_1.to_string());
    resources.gpu_limit = Some(2);

    (
        name,
        description,
        primals,
        services,
        resources,
        security,
        networking,
        storage,
    )
}

/// Create distributed computing cluster template
pub fn create_distributed_template() -> TemplateComponents {
    use super::constants::{registries, service_names, template_names, versions};

    let name = template_names::DISTRIBUTED.to_string();
    let description =
        "Multi-node distributed computing cluster with Songbird orchestration".to_string();

    let (_, _, mut primals, mut services, mut resources, security, mut networking, mut storage) =
        super::basic_templates::create_basic_template();

    // Add Songbird for orchestration
    primals.insert(
        service_names::SONGBIRD.to_string(),
        PrimalConfig {
            version: versions::LATEST.to_string(),
            source: WorkloadSource::Container {
                registry: registries::SOVEREIGN_SCIENCE.to_string(),
                image: service_names::SONGBIRD.to_string(),
                tag: versions::LATEST.to_string(),
                digest: None,
            },
            enabled: true,
            config: HashMap::new(),
            dependencies: vec![service_names::BEARDOG.to_string()],
            health_check: Some(HealthCheck {
                command: vec![
                    service_names::SONGBIRD.to_string(),
                    super::constants::commands::HEALTH.to_string(),
                ],
                interval: 30,
                timeout: 10,
                retries: 3,
                start_period: 60,
            }),
        },
    );

    // Add NestGate for distributed storage
    primals.insert(
        service_names::NESTGATE.to_string(),
        PrimalConfig {
            version: versions::LATEST.to_string(),
            source: WorkloadSource::Container {
                registry: registries::SOVEREIGN_SCIENCE.to_string(),
                image: service_names::NESTGATE.to_string(),
                tag: versions::LATEST.to_string(),
                digest: None,
            },
            enabled: true,
            config: HashMap::new(),
            dependencies: vec!["capability:pki".to_string()],
            health_check: Some(HealthCheck {
                command: vec!["nestgate".to_string(), "health".to_string()],
                interval: 30,
                timeout: 10,
                retries: 3,
                start_period: 60,
            }),
        },
    );

    // Worker nodes
    services.insert(
        "worker".to_string(),
        ServiceConfig {
            version: versions::LATEST.to_string(),
            source: WorkloadSource::Container {
                registry: registries::SOVEREIGN_SCIENCE.to_string(),
                image: "compute-worker".to_string(),
                tag: versions::LATEST.to_string(),
                digest: None,
            },
            replicas: Some(10), // Multiple worker instances
            resources: ServiceResources {
                cpu_limit: Some(8.0),
                memory_limit: Some(super::constants::resource_sizes::GB_16.to_string()),
                storage_limit: Some(super::constants::resource_sizes::GB_100.to_string()),
            },
            environment: HashMap::new(),
            ports: vec![],
            volumes: vec![],
            dependencies: vec![
                service_names::BEARDOG.to_string(),
                service_names::SONGBIRD.to_string(),
            ],
            health_check: Some({
                let config = EnvironmentConfig::from_env();
                HealthCheck {
                    command: vec![
                        super::constants::commands::CURL.to_string(),
                        "-f".to_string(),
                        format!(
                            "http://{}:{}/health",
                            config.network.bind_address, config.network.songbird_port
                        ),
                    ],
                    interval: 30,
                    timeout: 10,
                    retries: 3,
                    start_period: 30,
                }
            }),
        },
    );

    // Cluster resources (scalable)
    resources.cpu_limit = Some(128.0);
    resources.memory_limit = Some("1TB".to_string());
    resources.storage_limit = Some("50TB".to_string());

    // Cluster mesh networking
    networking.mode = "mesh".to_string();
    networking.network_policies = vec!["cluster-internal".to_string()];

    // Distributed storage
    storage.nestgate_integration = Some(versions::LATEST.to_string());

    (
        name,
        description,
        primals,
        services,
        resources,
        security,
        networking,
        storage,
    )
}

/// Create sovereign/air-gapped template
pub fn create_sovereign_template() -> TemplateComponents {
    use super::constants::{template_names, versions};

    let name = template_names::SOVEREIGN.to_string();
    let description =
        "Maximum security sovereign computing with air-gapped configuration".to_string();

    let (_, _, primals, services, mut resources, mut security, mut networking, mut storage) =
        super::basic_templates::create_basic_template();

    // Maximum security settings
    security.isolation_level = "maximum".to_string();
    security.trust_level = "sovereign".to_string();
    security.crypto_policies = vec![
        "post-quantum".to_string(),
        "aes-256-gcm".to_string(),
        "ed25519".to_string(),
    ];
    security.allowed_networks = vec!["none".to_string()]; // Air-gapped
    security.forbidden_syscalls = vec![
        "mount".to_string(),
        "reboot".to_string(),
        "network".to_string(),
    ];

    // Air-gapped networking (no external connections)
    networking.mode = "none".to_string();
    networking.dns_servers = vec![];
    networking.network_policies = vec!["deny-all".to_string()];

    // Moderate resources for security-focused workloads
    resources.cpu_limit = Some(8.0);
    resources.memory_limit = Some(super::constants::resource_sizes::GB_16.to_string());
    resources.storage_limit = Some(super::constants::resource_sizes::GB_100.to_string());

    // Secure storage with NestGate for maximum data protection
    storage.nestgate_integration = Some(versions::LATEST.to_string());
    storage.backup_policy = Some("encrypted-daily".to_string());

    (
        name,
        description,
        primals,
        services,
        resources,
        security,
        networking,
        storage,
    )
}

/// Create custom template from user specification
pub fn create_custom_template(spec: &CustomTemplateSpec) -> TemplateComponents {
    let name = format!("{}-biome", spec.name);
    let description = spec.description.clone();

    let (_, _, mut primals, mut services, mut resources, mut security, networking, storage) =
        super::basic_templates::create_basic_template();

    // Add requested primals
    for primal_name in &spec.primals {
        if !primals.contains_key(primal_name) {
            primals.insert(
                primal_name.clone(),
                PrimalConfig {
                    version: super::constants::versions::LATEST.to_string(),
                    source: WorkloadSource::Container {
                        registry: super::constants::registries::SOVEREIGN_SCIENCE.to_string(),
                        image: primal_name.clone(),
                        tag: super::constants::versions::LATEST.to_string(),
                        digest: None,
                    },
                    enabled: true,
                    config: HashMap::new(),
                    dependencies: vec![super::constants::service_names::BEARDOG.to_string()],
                    health_check: Some(HealthCheck {
                        command: vec![
                            primal_name.clone(),
                            super::constants::commands::HEALTH.to_string(),
                        ],
                        interval: 30,
                        timeout: 10,
                        retries: 3,
                        start_period: 60,
                    }),
                },
            );
        }
    }

    // Add custom services
    for service_spec in &spec.services {
        services.insert(
            service_spec.name.clone(),
            ServiceConfig {
                version: "latest".to_string(),
                source: WorkloadSource::Container {
                    registry: "docker.io".to_string(),
                    image: service_spec.image.clone(),
                    tag: "latest".to_string(),
                    digest: None,
                },
                replicas: Some(1),
                resources: ServiceResources {
                    cpu_limit: Some(4.0),
                    memory_limit: Some("8GB".to_string()),
                    storage_limit: Some("50GB".to_string()),
                },
                environment: service_spec.environment.clone(),
                ports: service_spec
                    .ports
                    .iter()
                    .map(|&port| ServicePort {
                        container_port: port,
                        host_port: Some(port),
                        protocol: "tcp".to_string(),
                    })
                    .collect(),
                volumes: vec![], // Custom volumes not yet supported in specs
                dependencies: vec!["capability:pki".to_string()], // PKI capability (runtime discovery)
                health_check: None,
            },
        );
    }

    // Apply resource profile
    match spec.resource_profile.as_str() {
        "low" => {
            resources.cpu_limit = Some(4.0);
            resources.memory_limit = Some("8GB".to_string());
            resources.storage_limit = Some("50GB".to_string());
        }
        "high" => {
            resources.cpu_limit = Some(32.0);
            resources.memory_limit = Some("128GB".to_string());
            resources.storage_limit = Some("2TB".to_string());
        }
        _ => {
            // Medium (default)
            resources.cpu_limit = Some(16.0);
            resources.memory_limit = Some("32GB".to_string());
            resources.storage_limit = Some("500GB".to_string());
        }
    }

    // Apply security level
    security.isolation_level = spec.security_level.clone();

    (
        name,
        description,
        primals,
        services,
        resources,
        security,
        networking,
        storage,
    )
}
