//! Biome Templates - Universal Compute Manifest Generation
//!
//! Templates for creating biome.yaml manifests for different scientific computing workflows.
//! Each template embodies the principles of SOVEREIGN SCIENCE and universal compute.

use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::info;

use crate::{
    BiomeManifest, BiomeMetadata, BiomeNetworking, BiomeResources, BiomeSecurity, BiomeStorage,
    DatasetConfig, HealthCheck, PrimalConfig, ServiceConfig, ServicePort, ServiceResources,
    WorkloadSource,
};

/// Available biome template types
#[derive(Debug, Clone)]
pub enum BiomeTemplate {
    /// Basic biome with essential services
    Basic,
    /// Scientific computing with data analysis
    Science,
    /// AI/ML training and inference
    AiResearch,
    /// Quantum computing research
    Quantum,
    /// Bioinformatics and genomics
    Genomics,
    /// Computer vision and imaging
    Vision,
    /// Distributed computing cluster
    Distributed,
    /// Security-focused sovereign computing
    Sovereign,
    /// Development and testing environment
    Development,
    /// Custom template from user specification
    Custom(CustomTemplateSpec),
}

/// Custom template specification
#[derive(Debug, Clone)]
pub struct CustomTemplateSpec {
    pub name: String,
    pub description: String,
    pub primals: Vec<String>,
    pub services: Vec<CustomServiceSpec>,
    pub security_level: String,
    pub resource_profile: String,
}

#[derive(Debug, Clone)]
pub struct CustomServiceSpec {
    pub name: String,
    pub image: String,
    pub ports: Vec<u16>,
    pub environment: HashMap<String, String>,
    pub volumes: Vec<String>,
}

/// Template generator for biome manifests
pub struct TemplateGenerator {
    output_dir: PathBuf,
    force_overwrite: bool,
}

impl TemplateGenerator {
    pub fn new(output_dir: PathBuf, force_overwrite: bool) -> Self {
        Self {
            output_dir,
            force_overwrite,
        }
    }

    /// Generate biome manifest from template
    pub async fn generate(&self, template: BiomeTemplate) -> Result<PathBuf> {
        let manifest = self.create_manifest(&template)?;
        let output_path = self.output_dir.join("biome.yaml");

        // Check if file exists and handle overwrite
        if output_path.exists() && !self.force_overwrite {
            return Err(anyhow::anyhow!(
                "biome.yaml already exists. Use --force to overwrite."
            ));
        }

        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Generate YAML content
        let yaml_content = self.manifest_to_yaml(&manifest)?;

        // Write to file
        fs::write(&output_path, yaml_content)
            .await
            .with_context(|| format!("Failed to write biome.yaml to {}", output_path.display()))?;

        info!("✅ Generated biome.yaml: {}", output_path.display());
        self.print_template_info(&template);

        Ok(output_path)
    }

    /// List available templates
    pub fn list_templates() -> Vec<(String, String)> {
        vec![
            (
                "basic".to_string(),
                "Essential services for general computing".to_string(),
            ),
            (
                "science".to_string(),
                "Scientific computing with data analysis tools".to_string(),
            ),
            (
                "ai-research".to_string(),
                "AI/ML training and inference environment".to_string(),
            ),
            (
                "quantum".to_string(),
                "Quantum computing research platform".to_string(),
            ),
            (
                "genomics".to_string(),
                "Bioinformatics and genomics analysis".to_string(),
            ),
            (
                "vision".to_string(),
                "Computer vision and imaging processing".to_string(),
            ),
            (
                "distributed".to_string(),
                "Multi-node distributed computing cluster".to_string(),
            ),
            (
                "sovereign".to_string(),
                "Maximum security sovereign computing".to_string(),
            ),
            (
                "development".to_string(),
                "Development and testing environment".to_string(),
            ),
        ]
    }

