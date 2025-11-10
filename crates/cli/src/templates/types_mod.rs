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
    pub name: String,
    pub description: String,
    pub primals: Vec<String>,
    pub services: Vec<CustomServiceSpec>,
    pub security_level: String,
    pub resource_profile: String,
}

/// Custom service specification
#[derive(Debug, Clone)]
pub struct CustomServiceSpec {
    pub name: String,
    pub image: String,
    pub ports: Vec<u16>,
    pub environment: HashMap<String, String>,
    pub volumes: Vec<String>,
}
