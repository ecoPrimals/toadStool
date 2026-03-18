// SPDX-License-Identifier: AGPL-3.0-or-later
//! Type definitions for natural language configuration
//!
//! This module contains the core types used across the natural language
//! configuration system, including preferences, intents, and templates.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Performance preference levels.
#[derive(Debug, Clone)]
pub enum PerformancePreference {
    /// Minimize power consumption.
    PowerSaver,
    /// Balance performance and power.
    Balanced,
    /// Prioritize performance.
    HighPerformance,
    /// Maximum performance.
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

/// Configuration intent extracted from natural language.
#[derive(Debug, Clone)]
pub struct ConfigurationIntent {
    /// Extracted keywords.
    pub keywords: Vec<String>,
    /// Priority features.
    pub priority_features: Vec<String>,
    /// Performance preference.
    pub performance_preference: PerformancePreference,
    /// Security preference.
    pub security_preference: SecurityPreference,
}

/// Intent analysis results.
#[derive(Debug, Clone)]
pub struct IntentAnalysis {
    /// Primary intent string.
    pub primary_intent: String,
    /// Confidence score (0–1).
    pub confidence: f64,
    /// Matched keywords.
    pub matched_keywords: Vec<String>,
    /// Secondary intents with confidence.
    pub secondary_intents: Vec<(String, f64)>,
    /// Explicit user preferences.
    pub explicit_preferences: ExplicitPreferences,
}

/// Explicit preferences extracted from text.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExplicitPreferences {
    /// Performance priority hint.
    pub performance_priority: Option<String>,
    /// Security priority hint.
    pub security_priority: Option<String>,
    /// Memory usage hint.
    pub memory_usage: Option<String>,
    /// Whether GPU is requested.
    pub use_gpu: Option<bool>,
    /// Whether containers are requested.
    pub use_containers: Option<bool>,
}

/// Runtime types that can be enabled.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeType {
    /// GPU runtime.
    Gpu,
    /// Python runtime.
    Python,
    /// Container runtime.
    Container,
    /// WebAssembly runtime.
    Wasm,
}

/// Runtime preferences in template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimePreferences {
    /// Enabled runtime types.
    pub enabled_runtimes: HashSet<RuntimeType>,
    /// GPU memory fraction (0–1).
    pub gpu_memory_fraction: f64,
    /// Python memory limit in GB.
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

/// Resource preferences for configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePreferences {
    /// CPU-intensive workload.
    pub cpu_intensive: bool,
    /// Memory-intensive workload.
    pub memory_intensive: bool,
    /// GPU required.
    pub requires_gpu: bool,
    /// Memory allocation strategy.
    pub memory_allocation_strategy: String,
    /// CPU priority setting.
    pub cpu_priority: String,
    /// Storage optimization hint.
    pub storage_optimization: String,
}

/// Security preferences for configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPreferences {
    /// Sandbox level.
    pub sandbox_level: String,
    /// Network isolation enabled.
    pub network_isolation: bool,
    /// Crypto verification enabled.
    pub crypto_verification: bool,
}

/// Configuration template for specific use cases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationTemplate {
    /// Template name.
    pub name: String,
    /// Template description.
    pub description: String,
    /// Use case pattern.
    pub use_case: UsagePattern,
    /// Security preference.
    pub security_preference: SecurityPreference,
    /// Runtime preferences.
    pub runtime_preferences: RuntimePreferences,
    /// Resource preferences.
    pub resource_preferences: ResourcePreferences,
    /// Explicit preferences.
    pub explicit_preferences: ExplicitPreferences,
}
