// SPDX-License-Identifier: AGPL-3.0-only
//! Template type definitions

use std::collections::HashMap;

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
    /// Template name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Primal names to include
    pub primals: Vec<String>,
    /// Custom service specs
    pub services: Vec<CustomServiceSpec>,
    /// Security level (low, medium, high, maximum)
    pub security_level: String,
    /// Resource profile (minimal, standard, high)
    pub resource_profile: String,
}

/// Custom service specification
#[derive(Debug, Clone)]
pub struct CustomServiceSpec {
    /// Service name
    pub name: String,
    /// Container image
    pub image: String,
    /// Exposed ports
    pub ports: Vec<u16>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Volume mounts
    pub volumes: Vec<String>,
}
