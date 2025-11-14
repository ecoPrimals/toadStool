//! Basic and development template implementations
//!
//! This module contains the foundational biome templates:
//! - `create_basic_template()`: Minimal universal compute biome with BearDog
//! - `create_development_template()`: Development environment with enhanced tooling
//!
//! These templates form the basis for all specialized templates.

use std::collections::HashMap;
use toadstool_config::env_config::EnvironmentConfig;

use crate::{
    BiomeNetworking, BiomeResources, BiomeSecurity, BiomeStorage, HealthCheck, PrimalConfig,
    ServiceConfig, ServicePort, ServiceResources, WorkloadSource,
};

/// Template return type for consistency
pub type TemplateComponents = (
    String,                         // name
    String,                         // description
    HashMap<String, PrimalConfig>,  // primals
    HashMap<String, ServiceConfig>, // services
    BiomeResources,                 // resources
    BiomeSecurity,                  // security
    BiomeNetworking,                // networking
    BiomeStorage,                   // storage
);

/// Create basic template with essential services
pub fn create_basic_template() -> TemplateComponents {
    let name = "basic-biome".to_string();
    let description = "Basic universal compute biome with essential services".to_string();

    // Essential primals
    let mut primals = HashMap::new();
    primals.insert(
        "beardog".to_string(),
        PrimalConfig {
            version: "latest".to_string(),
            source: WorkloadSource::Container {
                registry: "registry.ecosystem.sovereignscience.org".to_string(),
                image: "beardog".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            enabled: true,
            config: HashMap::new(),
            dependencies: vec![],
            health_check: Some(HealthCheck {
                command: vec!["beardog".to_string(), "health".to_string()],
                interval: 30,
                timeout: 10,
                retries: 3,
                start_period: 60,
            }),
        },
    );

    // Basic services
    let mut services = HashMap::new();
    services.insert(
        "compute".to_string(),
        ServiceConfig {
            version: "latest".to_string(),
            source: WorkloadSource::Container {
                registry: "registry.ecosystem.sovereignscience.org".to_string(),
                image: "universal-compute".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: ServiceResources {
                cpu_limit: Some(2.0),
                memory_limit: Some("4GB".to_string()),
                storage_limit: Some("10GB".to_string()),
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
            dependencies: vec!["beardog".to_string()],
            health_check: Some({
                let config = EnvironmentConfig::from_env();
                HealthCheck {
                    command: vec![
                        "curl".to_string(),
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

    let resources = BiomeResources {
        cpu_limit: Some(4.0),
        memory_limit: Some("8GB".to_string()),
        storage_limit: Some("50GB".to_string()),
        gpu_limit: None,
        network_bandwidth: Some("1Gbps".to_string()),
    };

    let security = BiomeSecurity {
        isolation_level: "high".to_string(),
        trust_level: "verified".to_string(),
        beardog_required: true,
        crypto_policies: vec!["default".to_string()],
        allowed_networks: vec!["private".to_string()],
        forbidden_syscalls: vec!["mount".to_string(), "reboot".to_string()],
    };

    let networking = BiomeNetworking {
        mode: "bridge".to_string(),
        dns_servers: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
        port_mappings: vec![],
        network_policies: vec!["default-deny".to_string()],
    };

    let storage = BiomeStorage {
        nestgate_integration: false,
        datasets: vec![],
        volumes: vec![],
        backup_policy: None,
    };

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

/// Create development template with debugging tools
pub fn create_development_template() -> TemplateComponents {
    let name = "dev-biome".to_string();
    let description = "Development and testing environment with debugging tools".to_string();

    let (_, _, primals, mut services, resources, mut security, networking, storage) =
        create_basic_template();

    // Development tools
    services.insert(
        "vscode-server".to_string(),
        ServiceConfig {
            version: "latest".to_string(),
            source: WorkloadSource::Container {
                registry: "docker.io".to_string(),
                image: "codercom/code-server".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: ServiceResources {
                cpu_limit: Some(4.0),
                memory_limit: Some("8GB".to_string()),
                storage_limit: Some("50GB".to_string()),
            },
            environment: vec![("PASSWORD".to_string(), "${APP_PASSWORD:-}".to_string())]
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
            dependencies: vec!["beardog".to_string()],
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

    // Relaxed security for development
    security.isolation_level = "medium".to_string();
    security.trust_level = "development".to_string();

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
