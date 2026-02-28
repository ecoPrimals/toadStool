//! # AI Session & Preferences
//!
//! Session tracking and AI preference types for the MCP interface.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// AI session tracking for context management
#[derive(Debug, Clone)]
pub struct AiSession {
    /// Session identifier
    pub session_id: String,
    /// AI agent identifier
    pub agent_id: String,
    /// Current configuration state
    pub current_config: Option<toadstool_config::ToadStoolConfig>,
    /// Session start time
    pub started_at: SystemTime,
    /// Last activity timestamp
    pub last_activity: SystemTime,
    /// AI preferences learned
    pub preferences: AiPreferences,
}

/// AI preferences for personalized configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPreferences {
    /// Preferred security level
    pub security_level: Option<String>,
    /// Performance vs. security trade-off
    pub performance_priority: f64, // 0.0 = security first, 1.0 = performance first
    /// Resource usage preferences
    pub resource_preferences: ResourcePreferences,
    /// Preferred runtime environments
    pub runtime_preferences: Vec<String>,
}

/// Resource allocation preferences (AI/MCP specific)
///
/// Distinct from `natural_language::ResourcePreferences` which uses
/// cpu_intensive/memory_intensive. This type uses strategy strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePreferences {
    /// CPU allocation strategy
    pub cpu_strategy: String, // "conservative", "balanced", "aggressive"
    /// Memory allocation strategy
    pub memory_strategy: String,
    /// GPU usage preference
    pub gpu_preference: String, // "auto", "required", "disabled"
    /// Storage performance preference
    pub storage_preference: String, // "speed", "capacity", "balanced"
}

impl Default for AiPreferences {
    fn default() -> Self {
        Self {
            security_level: Some("balanced".to_string()),
            performance_priority: 0.7,
            resource_preferences: ResourcePreferences {
                cpu_strategy: "balanced".to_string(),
                memory_strategy: "balanced".to_string(),
                gpu_preference: "auto".to_string(),
                storage_preference: "balanced".to_string(),
            },
            runtime_preferences: vec!["native".to_string(), "container".to_string()],
        }
    }
}
