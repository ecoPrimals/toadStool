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
//! - `create_distributed_template()`: coordination / orchestration, multi-node clusters
//! - `create_sovereign_template()`: Maximum security, air-gapped configuration
//! - `create_custom_template()`: User-specified custom configurations
//!
//! Extracted from `generator_impl.rs` (Nov 7, 2025) as part of the refactoring
//! to keep files under 1000 lines.
//!
//! Ports sourced from `EnvironmentConfig` at runtime; only well-known service defaults (Redis 6379, Postgres 5432, TensorBoard 6006) are literal.

mod custom_templates;
mod infrastructure_templates;
mod ml_science_templates;

// Re-export all template creators to preserve public API
pub use custom_templates::create_custom_template;
pub use infrastructure_templates::{create_distributed_template, create_sovereign_template};
pub use ml_science_templates::{
    create_ai_research_template, create_genomics_template, create_quantum_template,
    create_science_template, create_vision_template,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::types_mod::{CustomServiceSpec, CustomTemplateSpec};

    #[test]
    fn test_create_science_template() {
        let (name, desc, primals, services, _res, _security, _, _) = create_science_template();
        assert!(name.contains("science"));
        assert!(desc.contains("Jupyter") || desc.contains("data"));
        assert!(!primals.is_empty() || !services.is_empty());
        assert!(services.contains_key("jupyter") || services.contains_key("postgres"));
    }

    #[test]
    fn test_create_ai_research_template() {
        let (name, _, primals, services, _, _, _, _) = create_ai_research_template();
        assert!(name.contains("ai-research"));
        assert!(!primals.is_empty() || !services.is_empty());
        assert!(services.contains_key("jupyter") || services.len() > 1);
    }

    #[test]
    fn test_create_quantum_template() {
        let (name, desc, _, services, _, _, _, _) = create_quantum_template();
        assert!(name.contains("quantum"));
        assert!(!desc.is_empty());
        assert!(services.contains_key("qiskit") || !services.is_empty());
    }

    #[test]
    fn test_create_genomics_template() {
        let (name, _, _, _, _res, security, _, _) = create_genomics_template();
        assert!(name.contains("genomics"));
        assert_eq!(security.isolation_level, "maximum");
    }

    #[test]
    fn test_create_vision_template() {
        let (name, _, _, services, _, _, _, _) = create_vision_template();
        assert!(name.contains("vision"));
        assert!(services.contains_key("opencv") || !services.is_empty());
    }

    #[test]
    fn test_create_distributed_template() {
        let (name, _, primals, services, _, _, _, _) = create_distributed_template();
        assert!(name.contains("distributed"));
        assert!(!primals.is_empty() || !services.is_empty());
    }

    #[test]
    fn test_create_sovereign_template() {
        let (name, _, _, _, _res, security, _, _) = create_sovereign_template();
        assert!(name.contains("sovereign"));
        assert!(security.security_required);
        assert_eq!(security.isolation_level, "maximum");
    }

    #[test]
    fn test_create_custom_template_minimal() {
        let spec = CustomTemplateSpec {
            name: "my-lab".to_string(),
            description: "Custom lab".to_string(),
            primals: vec![],
            services: vec![],
            resource_profile: "medium".to_string(),
            security_level: "standard".to_string(),
        };
        let (name, desc, _, _, resources, security, _, _) = create_custom_template(&spec);
        assert_eq!(name, "my-lab-biome");
        assert_eq!(desc, "Custom lab");
        assert_eq!(security.isolation_level, "standard");
        assert!(resources.cpu_limit.unwrap() > 0.0);
    }

    #[test]
    fn test_create_custom_template_with_primals() {
        let spec = CustomTemplateSpec {
            name: "custom".to_string(),
            description: "Test".to_string(),
            primals: vec!["custom-primal".to_string()],
            services: vec![],
            resource_profile: "low".to_string(),
            security_level: "low".to_string(),
        };
        let (_, _, primals, _, resources, _, _, _) = create_custom_template(&spec);
        assert!(primals.contains_key("custom-primal"));
        assert_eq!(resources.cpu_limit, Some(4.0));
    }

    #[test]
    fn test_create_custom_template_high_profile() {
        let spec = CustomTemplateSpec {
            name: "hpc".to_string(),
            description: "HPC".to_string(),
            primals: vec![],
            services: vec![],
            resource_profile: "high".to_string(),
            security_level: "high".to_string(),
        };
        let (_, _, _, _, resources, _, _, _) = create_custom_template(&spec);
        assert_eq!(resources.cpu_limit, Some(32.0));
        assert_eq!(resources.memory_limit, Some("128GB".to_string()));
    }

    #[test]
    fn test_create_custom_template_with_services() {
        let spec = CustomTemplateSpec {
            name: "svc-test".to_string(),
            description: String::new(),
            primals: vec![],
            services: vec![CustomServiceSpec {
                name: "redis".to_string(),
                image: "redis".to_string(),
                environment: std::collections::HashMap::new(),
                ports: vec![6379],
                volumes: vec![],
            }],
            resource_profile: "medium".to_string(),
            security_level: "standard".to_string(),
        };
        let (_, _, _, services, _, _, _, _) = create_custom_template(&spec);
        assert!(services.contains_key("redis"));
        let svc = services.get("redis").unwrap();
        assert!(matches!(
            svc.source,
            crate::WorkloadSource::Container { .. }
        ));
    }
}
