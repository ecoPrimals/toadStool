// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::nursery, dead_code)]
//! Types for standalone universal compute demo

#[derive(Debug, Clone)]
pub struct ComputeNode {
    pub id: String,
    pub node_type: String,
    pub capabilities: NodeCapabilities,
    pub performance_score: f64,
}

#[derive(Debug, Clone)]
pub struct NodeCapabilities {
    pub webgpu: bool,
    pub vulkan: bool,
    pub rocm: bool,
    pub opencl: bool,
    pub metal: bool,
    pub cuda: bool,
    pub oneapi: bool,
    pub llama_cpp_backends: Vec<String>,
    pub onnx_providers: Vec<String>,
    pub candle_support: bool,
    pub burn_support: bool,
    pub wasm_support: bool,
    pub edge_optimized: bool,
}

#[derive(Debug)]
pub struct WorkloadRequest {
    pub name: String,
    pub workload_type: WorkloadType,
    pub requires_proprietary: bool,
    pub preferred_frameworks: Vec<String>,
}

#[derive(Debug)]
pub enum WorkloadType {
    AiInference { model: String, framework: String },
    GeneralCompute { parallel: bool },
    MediaProcessing { codec: String },
    ScientificComputing { domain: String },
}

#[derive(Debug)]
pub struct SchedulingDecision {
    pub target_node: String,
    pub strategy: ExecutionStrategy,
    pub score: f64,
    pub reasoning: String,
    pub open_score_bonus: f64,
}

#[derive(Debug)]
pub enum ExecutionStrategy {
    WebGPU,
    Vulkan,
    ROCm,
    LlamaCppVulkan,
    LlamaCppMetal,
    LlamaCppROCm,
    ONNXVulkan,
    ONNXROCm,
    CandleWGPU,
    BurnWGPU,
    OptimizedCPU,
    CudaIsolated { warning: String },
}
