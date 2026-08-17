// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ToadStoolResult;
use tracing::{debug, info};
use uuid::Uuid;

use super::super::session::{AiPreferences, AiSession};
use super::super::types::{
    ConfigurationSummary, ExecutionIntent, McpResponse, MemoryPattern, ResourceAllocation,
    SessionInfo,
};
use std::time::SystemTime;

impl super::AiMcpInterface {
    /// Handle natural language configuration from AI
    pub(super) async fn handle_natural_config(
        &mut self,
        request_id: String,
        instruction: String,
    ) -> ToadStoolResult<McpResponse> {
        info!("🧠 Processing natural language config: {}", instruction);

        let config_response = self
            .config_assistant
            .configure_from_text(&instruction)
            .await?;

        let config_summary = ConfigurationSummary {
            name: "AI Generated Configuration".to_string(),
            description: "Generated from AI instructions".to_string(),
            security_level: if config_response.security.auth.enabled {
                "High".to_string()
            } else {
                "Basic".to_string()
            },
            performance_level: if config_response.runtime.gpu.is_some() {
                "High Performance".to_string()
            } else {
                "Standard Performance".to_string()
            },
            enabled_runtimes: {
                let mut runtimes = vec!["Native".to_string()];
                if !config_response.runtime.container.runtime.is_empty() {
                    runtimes.push("Container".to_string());
                }
                if config_response.runtime.wasm.max_memory > 0 {
                    runtimes.push("WASM".to_string());
                }
                if config_response.runtime.gpu.is_some() {
                    runtimes.push("GPU".to_string());
                }
                runtimes
            },
            resource_allocation: ResourceAllocation {
                cpu_cores: config_response.runtime.resource_limits.max_cpu_usage,
                memory_gb: config_response.runtime.resource_limits.max_memory_usage,
                gpu_enabled: config_response.runtime.gpu.is_some(),
                storage_gb: config_response.runtime.resource_limits.max_disk_usage / 1024.0,
            },
        };

        Ok(McpResponse {
            request_id,
            success: true,
            message: "✅ Configuration applied".to_string(),
            data: Some(serde_json::to_value(&config_response)?),
            suggestions: vec![],
            session_info: None,
            config_applied: Some(config_summary),
        })
    }

    /// Execute code with AI-understood intent
    pub(super) async fn handle_execute_with_intent(
        &mut self,
        request_id: String,
        code: String,
        intent: ExecutionIntent,
    ) -> ToadStoolResult<McpResponse> {
        info!("🎯 Executing with AI intent: {}", intent.purpose);

        let mut config = self.auto_config.generate_intelligent_config().await?;
        Self::apply_intent_optimizations(&mut config, &intent);

        let config_summary = ConfigurationSummary {
            name: "Intent-Optimized Configuration".to_string(),
            description: format!("Optimized for: {}", intent.purpose),
            security_level: if config.security.auth.enabled {
                "High".to_string()
            } else {
                "Basic".to_string()
            },
            performance_level: "AI-Optimized".to_string(),
            enabled_runtimes: vec!["All Available".to_string()],
            resource_allocation: ResourceAllocation {
                cpu_cores: config.runtime.resource_limits.max_cpu_usage,
                memory_gb: config.runtime.resource_limits.max_memory_usage,
                gpu_enabled: config.runtime.gpu.is_some(),
                storage_gb: config.runtime.resource_limits.max_disk_usage / 1024.0,
            },
        };

        Ok(McpResponse {
            request_id,
            success: true,
            message: format!(
                "🚀 Code ready for execution with intent: {}",
                intent.purpose
            ),
            data: Some(serde_json::json!({
                "code": code,
                "intent": intent,
                "config": config
            })),
            suggestions: vec![
                "Consider adding error handling for production use".to_string(),
                "Monitor resource usage during execution".to_string(),
                "Enable logging for debugging if needed".to_string(),
            ],
            session_info: None,
            config_applied: Some(config_summary),
        })
    }

    /// Optimize configuration for specific task
    pub(super) async fn handle_optimize_for_task(
        &mut self,
        request_id: String,
        task_description: String,
    ) -> ToadStoolResult<McpResponse> {
        info!("⚡ Optimizing for task: {}", task_description);

        let config_response = self
            .config_assistant
            .configure_from_text(&task_description)
            .await?;

        Ok(McpResponse {
            request_id,
            success: true,
            message: format!("🎯 ToadStool optimized for: {task_description}"),
            data: Some(serde_json::to_value(&config_response)?),
            suggestions: vec![
                "Configuration optimized for your specific use case".to_string(),
                "All system resources allocated optimally".to_string(),
                "Security and performance balanced for your needs".to_string(),
            ],
            session_info: None,
            config_applied: Some(ConfigurationSummary {
                name: "Task-Optimized Configuration".to_string(),
                description: format!("Optimized for: {task_description}"),
                security_level: if config_response.security.auth.enabled {
                    "High".to_string()
                } else {
                    "Basic".to_string()
                },
                performance_level: "Task-Optimized".to_string(),
                enabled_runtimes: vec!["Optimal Selection".to_string()],
                resource_allocation: ResourceAllocation {
                    cpu_cores: config_response.runtime.resource_limits.max_cpu_usage,
                    memory_gb: config_response.runtime.resource_limits.max_memory_usage,
                    gpu_enabled: config_response.runtime.gpu.is_some(),
                    storage_gb: config_response.runtime.resource_limits.max_disk_usage / 1024.0,
                },
            }),
        })
    }

