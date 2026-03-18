// SPDX-License-Identifier: AGPL-3.0-or-later
//! Type definitions for natural language configuration
//!
//! This module contains the core types used across the natural language
//! configuration system, including preferences, intents, and templates.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Performance preference levels
#[derive(Debug, Clone)]
pub enum PerformancePreference {
    PowerSaver,
    Balanced,
    HighPerformance,
    MaximumPerformance,
}

/// Security preference levels for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityPreference {
    /// Minimal security for development
    Minimal,
    /// Balanced security for general use
    Balanced,
    /// High security for production
    High,
    /// Maximum security for sensitive workloads
    Maximum,
}

/// Usage patterns for different workload types
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum UsagePattern {
    /// Machine learning and AI workloads
    MachineLearning,
    /// Web development and services
    WebDevelopment,
    /// Scientific computing
    ScientificComputing,
    /// General purpose computing
    #[default]
    GeneralPurpose,
    /// High performance computing
    HighPerformanceComputing,
    /// Development and testing
    Development,
    /// Enterprise security workloads
    EnterpriseSecurity,
    /// Custom usage pattern
    Custom(String),
}

/// Configuration intent extracted from natural language
#[derive(Debug, Clone)]
pub struct ConfigurationIntent {
    pub keywords: Vec<String>,
    pub priority_features: Vec<String>,
    pub performance_preference: PerformancePreference,
    pub security_preference: SecurityPreference,
}

/// Intent analysis results
#[derive(Debug, Clone)]
pub struct IntentAnalysis {
    pub primary_intent: String,
    pub confidence: f64,
    pub matched_keywords: Vec<String>,
    pub secondary_intents: Vec<(String, f64)>,
    pub explicit_preferences: ExplicitPreferences,
}

/// Explicit preferences extracted from text
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExplicitPreferences {
    pub performance_priority: Option<String>,
    pub security_priority: Option<String>,
    pub memory_usage: Option<String>,
    pub use_gpu: Option<bool>,
    pub use_containers: Option<bool>,
}

/// Runtime types that can be enabled
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeType {
    Gpu,
    Python,
    Container,
    Wasm,
}

/// Runtime preferences in template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimePreferences {
    pub enabled_runtimes: HashSet<RuntimeType>,
    pub gpu_memory_fraction: f64,
    pub python_memory_limit_gb: f64,
}

impl RuntimePreferences {
    /// Check if a runtime type is enabled
    #[must_use]
    pub fn is_enabled(&self, runtime_type: &RuntimeType) -> bool {
        self.enabled_runtimes.contains(runtime_type)
    }

    /// Check if GPU is enabled
    #[must_use]
    pub fn enable_gpu(&self) -> bool {
        self.is_enabled(&RuntimeType::Gpu)
    }

    /// Check if Python is enabled
    #[must_use]
    pub fn enable_python(&self) -> bool {
        self.is_enabled(&RuntimeType::Python)
    }

    /// Check if Container is enabled
    #[must_use]
    pub fn enable_container(&self) -> bool {
        self.is_enabled(&RuntimeType::Container)
    }

    /// Check if WASM is enabled
    #[must_use]
    pub fn enable_wasm(&self) -> bool {
        self.is_enabled(&RuntimeType::Wasm)
    }
}

/// Resource preferences for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePreferences {
    pub cpu_intensive: bool,
    pub memory_intensive: bool,
    pub requires_gpu: bool,
    pub memory_allocation_strategy: String,
    pub cpu_priority: String,
    pub storage_optimization: String,
}

/// Security preferences for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPreferences {
    pub sandbox_level: String,
    pub network_isolation: bool,
    pub crypto_verification: bool,
}

/// Configuration template for specific use cases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationTemplate {
    pub name: String,
    pub description: String,
    pub use_case: UsagePattern,
    pub security_preference: SecurityPreference,
    pub runtime_preferences: RuntimePreferences,
    pub resource_preferences: ResourcePreferences,
    pub explicit_preferences: ExplicitPreferences,
}
