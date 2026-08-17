// SPDX-License-Identifier: AGPL-3.0-or-later
//! Runtime AI/MCP interface implementation (requires `runtime` feature).

#[path = "handlers.rs"]
mod handlers;

use crate::{IntelligentAutoConfig, NaturalLanguageConfig, ToadStoolResult};
use std::collections::HashMap;
use std::time::SystemTime;

use std::sync::RwLock;
use tracing::info;

use super::session::AiSession;
use super::types::{McpRequest, McpRequestType, McpResponse};

/// Universal AI/MCP interface
///
/// Works with ANY AI service that supports Model Context Protocol (MCP).
/// Discovers providers at runtime via capability-based discovery.
pub struct AiMcpInterface {
    /// Natural language configuration processor
    pub(crate) config_assistant: NaturalLanguageConfig,
    /// Auto-configuration system
    pub(crate) auto_config: IntelligentAutoConfig,
    /// Active AI sessions
    pub(crate) active_sessions: RwLock<HashMap<String, AiSession>>,
    /// Request counter for tracking
    pub(crate) request_counter: RwLock<u64>,
}

impl AiMcpInterface {
    /// Create new AI/MCP interface
    ///
    /// Note: Future versions will integrate runtime discovery to find
    /// AI providers via `AI_PROCESSING` capability
    pub fn new() -> ToadStoolResult<Self> {
        info!("🤖 Initializing AI/MCP interface (capability-based)");

        Ok(Self {
            config_assistant: NaturalLanguageConfig::new(),
            auto_config: IntelligentAutoConfig::new(),
            active_sessions: RwLock::new(HashMap::new()),
            request_counter: RwLock::new(0),
        })
    }

    /// Create new AI/MCP interface with custom components (for testing)
    #[cfg(test)]
    pub fn new_with_components(
        config_assistant: NaturalLanguageConfig,
        auto_config: IntelligentAutoConfig,
    ) -> ToadStoolResult<Self> {
        info!("🤖 Initializing AI/MCP interface with custom components");

        Ok(Self {
            config_assistant,
            auto_config,
            active_sessions: RwLock::new(HashMap::new()),
            request_counter: RwLock::new(0),
        })
    }

    /// Process MCP requests from any AI provider
    ///
    /// Handles requests from intelligence services, Claude, `OpenAI`, or any MCP-compatible service
    pub async fn process_ai_request(
        &mut self,
        request: McpRequest,
    ) -> ToadStoolResult<McpResponse> {
        // Increment request counter
        {
            let mut counter = self
                .request_counter
                .write()
                .unwrap_or_else(|e| e.into_inner());
            *counter += 1;
        }

        info!(
            "🤖 Processing MCP request: {} (type: {:?})",
            request.request_id, request.request_type
        );

        let response = match request.request_type {
            McpRequestType::NaturalLanguageConfig { instruction } => {
                self.handle_natural_config(request.request_id.clone(), instruction)
                    .await
            }
            McpRequestType::ExecuteWithIntent { code, intent } => {
                self.handle_execute_with_intent(request.request_id.clone(), code, intent)
                    .await
            }
            McpRequestType::OptimizeForTask { task_description } => {
                self.handle_optimize_for_task(request.request_id.clone(), task_description)
                    .await
            }
            McpRequestType::GetSystemStatus => {
                self.handle_get_system_status(request.request_id.clone())
                    .await
            }
            McpRequestType::CreateSession { preferences } => {
                self.handle_create_session(
                    request.request_id.clone(),
                    request.agent_id,
                    preferences,
                )
                .await
            }
            McpRequestType::UpdatePreferences { preferences } => {
                self.handle_update_preferences(
                    request.request_id.clone(),
                    request.session_id.clone(),
                    preferences,
                )
                .await
            }
        };

        // Update session activity if applicable
        if let Some(session_id) = &request.session_id {
            self.update_session_activity(session_id).await;
        }

        response
    }

    /// Update session activity timestamp
    async fn update_session_activity(&self, session_id: &str) {
        let mut sessions = self
            .active_sessions
            .write()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = SystemTime::now();
        }
    }

    /// Get session statistics
    pub async fn get_session_stats(&self) -> HashMap<String, serde_json::Value> {
        let sessions = self
            .active_sessions
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let request_count = *self
            .request_counter
            .read()
            .unwrap_or_else(|e| e.into_inner());

        let mut stats = HashMap::new();
        stats.insert(
            "active_sessions".to_string(),
            serde_json::Value::Number(sessions.len().into()),
        );
        stats.insert(
            "total_requests".to_string(),
            serde_json::Value::Number(request_count.into()),
        );
        let avg_duration_desc = if sessions.is_empty() {
            "no active sessions".to_string()
        } else {
            let now = SystemTime::now();
            let total_secs: u64 = sessions
                .values()
                .filter_map(|s| now.duration_since(s.started_at).ok())
                .map(|d| d.as_secs())
                .sum();
            let avg_secs = total_secs / sessions.len() as u64;
            let mins = avg_secs / 60;
            let secs = avg_secs % 60;
            drop(sessions);
            format!("{mins}m {secs}s")
        };
        stats.insert(
            "average_session_duration".to_string(),
            serde_json::Value::String(avg_duration_desc),
        );

        stats
    }
}