    /// Get current system status
    pub(super) async fn handle_get_system_status(
        &mut self,
        request_id: String,
    ) -> ToadStoolResult<McpResponse> {
        info!("📊 Getting system status for AI");

        let hardware = self.auto_config.scan_system().await?;
        let ecosystem = self.auto_config.discover_services().await?;

        let status = serde_json::json!({
            "hardware": {
                "cpu_cores": hardware.cpu_cores,
                "memory_gb": hardware.memory_gb,
                "gpu_count": hardware.gpu_count,
                "storage_gb": hardware.storage_gb,
                "platform": format!("{} ({} cores)", hardware.cpu_info.model_name, hardware.cpu_info.physical_cores)
            },
            "ecosystem": {
                "services_discovered": ecosystem.discovered_services.len(),
                "available_services": ecosystem.discovered_services.keys().collect::<Vec<_>>()
            },
            "toadstool_status": "Ready for AI workloads",
            "request_count": *self.request_counter.read().unwrap_or_else(std::sync::PoisonError::into_inner),
            "active_sessions": self.active_sessions.read().unwrap_or_else(std::sync::PoisonError::into_inner).len()
        });

        Ok(McpResponse {
            request_id,
            success: true,
            message: "📊 System status retrieved".to_string(),
            data: Some(status),
            suggestions: vec![
                "System is ready for AI workloads".to_string(),
                "All hardware resources detected and available".to_string(),
                "Ecosystem services discovered and ready".to_string(),
            ],
            session_info: None,
            config_applied: None,
        })
    }

    /// Create new AI session
    pub(super) async fn handle_create_session(
        &self,
        request_id: String,
        agent_id: String,
        preferences: Option<AiPreferences>,
    ) -> ToadStoolResult<McpResponse> {
        let session_id = Uuid::new_v4().to_string();

        info!(
            "🔄 Creating new AI session: {} for agent: {}",
            session_id, agent_id
        );

        let session = AiSession {
            session_id: session_id.clone(),
            agent_id: agent_id.clone(),
            current_config: None,
            started_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            preferences: preferences.unwrap_or_default(),
        };

        self.active_sessions
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(session_id.clone(), session.clone());

        Ok(McpResponse {
            request_id,
            success: true,
            message: format!("🎉 AI session created: {session_id}"),
            data: Some(serde_json::json!({
                "session_id": session_id,
                "agent_id": agent_id,
                "created_at": session.started_at.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs()
            })),
            suggestions: vec![
                "Session created successfully".to_string(),
                "You can now make personalized configuration requests".to_string(),
                "Preferences will be remembered for this session".to_string(),
            ],
            session_info: Some(SessionInfo {
                session_id,
                status: "Active".to_string(),
                preferences: session.preferences,
            }),
            config_applied: None,
        })
    }

    /// Update session preferences
    pub(super) async fn handle_update_preferences(
        &self,
        request_id: String,
        session_id: Option<String>,
        preferences: AiPreferences,
    ) -> ToadStoolResult<McpResponse> {
        if let Some(session_id) = session_id {
            let mut sessions = self
                .active_sessions
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(session) = sessions.get_mut(&session_id) {
                session.preferences = preferences.clone();
                session.last_activity = SystemTime::now();

                info!("🔧 Updated preferences for session: {}", session_id);

                return Ok(McpResponse {
                    request_id,
                    success: true,
                    message: "✅ Preferences updated".to_string(),
                    data: Some(serde_json::to_value(&preferences)?),
                    suggestions: vec![
                        "Preferences updated successfully".to_string(),
                        "Future requests will use these preferences".to_string(),
                    ],
                    session_info: Some(SessionInfo {
                        session_id,
                        status: "Active".to_string(),
                        preferences,
                    }),
                    config_applied: None,
                });
            }
        }

        Ok(McpResponse {
            request_id,
            success: false,
            message: "❌ Session not found".to_string(),
            data: None,
            suggestions: vec![
                "Please create a session first".to_string(),
                "Check that the session ID is correct".to_string(),
            ],
            session_info: None,
            config_applied: None,
        })
    }

    /// Apply intent-specific optimizations
    pub(super) fn apply_intent_optimizations(
        config: &mut toadstool_config::ToadStoolConfig,
        intent: &ExecutionIntent,
    ) {
        debug!("🔧 Applying intent optimizations for: {}", intent.purpose);

        for requirement in &intent.security_requirements {
            match requirement.as_str() {
                "high_security" => {
                    config.security.sandbox.enabled = true;
                    config.security.auth.enabled = true;
                    config.security.auth.provider = "jwt".to_string();
                }
                "data_privacy" => {
                    config.security.auth.enabled = true;
                    config.security.auth.provider = "jwt".to_string();
                }
                _ => debug!("Unknown security requirement: {}", requirement),
            }
        }

        match intent.performance_expectations.memory_pattern {
            MemoryPattern::Large => {
                let current_memory = config.runtime.resource_limits.max_memory_usage;
                config.runtime.resource_limits.max_memory_usage =
                    (current_memory * 1.5).min(0.9 * 16.0);
            }
            MemoryPattern::Minimal => {
                let current_memory = config.runtime.resource_limits.max_memory_usage;
                config.runtime.resource_limits.max_memory_usage =
                    (current_memory * 0.5).max(0.9 * 16.0);
            }
            _ => {}
        }

        if let Some(cpu_cores) = intent.resource_hints.cpu_cores {
            config.runtime.resource_limits.max_cpu_usage = cpu_cores.min(16.0) / 16.0;
        }
        if let Some(memory_gb) = intent.resource_hints.memory_gb {
            config.runtime.resource_limits.max_memory_usage = (memory_gb / 16.0).min(0.9);
        }
        if intent.resource_hints.gpu_required {
            config.runtime.gpu = Some(toadstool_config::GpuConfig::default());
        }
    }
}
