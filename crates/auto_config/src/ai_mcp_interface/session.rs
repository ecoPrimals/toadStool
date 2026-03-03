// SPDX-License-Identifier: AGPL-3.0-or-later
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_preferences_default() {
        let prefs = AiPreferences::default();
        assert_eq!(prefs.security_level.as_deref(), Some("balanced"));
        assert!((prefs.performance_priority - 0.7).abs() < f64::EPSILON);
        assert_eq!(prefs.resource_preferences.cpu_strategy, "balanced");
        assert_eq!(prefs.resource_preferences.gpu_preference, "auto");
        assert_eq!(prefs.runtime_preferences.len(), 2);
        assert_eq!(prefs.runtime_preferences[0], "native");
    }

    #[test]
    fn test_resource_preferences_fields() {
        let rp = ResourcePreferences {
            cpu_strategy: "aggressive".to_string(),
            memory_strategy: "conservative".to_string(),
            gpu_preference: "required".to_string(),
            storage_preference: "speed".to_string(),
        };
        assert_eq!(rp.cpu_strategy, "aggressive");
        assert_eq!(rp.storage_preference, "speed");
    }

    #[test]
    fn test_ai_preferences_serialization_roundtrip() {
        let prefs = AiPreferences::default();
        let json = serde_json::to_string(&prefs).expect("serialize");
        let restored: AiPreferences = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.security_level, prefs.security_level);
        assert!((restored.performance_priority - prefs.performance_priority).abs() < f64::EPSILON);
        assert_eq!(
            restored.resource_preferences.gpu_preference,
            prefs.resource_preferences.gpu_preference
        );
    }

    #[test]
    fn test_ai_session_construction() {
        let session = AiSession {
            session_id: "s1".to_string(),
            agent_id: "agent-1".to_string(),
            current_config: None,
            started_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            preferences: AiPreferences::default(),
        };
        assert_eq!(session.session_id, "s1");
        assert!(session.current_config.is_none());
    }
}
