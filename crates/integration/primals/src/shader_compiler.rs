// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shader compiler capability discovery — queries coralReef via capability-based IPC.
//!
//! ToadStool discovers the shader compiler (coralReef) at runtime by its
//! `shader.compile` capability. It then queries `shader.compile.capabilities`
//! to learn what compilation backends and precision modes are available,
//! feeding that into silicon registry routing decisions.
//!
//! **Self-knowledge pattern**: ToadStool never names coralReef directly.
//! It discovers "whoever provides `shader.compile`" and queries their surface.

use serde::{Deserialize, Serialize};
use toadstool_common::interned_strings::capabilities;

/// Shader compiler capabilities as discovered from the compilation primal.
///
/// This struct represents the compilation surface that the shader compiler
/// (coralReef) reports. ToadStool uses it to decide:
/// - Whether native ISA dispatch is available (vs. WGSL-only)
/// - Which precision modes the compiler supports (Fp64Strategy)
/// - Whether subgroup operations are supported (for efficient reductions)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShaderCompilerCapabilities {
    /// Available compilation backends (e.g. "wgsl", "spirv", "ptx", "amdgcn")
    pub backends: Vec<String>,
    /// Supported precision strategies reported by the compiler
    pub precision_modes: Vec<PrecisionMode>,
    /// Whether integer subgroup operations are available
    pub integer_subgroup: bool,
    /// Whether the compiler supports GEMM tiling optimization
    pub gemm_tiling: bool,
    /// Maximum workgroup size the compiler can target
    pub max_workgroup_size: Option<u32>,
    /// Compiler version string
    pub compiler_version: Option<String>,
}

/// Precision mode advertised by the shader compiler.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PrecisionMode {
    /// Native f32 only
    F32,
    /// Double-float emulation via two f32s
    Df64,
    /// Native f64 (SHADER_F64 feature)
    NativeF64,
    /// f16 (half precision, free throughput on some GPUs)
    F16,
    /// Custom precision strategy
    Custom(String),
}

/// Discovery status of the shader compiler.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShaderCompilerStatus {
    /// Not yet discovered
    Unknown,
    /// Discovered and capabilities queried
    Available(String),
    /// Discovery attempted but no shader.compile provider found
    Unavailable,
    /// Discovery failed with error
    Error(String),
}

/// Shader compiler discovery query — the JSON-RPC request type.
///
/// Sent to whichever primal provides `shader.compile` capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderCapabilitiesQuery {
    /// Which silicon units we're interested in routing to
    pub target_units: Vec<String>,
    /// Whether to include experimental/unstable backends
    pub include_experimental: bool,
}

impl Default for ShaderCapabilitiesQuery {
    fn default() -> Self {
        Self {
            target_units: vec![
                "shader_core".to_string(),
                "tensor_core".to_string(),
            ],
            include_experimental: false,
        }
    }
}

/// Result of a shader compiler capability query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderCapabilitiesResponse {
    /// The capabilities of the discovered compiler
    pub capabilities: ShaderCompilerCapabilities,
    /// Socket/endpoint address of the compiler
    pub endpoint: String,
    /// Status of the query
    pub status: ShaderCompilerStatus,
}

/// The capability constant used to discover the shader compiler.
pub const SHADER_COMPILE_CAPABILITY: &str = capabilities::SHADER_COMPILE;

/// The specific method to query compiler capabilities.
pub const SHADER_CAPABILITIES_METHOD: &str = "shader.compile.capabilities";