    /// Parse template type from string
    pub fn parse_template(template_str: &str) -> Result<BiomeTemplate> {
        match template_str.to_lowercase().as_str() {
            "basic" => Ok(BiomeTemplate::Basic),
            "science" => Ok(BiomeTemplate::Science),
            "ai-research" | "ai" | "ml" => Ok(BiomeTemplate::AiResearch),
            "quantum" => Ok(BiomeTemplate::Quantum),
            "genomics" | "bio" | "bioinformatics" => Ok(BiomeTemplate::Genomics),
            "vision" | "cv" | "imaging" => Ok(BiomeTemplate::Vision),
            "distributed" | "cluster" => Ok(BiomeTemplate::Distributed),
            "sovereign" | "security" => Ok(BiomeTemplate::Sovereign),
            "development" | "dev" | "test" => Ok(BiomeTemplate::Development),
            _ => Err(anyhow::anyhow!("Unknown template type: {}", template_str)),
        }
    }

    fn create_manifest(&self, template: &BiomeTemplate) -> Result<BiomeManifest> {
        let now = Utc::now();

        let (name, description, primals, services, resources, security, networking, storage) =
            match template {
                BiomeTemplate::Basic => self.create_basic_template(),
                BiomeTemplate::Science => self.create_science_template(),
                BiomeTemplate::AiResearch => self.create_ai_research_template(),
                BiomeTemplate::Quantum => self.create_quantum_template(),
                BiomeTemplate::Genomics => self.create_genomics_template(),
                BiomeTemplate::Vision => self.create_vision_template(),
                BiomeTemplate::Distributed => self.create_distributed_template(),
                BiomeTemplate::Sovereign => self.create_sovereign_template(),
                BiomeTemplate::Development => self.create_development_template(),
                BiomeTemplate::Custom(spec) => self.create_custom_template(spec),
            };

        Ok(BiomeManifest {
            metadata: BiomeMetadata {
                name,
                version: "1.0.0".to_string(),
                description: Some(description),
                author: Some("ToadStool Universal Compute".to_string()),
                created: now,
                updated: now,
                tags: self.get_template_tags(template),
            },
            primals,
            services,
            resources,
            security,
            networking,
            storage,
        })
    }

