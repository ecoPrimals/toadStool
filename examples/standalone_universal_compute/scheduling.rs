// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::nursery, dead_code)]
//! Workload scheduling logic

use super::types::{ComputeNode, ExecutionStrategy, SchedulingDecision, WorkloadRequest, WorkloadType};

pub fn schedule_workload(request: &WorkloadRequest, ecosystem: &[ComputeNode]) -> SchedulingDecision {
    let mut scored_nodes: Vec<(&ComputeNode, f64, f64)> = ecosystem
        .iter()
        .map(|node| {
            let (score, open_bonus) = calculate_node_score(request, node);
            (node, score, open_bonus)
        })
        .collect();

    scored_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let (best_node, best_score, open_bonus) = scored_nodes.first().unwrap();
    let strategy = determine_execution_strategy(request, best_node);
    let reasoning = generate_reasoning(best_node, &strategy);

    SchedulingDecision {
        target_node: best_node.id.clone(),
        strategy,
        score: *best_score,
        reasoning,
        open_score_bonus: *open_bonus,
    }
}

pub fn calculate_node_score(request: &WorkloadRequest, node: &ComputeNode) -> (f64, f64) {
    let mut score = node.performance_score;
    let mut open_bonus = 0.0;

    if node.capabilities.webgpu {
        open_bonus += 800.0;
        score += 800.0;
    }
    if node.capabilities.vulkan {
        open_bonus += 600.0;
        score += 600.0;
    }
    if node.capabilities.rocm {
        open_bonus += 700.0;
        score += 700.0;
    }
    if node.capabilities.opencl {
        open_bonus += 400.0;
        score += 400.0;
    }
    if node.capabilities.metal {
        open_bonus += 500.0;
        score += 500.0;
    }

    let non_cuda_llama_backends = node
        .capabilities
        .llama_cpp_backends
        .iter()
        .filter(|b| *b != "cuda")
        .count();
    if non_cuda_llama_backends > 0 {
        let bonus = 1000.0 * non_cuda_llama_backends as f64;
        open_bonus += bonus;
        score += bonus;
    }

    let non_cuda_onnx_providers = node
        .capabilities
        .onnx_providers
        .iter()
        .filter(|p| *p != "cuda")
        .count();
    if non_cuda_onnx_providers > 0 {
        let bonus = 800.0 * non_cuda_onnx_providers as f64;
        open_bonus += bonus;
        score += bonus;
    }

    if node.capabilities.candle_support {
        open_bonus += 600.0;
        score += 600.0;
    }
    if node.capabilities.burn_support {
        open_bonus += 550.0;
        score += 550.0;
    }
    if node.capabilities.wasm_support {
        open_bonus += 300.0;
        score += 300.0;
    }
    if node.capabilities.edge_optimized {
        open_bonus += 250.0;
        score += 250.0;
    }

    if !request.requires_proprietary {
        if node.capabilities.cuda && !has_open_alternatives(node) {
            score -= 2000.0;
        } else if node.capabilities.cuda {
            score -= 300.0;
        }
    }

    (score, open_bonus)
}

pub fn has_open_alternatives(node: &ComputeNode) -> bool {
    node.capabilities.webgpu
        || node.capabilities.vulkan
        || node.capabilities.rocm
        || node.capabilities.metal
        || node.capabilities.opencl
}

