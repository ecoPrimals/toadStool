//! # MCP Protocol Message Types
//!
//! Request/response structures for the Model Context Protocol (MCP).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use super::session::AiPreferences;

// =============================================================================
// Request Types
// =============================================================================

/// MCP request from any AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    /// Request identifier
    pub request_id: String,
    /// Session identifier
    pub session_id: Option<String>,
    /// AI agent identifier
    pub agent_id: String,
    /// Request type
    pub request_type: McpRequestType,
    /// Request metadata
    pub metadata: HashMap<String, String>,
    /// Request timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: SystemTime,
}

/// Types of MCP requests from AI providers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpRequestType {
    /// Natural language configuration request
    NaturalLanguageConfig { instruction: String },
    /// Execute code with AI intent
    ExecuteWithIntent {
        code: String,
        intent: ExecutionIntent,
    },
    /// Optimize configuration for specific task
    OptimizeForTask { task_description: String },
    /// Get current system status
    GetSystemStatus,
    /// Create new AI session
    CreateSession { preferences: Option<AiPreferences> },
    /// Update session preferences
    UpdatePreferences { preferences: AiPreferences },
}

/// AI-understood execution intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionIntent {
    /// What the AI thinks this code should do
    pub purpose: String,
    /// Security requirements from AI analysis
    pub security_requirements: Vec<String>,
    /// Performance expectations
    pub performance_expectations: PerformanceExpectations,
    /// Resource hints from AI
    pub resource_hints: ResourceHints,
    /// Expected runtime environment
    pub runtime_hint: Option<String>,
}

/// Performance expectations from AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceExpectations {
    /// Expected execution time
    pub expected_duration: Option<Duration>,
    /// CPU intensity (0.0 - 1.0)
    pub cpu_intensity: f64,
    /// Memory usage pattern
    pub memory_pattern: MemoryPattern,
    /// I/O intensity
    pub io_intensity: IoIntensity,
}

/// Memory usage patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryPattern {
    /// Small memory footprint
    Minimal,
    /// Normal memory usage
    Normal,
    /// Large memory requirements
    Large,
    /// Streaming/incremental memory usage
    Streaming,
}

/// I/O intensity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IoIntensity {
    /// Minimal I/O
    Low,
    /// Normal I/O
    Medium,
    /// High I/O throughput
    High,
    /// Extremely high I/O
    Extreme,
}

/// Resource hints from AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceHints {
    /// Recommended CPU cores
    pub cpu_cores: Option<f64>,
    /// Recommended memory in GB
    pub memory_gb: Option<f64>,
    /// GPU acceleration needed
    pub gpu_required: bool,
    /// Storage requirements
    pub storage_gb: Option<f64>,
}

// =============================================================================
// Response Types
// =============================================================================

/// MCP response to AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    /// Request identifier
    pub request_id: String,
    /// Success status
    pub success: bool,
    /// Human-readable message
    pub message: String,
    /// Structured response data
    pub data: Option<serde_json::Value>,
    /// AI-friendly suggestions
    pub suggestions: Vec<String>,
    /// Session information
    pub session_info: Option<SessionInfo>,
    /// Configuration applied
    pub config_applied: Option<ConfigurationSummary>,
}

/// Session information in response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session identifier
    pub session_id: String,
    /// Session status
    pub status: String,
    /// Current preferences
    pub preferences: AiPreferences,
}

/// Configuration summary for AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationSummary {
    /// Configuration name
    pub name: String,
    /// Description of changes
    pub description: String,
    /// Security level applied
    pub security_level: String,
    /// Performance optimization level
    pub performance_level: String,
    /// Enabled runtimes
    pub enabled_runtimes: Vec<String>,
    /// Resource allocation
    pub resource_allocation: ResourceAllocation,
}

/// Resource allocation summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// CPU cores allocated
    pub cpu_cores: f64,
    /// Memory allocated in GB
    pub memory_gb: f64,
    /// GPU acceleration enabled
    pub gpu_enabled: bool,
    /// Storage allocated in GB
    pub storage_gb: f64,
}