    fn create_basic_template(
        &self,
    ) -> (
        String,
        String,
        HashMap<String, PrimalConfig>,
        HashMap<String, ServiceConfig>,
        BiomeResources,
        BiomeSecurity,
        BiomeNetworking,
        BiomeStorage,
    ) {
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
                ports: vec![ServicePort {
                    container_port: 8080,
                    host_port: Some(8080),
                    protocol: "tcp".to_string(),
                }],
                volumes: vec![],
                dependencies: vec!["beardog".to_string()],
                health_check: Some(HealthCheck {
                    command: vec![
                        "curl".to_string(),
                        "-f".to_string(),
                        "http://localhost:8080/health".to_string(),
                    ],
                    interval: 30,
                    timeout: 10,
                    retries: 3,
                    start_period: 30,
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

    fn create_science_template(
        &self,
    ) -> (
        String,
        String,
        HashMap<String, PrimalConfig>,
        HashMap<String, ServiceConfig>,
        BiomeResources,
        BiomeSecurity,
        BiomeNetworking,
        BiomeStorage,
    ) {
        let name = "science-biome".to_string();
        let description =
            "Scientific computing environment with data analysis and visualization".to_string();

        let (_, _, primals, mut services, mut resources, security, networking, mut storage) =
            self.create_basic_template();

        // Add scientific computing services
        services.insert(
            "jupyter".to_string(),
            ServiceConfig {
                version: "latest".to_string(),
                source: WorkloadSource::Container {
                    registry: "docker.io".to_string(),
                    image: "jupyter/scipy-notebook".to_string(),
                    tag: "latest".to_string(),
                    digest: None,
                },
                replicas: Some(1),
                resources: ServiceResources {
                    cpu_limit: Some(4.0),
                    memory_limit: Some("8GB".to_string()),
                    storage_limit: Some("20GB".to_string()),
                },
                environment: vec![("JUPYTER_ENABLE_LAB".to_string(), "yes".to_string())]
                    .into_iter()
                    .collect(),
                ports: vec![ServicePort {
                    container_port: 8888,
                    host_port: Some(8888),
                    protocol: "tcp".to_string(),
                }],
                volumes: vec![],
                dependencies: vec!["beardog".to_string()],
                health_check: Some(HealthCheck {
                    command: vec![
                        "curl".to_string(),
                        "-f".to_string(),
                        "http://localhost:8888".to_string(),
                    ],
                    interval: 30,
                    timeout: 10,
                    retries: 3,
                    start_period: 60,
                }),
            },
        );

        services.insert(
            "postgresql".to_string(),
            ServiceConfig {
                version: "15".to_string(),
                source: WorkloadSource::Container {
                    registry: "docker.io".to_string(),
                    image: "postgres".to_string(),
                    tag: "15".to_string(),
                    digest: None,
                },
                replicas: Some(1),
                resources: ServiceResources {
                    cpu_limit: Some(2.0),
                    memory_limit: Some("4GB".to_string()),
                    storage_limit: Some("50GB".to_string()),
                },
                environment: vec![
                    ("POSTGRES_DB".to_string(), "science".to_string()),
                    ("POSTGRES_USER".to_string(), "scientist".to_string()),
                    ("POSTGRES_PASSWORD".to_string(), "changeme".to_string()),
                ]
                .into_iter()
                .collect(),
                ports: vec![ServicePort {
                    container_port: 5432,
                    host_port: Some(5432),
                    protocol: "tcp".to_string(),
                }],
                volumes: vec![],
                dependencies: vec!["beardog".to_string()],
                health_check: Some(HealthCheck {
                    command: vec![
                        "pg_isready".to_string(),
                        "-U".to_string(),
                        "scientist".to_string(),
                    ],
                    interval: 30,
                    timeout: 10,
                    retries: 3,
                    start_period: 30,
                }),
            },
        );

        // Enhanced resources for scientific computing
        resources.cpu_limit = Some(16.0);
        resources.memory_limit = Some("32GB".to_string());
        resources.storage_limit = Some("500GB".to_string());

        // NestGate integration for data management
        storage.nestgate_integration = true;
        storage.datasets.push(DatasetConfig {
            name: "research-data".to_string(),
            size: Some("100GB".to_string()),
            compression: Some("lz4".to_string()),
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

    fn create_ai_research_template(
        &self,
    ) -> (
        String,
        String,
        HashMap<String, PrimalConfig>,
        HashMap<String, ServiceConfig>,
        BiomeResources,
        BiomeSecurity,
        BiomeNetworking,
        BiomeStorage,
    ) {
        let name = "ai-research-biome".to_string();
        let description =
            "AI/ML research environment with GPU acceleration and model training".to_string();

        let (_, _, primals, mut services, mut resources, security, networking, mut storage) =
            self.create_science_template();

        // Add AI/ML services
        services.insert(
            "pytorch".to_string(),
            ServiceConfig {
                version: "latest".to_string(),
                source: WorkloadSource::Container {
                    registry: "docker.io".to_string(),
                    image: "pytorch/pytorch".to_string(),
                    tag: "latest".to_string(),
                    digest: None,
                },
                replicas: Some(1),
                resources: ServiceResources {
                    cpu_limit: Some(8.0),
                    memory_limit: Some("16GB".to_string()),
                    storage_limit: Some("100GB".to_string()),
                },
                environment: vec![("CUDA_VISIBLE_DEVICES".to_string(), "all".to_string())]
                    .into_iter()
                    .collect(),
                ports: vec![ServicePort {
                    container_port: 8000,
                    host_port: Some(8000),
                    protocol: "tcp".to_string(),
                }],
                volumes: vec![],
                dependencies: vec!["beardog".to_string()],
                health_check: Some(HealthCheck {
                    command: vec![
                        "python".to_string(),
                        "-c".to_string(),
                        "import torch; print(torch.cuda.is_available())".to_string(),
                    ],
                    interval: 60,
                    timeout: 30,
                    retries: 3,
                    start_period: 120,
                }),
            },
        );

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
                    cpu_limit: Some(2.0),
                    memory_limit: Some("4GB".to_string()),
                    storage_limit: Some("20GB".to_string()),
                },
                environment: HashMap::new(),
                ports: vec![ServicePort {
                    container_port: 6006,
                    host_port: Some(6006),
                    protocol: "tcp".to_string(),
                }],
                volumes: vec![],
                dependencies: vec!["beardog".to_string()],
                health_check: Some(HealthCheck {
                    command: vec![
                        "curl".to_string(),
                        "-f".to_string(),
                        "http://localhost:6006".to_string(),
                    ],
                    interval: 30,
                    timeout: 10,
                    retries: 3,
                    start_period: 60,
                }),
            },
        );

        // GPU resources
        resources.cpu_limit = Some(32.0);
        resources.memory_limit = Some("128GB".to_string());
        resources.gpu_limit = Some(4);
        resources.storage_limit = Some("2TB".to_string());

        // AI datasets
        storage.datasets.push(DatasetConfig {
            name: "models".to_string(),
            size: Some("500GB".to_string()),
            compression: Some("lz4".to_string()),
            encryption: true,
        });

        storage.datasets.push(DatasetConfig {
            name: "training-data".to_string(),
            size: Some("1TB".to_string()),
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

    fn create_quantum_template(
        &self,
    ) -> (
        String,
        String,
        HashMap<String, PrimalConfig>,
        HashMap<String, ServiceConfig>,
        BiomeResources,
        BiomeSecurity,
        BiomeNetworking,
        BiomeStorage,
    ) {
        let name = "quantum-biome".to_string();
        let description =
            "Quantum computing research environment with simulators and hardware access"
                .to_string();

        let (_, _, primals, mut services, mut resources, security, networking, storage) =
            self.create_basic_template();

        // Quantum computing services
        services.insert(
            "qiskit".to_string(),
            ServiceConfig {
                version: "latest".to_string(),
                source: WorkloadSource::Container {
                    registry: "registry.ecosystem.sovereignscience.org".to_string(),
                    image: "quantum/qiskit".to_string(),
                    tag: "latest".to_string(),
                    digest: None,
                },
                replicas: Some(1),
                resources: ServiceResources {
                    cpu_limit: Some(16.0),
                    memory_limit: Some("32GB".to_string()),
                    storage_limit: Some("100GB".to_string()),
                },
                environment: vec![("QISKIT_BACKEND".to_string(), "aer_simulator".to_string())]
                    .into_iter()
                    .collect(),
                ports: vec![ServicePort {
                    container_port: 8888,
                    host_port: Some(8888),
                    protocol: "tcp".to_string(),
                }],
                volumes: vec![],
                dependencies: vec!["beardog".to_string()],
                health_check: Some(HealthCheck {
                    command: vec![
                        "python".to_string(),
                        "-c".to_string(),
                        "import qiskit; print('Qiskit ready')".to_string(),
                    ],
                    interval: 60,
                    timeout: 30,
                    retries: 3,
                    start_period: 120,
                }),
            },
        );

        // Enhanced resources for quantum simulation
        resources.cpu_limit = Some(64.0);
        resources.memory_limit = Some("256GB".to_string());
        resources.storage_limit = Some("1TB".to_string());

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

    fn create_genomics_template(
        &self,
    ) -> (
        String,
        String,
        HashMap<String, PrimalConfig>,
        HashMap<String, ServiceConfig>,
        BiomeResources,
        BiomeSecurity,
        BiomeNetworking,
        BiomeStorage,
    ) {
        let name = "genomics-biome".to_string();
        let description =
            "Bioinformatics and genomics analysis environment with secure data handling"
                .to_string();

        let (_, _, primals, mut services, resources, mut security, networking, mut storage) =
            self.create_science_template();

        // Genomics services
        services.insert(
            "bioconductor".to_string(),
            ServiceConfig {
                version: "latest".to_string(),
                source: WorkloadSource::Container {
                    registry: "docker.io".to_string(),
                    image: "bioconductor/bioconductor_docker".to_string(),
                    tag: "latest".to_string(),
                    digest: None,
                },
                replicas: Some(1),
                resources: ServiceResources {
                    cpu_limit: Some(16.0),
                    memory_limit: Some("64GB".to_string()),
                    storage_limit: Some("200GB".to_string()),
                },
                environment: HashMap::new(),
                ports: vec![ServicePort {
                    container_port: 8787,
                    host_port: Some(8787),
                    protocol: "tcp".to_string(),
                }],
                volumes: vec![],
                dependencies: vec!["beardog".to_string()],
                health_check: Some(HealthCheck {
                    command: vec![
                        "curl".to_string(),
                        "-f".to_string(),
                        "http://localhost:8787".to_string(),
                    ],
                    interval: 30,
                    timeout: 10,
                    retries: 3,
                    start_period: 120,
                }),
            },
        );

        // Enhanced security for genomic data
        security.crypto_policies.push("genomics-hipaa".to_string());
        security
            .forbidden_syscalls
            .extend(vec!["ptrace".to_string(), "process_vm_readv".to_string()]);

        // Genomics datasets
        storage.datasets.push(DatasetConfig {
            name: "reference-genomes".to_string(),
            size: Some("100GB".to_string()),
            compression: Some("zstd".to_string()),
            encryption: true,
        });

        storage.datasets.push(DatasetConfig {
            name: "sequencing-data".to_string(),
            size: Some("5TB".to_string()),
            compression: Some("lz4".to_string()),
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

    fn create_vision_template(
        &self,
    ) -> (
        String,
        String,
        HashMap<String, PrimalConfig>,
        HashMap<String, ServiceConfig>,
        BiomeResources,
        BiomeSecurity,
        BiomeNetworking,
        BiomeStorage,
    ) {
        let name = "vision-biome".to_string();
        let description = "Computer vision and imaging processing environment".to_string();

        let (_, _, primals, mut services, resources, security, networking, mut storage) =
            self.create_ai_research_template();

        // Computer vision services
        services.insert(
            "opencv".to_string(),
            ServiceConfig {
                version: "latest".to_string(),
                source: WorkloadSource::Container {
                    registry: "registry.ecosystem.sovereignscience.org".to_string(),
                    image: "vision/opencv-cuda".to_string(),
                    tag: "latest".to_string(),
                    digest: None,
                },
                replicas: Some(1),
                resources: ServiceResources {
                    cpu_limit: Some(8.0),
                    memory_limit: Some("16GB".to_string()),
                    storage_limit: Some("100GB".to_string()),
                },
                environment: vec![("CUDA_VISIBLE_DEVICES".to_string(), "all".to_string())]
                    .into_iter()
                    .collect(),
                ports: vec![ServicePort {
                    container_port: 8080,
                    host_port: Some(8080),
                    protocol: "tcp".to_string(),
                }],
                volumes: vec![],
                dependencies: vec!["beardog".to_string()],
                health_check: Some(HealthCheck {
                    command: vec![
                        "python".to_string(),
                        "-c".to_string(),
                        "import cv2; print('OpenCV ready')".to_string(),
                    ],
                    interval: 60,
                    timeout: 30,
                    retries: 3,
                    start_period: 120,
                }),
            },
        );

        // Vision datasets
        storage.datasets.push(DatasetConfig {
            name: "images".to_string(),
            size: Some("1TB".to_string()),
            compression: Some("zstd".to_string()),
            encryption: false, // Images may not need encryption
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

    fn create_distributed_template(
        &self,
    ) -> (
        String,
        String,
        HashMap<String, PrimalConfig>,
        HashMap<String, ServiceConfig>,
        BiomeResources,
        BiomeSecurity,
        BiomeNetworking,
        BiomeStorage,
    ) {
        let name = "distributed-biome".to_string();
        let description =
            "Multi-node distributed computing cluster with Songbird coordination".to_string();

        let (_, _, mut primals, services, mut resources, security, mut networking, mut storage) =
            self.create_basic_template();

        // Add Songbird for coordination
        primals.insert(
            "songbird".to_string(),
            PrimalConfig {
                version: "latest".to_string(),
                source: WorkloadSource::Container {
                    registry: "registry.ecosystem.sovereignscience.org".to_string(),
                    image: "songbird".to_string(),
                    tag: "latest".to_string(),
                    digest: None,
                },
                enabled: true,
                config: HashMap::new(),
                dependencies: vec!["beardog".to_string()],
                health_check: Some(HealthCheck {
                    command: vec!["songbird".to_string(), "health".to_string()],
                    interval: 30,
                    timeout: 10,
                    retries: 3,
                    start_period: 60,
                }),
            },
        );

        // Add NestGate for distributed storage
        primals.insert(
            "nestgate".to_string(),
            PrimalConfig {
                version: "latest".to_string(),
                source: WorkloadSource::Container {
                    registry: "registry.ecosystem.sovereignscience.org".to_string(),
                    image: "nestgate".to_string(),
                    tag: "latest".to_string(),
                    digest: None,
                },
                enabled: true,
                config: HashMap::new(),
                dependencies: vec!["beardog".to_string()],
                health_check: Some(HealthCheck {
                    command: vec!["nestgate".to_string(), "health".to_string()],
                    interval: 30,
                    timeout: 10,
                    retries: 3,
                    start_period: 60,
                }),
            },
        );

        // Cluster networking
        networking.mode = "cluster".to_string();
        networking.network_policies.push("cluster-mesh".to_string());

        // Distributed storage
        storage.nestgate_integration = true;
        storage.datasets.push(DatasetConfig {
            name: "shared-compute".to_string(),
            size: Some("10TB".to_string()),
            compression: Some("lz4".to_string()),
            encryption: true,
        });

        // Enhanced resources for cluster
        resources.cpu_limit = Some(128.0);
        resources.memory_limit = Some("1TB".to_string());
        resources.storage_limit = Some("50TB".to_string());
        resources.network_bandwidth = Some("100Gbps".to_string());

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

    fn create_sovereign_template(
        &self,
    ) -> (
        String,
        String,
        HashMap<String, PrimalConfig>,
        HashMap<String, ServiceConfig>,
        BiomeResources,
        BiomeSecurity,
        BiomeNetworking,
        BiomeStorage,
    ) {
        let name = "sovereign-biome".to_string();
        let description =
            "Maximum security sovereign computing environment with zero external dependencies"
                .to_string();

        let (_, _, primals, services, resources, mut security, mut networking, mut storage) =
            self.create_basic_template();

        // Maximum security configuration
        security.isolation_level = "maximum".to_string();
        security.trust_level = "sovereign".to_string();
        security.crypto_policies.extend(vec![
            "zero-trust".to_string(),
            "post-quantum".to_string(),
            "air-gapped".to_string(),
        ]);
        security.allowed_networks = vec!["none".to_string()];
        security.forbidden_syscalls.extend(vec![
            "ptrace".to_string(),
            "process_vm_readv".to_string(),
            "process_vm_writev".to_string(),
            "keyctl".to_string(),
            "add_key".to_string(),
            "request_key".to_string(),
        ]);

        // Air-gapped networking
        networking.mode = "none".to_string();
        networking.dns_servers = vec![];
        networking.network_policies = vec!["deny-all".to_string()];

        // Encrypted storage only
        storage.nestgate_integration = true;
        storage.datasets.push(DatasetConfig {
            name: "sovereign-data".to_string(),
            size: Some("1TB".to_string()),
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

    fn create_development_template(
        &self,
    ) -> (
        String,
        String,
        HashMap<String, PrimalConfig>,
        HashMap<String, ServiceConfig>,
        BiomeResources,
        BiomeSecurity,
        BiomeNetworking,
        BiomeStorage,
    ) {
        let name = "dev-biome".to_string();
        let description = "Development and testing environment with debugging tools".to_string();

        let (_, _, primals, mut services, resources, mut security, networking, storage) =
            self.create_basic_template();

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
                environment: vec![("PASSWORD".to_string(), "changeme".to_string())]
                    .into_iter()
                    .collect(),
                ports: vec![ServicePort {
                    container_port: 8080,
                    host_port: Some(8080),
                    protocol: "tcp".to_string(),
                }],
                volumes: vec![],
                dependencies: vec!["beardog".to_string()],
                health_check: Some(HealthCheck {
                    command: vec![
                        "curl".to_string(),
                        "-f".to_string(),
                        "http://localhost:8080".to_string(),
                    ],
                    interval: 30,
                    timeout: 10,
                    retries: 3,
                    start_period: 60,
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

    fn create_custom_template(
        &self,
        spec: &CustomTemplateSpec,
    ) -> (
        String,
        String,
        HashMap<String, PrimalConfig>,
        HashMap<String, ServiceConfig>,
        BiomeResources,
        BiomeSecurity,
        BiomeNetworking,
        BiomeStorage,
    ) {
        let name = spec.name.clone();
        let description = spec.description.clone();

        let (_, _, primals, mut services, resources, mut security, networking, storage) =
            self.create_basic_template();

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
                        cpu_limit: Some(2.0),
                        memory_limit: Some("4GB".to_string()),
                        storage_limit: Some("20GB".to_string()),
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
                    volumes: vec![],
                    dependencies: vec!["beardog".to_string()],
                    health_check: None,
                },
            );
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

    fn get_template_tags(&self, template: &BiomeTemplate) -> Vec<String> {
        match template {
            BiomeTemplate::Basic => vec!["basic".to_string(), "essential".to_string()],
            BiomeTemplate::Science => vec![
                "science".to_string(),
                "research".to_string(),
                "data-analysis".to_string(),
            ],
            BiomeTemplate::AiResearch => vec![
                "ai".to_string(),
                "ml".to_string(),
                "gpu".to_string(),
                "training".to_string(),
            ],
            BiomeTemplate::Quantum => vec![
                "quantum".to_string(),
                "simulation".to_string(),
                "research".to_string(),
            ],
            BiomeTemplate::Genomics => vec![
                "genomics".to_string(),
                "bioinformatics".to_string(),
                "secure".to_string(),
            ],
            BiomeTemplate::Vision => vec![
                "vision".to_string(),
                "imaging".to_string(),
                "opencv".to_string(),
                "gpu".to_string(),
            ],
            BiomeTemplate::Distributed => vec![
                "distributed".to_string(),
                "cluster".to_string(),
                "songbird".to_string(),
            ],
            BiomeTemplate::Sovereign => vec![
                "sovereign".to_string(),
                "security".to_string(),
                "air-gapped".to_string(),
            ],
            BiomeTemplate::Development => vec![
                "development".to_string(),
                "testing".to_string(),
                "tools".to_string(),
            ],
            BiomeTemplate::Custom(_) => vec!["custom".to_string()],
        }
    }

    fn manifest_to_yaml(&self, manifest: &BiomeManifest) -> Result<String> {
        let mut yaml = String::new();

        yaml.push_str(&format!(
            "# {}\n",
            manifest
                .metadata
                .description
                .as_ref()
                .unwrap_or(&"ToadStool Biome Manifest".to_string())
        ));
        yaml.push_str("# Generated by ToadStool Universal Compute Platform\n");
        yaml.push_str("# https://github.com/your-org/toadstool\n\n");

        let yaml_content =
            serde_yaml::to_string(manifest).context("Failed to serialize manifest to YAML")?;

        yaml.push_str(&yaml_content);

        Ok(yaml)
    }

    fn print_template_info(&self, template: &BiomeTemplate) {
        match template {
            BiomeTemplate::Basic => {
                info!("📦 Basic biome template generated");
                info!("   • Essential services for general computing");
                info!("   • BearDog security by default");
                info!("   • Resource limits: 4 CPU, 8GB RAM, 50GB storage");
            }
            BiomeTemplate::Science => {
                info!("🔬 Science biome template generated");
                info!("   • Jupyter notebook for interactive analysis");
                info!("   • PostgreSQL database for data storage");
                info!("   • NestGate integration for research data");
                info!("   • Resource limits: 16 CPU, 32GB RAM, 500GB storage");
            }
            BiomeTemplate::AiResearch => {
                info!("🤖 AI Research biome template generated");
                info!("   • PyTorch and TensorFlow environments");
                info!("   • TensorBoard for experiment tracking");
                info!("   • GPU acceleration support");
                info!("   • Resource limits: 32 CPU, 128GB RAM, 2TB storage, 4 GPUs");
            }
            BiomeTemplate::Quantum => {
                info!("⚛️  Quantum Computing biome template generated");
                info!("   • Qiskit quantum development environment");
                info!("   • Quantum simulators and hardware access");
                info!("   • Resource limits: 64 CPU, 256GB RAM, 1TB storage");
            }
            BiomeTemplate::Genomics => {
                info!("🧬 Genomics biome template generated");
                info!("   • Bioconductor R environment");
                info!("   • Enhanced security for genomic data");
                info!("   • HIPAA-compliant crypto policies");
                info!("   • Encrypted datasets for reference genomes and sequencing data");
            }
            BiomeTemplate::Vision => {
                info!("👁️  Computer Vision biome template generated");
                info!("   • OpenCV with CUDA acceleration");
                info!("   • Image processing and analysis tools");
                info!("   • GPU support for deep learning models");
            }
            BiomeTemplate::Distributed => {
                info!("🌐 Distributed Computing biome template generated");
                info!("   • Songbird coordination service");
                info!("   • NestGate distributed storage");
                info!("   • Cluster mesh networking");
                info!("   • Resource limits: 128 CPU, 1TB RAM, 50TB storage");
            }
            BiomeTemplate::Sovereign => {
                info!("🔒 Sovereign Computing biome template generated");
                info!("   • Maximum security isolation");
                info!("   • Air-gapped networking (no external connections)");
                info!("   • Post-quantum cryptography");
                info!("   • Zero external dependencies");
            }
            BiomeTemplate::Development => {
                info!("🛠️  Development biome template generated");
                info!("   • VS Code server for remote development");
                info!("   • Debugging and testing tools");
                info!("   • Relaxed security for development workflow");
            }
            BiomeTemplate::Custom(spec) => {
                info!("⚙️  Custom biome template generated: {}", spec.name);
                info!("   • {} services configured", spec.services.len());
                info!("   • Security level: {}", spec.security_level);
            }
        }

        info!("");
        info!("🚀 Next steps:");
        info!("   1. Review and customize the generated biome.yaml");
        info!("   2. Run: toadstool validate biome.yaml");
        info!("   3. Start: toadstool run biome.yaml");
        info!("");
        info!("🎯 SOVEREIGN SCIENCE: Your compute, your data, your control");
    }
}