fn determine_execution_strategy(request: &WorkloadRequest, node: &ComputeNode) -> ExecutionStrategy {
    match &request.workload_type {
        WorkloadType::AiInference { framework, .. } => match framework.as_str() {
            "llama.cpp" => {
                if node.capabilities.llama_cpp_backends.contains(&"vulkan".to_string()) {
                    ExecutionStrategy::LlamaCppVulkan
                } else if node.capabilities.llama_cpp_backends.contains(&"metal".to_string()) {
                    ExecutionStrategy::LlamaCppMetal
                } else if node.capabilities.llama_cpp_backends.contains(&"rocm".to_string()) {
                    ExecutionStrategy::LlamaCppROCm
                } else if node.capabilities.cuda && request.requires_proprietary {
                    ExecutionStrategy::CudaIsolated {
                        warning: "🚨 Using proprietary CUDA - migration to open standards recommended"
                            .to_string(),
                    }
                } else {
                    ExecutionStrategy::OptimizedCPU
                }
            }
            "onnx" => {
                if node.capabilities.onnx_providers.contains(&"vulkan".to_string()) {
                    ExecutionStrategy::ONNXVulkan
                } else if node.capabilities.onnx_providers.contains(&"rocm".to_string()) {
                    ExecutionStrategy::ONNXROCm
                } else {
                    ExecutionStrategy::OptimizedCPU
                }
            }
            "candle" => {
                if node.capabilities.webgpu {
                    ExecutionStrategy::CandleWGPU
                } else {
                    ExecutionStrategy::OptimizedCPU
                }
            }
            "burn" => {
                if node.capabilities.webgpu {
                    ExecutionStrategy::BurnWGPU
                } else {
                    ExecutionStrategy::OptimizedCPU
                }
            }
            _ => ExecutionStrategy::OptimizedCPU,
        },
        WorkloadType::GeneralCompute { .. } => {
            if node.capabilities.webgpu {
                ExecutionStrategy::WebGPU
            } else if node.capabilities.vulkan {
                ExecutionStrategy::Vulkan
            } else if node.capabilities.rocm {
                ExecutionStrategy::ROCm
            } else {
                ExecutionStrategy::OptimizedCPU
            }
        }
        WorkloadType::ScientificComputing { .. } => {
            if request.requires_proprietary && node.capabilities.cuda {
                ExecutionStrategy::CudaIsolated {
                    warning: "🔒 Legacy application locked to CUDA - consider open alternative migration"
                        .to_string(),
                }
            } else if node.capabilities.vulkan {
                ExecutionStrategy::Vulkan
            } else if node.capabilities.rocm {
                ExecutionStrategy::ROCm
            } else {
                ExecutionStrategy::OptimizedCPU
            }
        }
        WorkloadType::MediaProcessing { .. } => ExecutionStrategy::OptimizedCPU,
    }
}

fn generate_reasoning(node: &ComputeNode, strategy: &ExecutionStrategy) -> String {
    match strategy {
        ExecutionStrategy::WebGPU => {
            format!("✅ WebGPU on {} - Universal standard that works everywhere", node.id)
        }
        ExecutionStrategy::Vulkan => {
            format!("✅ Vulkan on {} - Industry-standard open GPU API", node.id)
        }
        ExecutionStrategy::ROCm => format!(
            "⭐ ROCm on {} - Open alternative breaking CUDA dependency",
            node.id
        ),
        ExecutionStrategy::LlamaCppVulkan => format!(
            "🌟 llama.cpp + Vulkan on {} - Open AI + open GPU = freedom",
            node.id
        ),
        ExecutionStrategy::LlamaCppMetal => format!(
            "🍎 llama.cpp + Metal on {} - Optimized for Apple Silicon",
            node.id
        ),
        ExecutionStrategy::LlamaCppROCm => format!(
            "🔥 llama.cpp + ROCm on {} - Pure open source AI stack",
            node.id
        ),
        ExecutionStrategy::ONNXVulkan => format!(
            "✅ ONNX + Vulkan on {} - Cross-platform AI standard + open GPU",
            node.id
        ),
        ExecutionStrategy::ONNXROCm => format!(
            "⭐ ONNX + ROCm on {} - Open AI standard + CUDA alternative",
            node.id
        ),
        ExecutionStrategy::CandleWGPU => format!(
            "🦀 Candle + WebGPU on {} - Pure Rust ML with universal GPU",
            node.id
        ),
        ExecutionStrategy::BurnWGPU => format!(
            "🔥 Burn + WebGPU on {} - Next-gen Rust ML framework",
            node.id
        ),
        ExecutionStrategy::OptimizedCPU => {
            format!("💻 Optimized CPU on {} - Universal compatibility", node.id)
        }
        ExecutionStrategy::CudaIsolated { warning } => {
            format!("⚠️ CUDA on {} (ISOLATED) - {}", node.id, warning)
        }
    }
}
