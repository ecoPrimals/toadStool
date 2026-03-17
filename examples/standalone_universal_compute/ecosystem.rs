// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::nursery, dead_code)]
//! Ecosystem creation and display

use super::types::{ComputeNode, NodeCapabilities};

pub fn create_diverse_ecosystem() -> Vec<ComputeNode> {
    vec![
        ComputeNode {
            id: "gaming-rtx4080".to_string(),
            node_type: "Gaming PC".to_string(),
            capabilities: NodeCapabilities {
                webgpu: true,
                vulkan: true,
                rocm: false,
                opencl: true,
                metal: false,
                cuda: true,
                oneapi: false,
                llama_cpp_backends: vec![
                    "cpu".to_string(),
                    "vulkan".to_string(),
                    "cuda".to_string(),
                ],
                onnx_providers: vec!["cpu".to_string(), "vulkan".to_string(), "cuda".to_string()],
                candle_support: true,
                burn_support: true,
                wasm_support: true,
                edge_optimized: false,
            },
            performance_score: 8500.0,
        },
        ComputeNode {
            id: "amd-rx7900xt".to_string(),
            node_type: "AMD Workstation".to_string(),
            capabilities: NodeCapabilities {
                webgpu: true,
                vulkan: true,
                rocm: true,
                opencl: true,
                metal: false,
                cuda: false,
                oneapi: false,
                llama_cpp_backends: vec![
                    "cpu".to_string(),
                    "vulkan".to_string(),
                    "rocm".to_string(),
                ],
                onnx_providers: vec!["cpu".to_string(), "vulkan".to_string(), "rocm".to_string()],
                candle_support: true,
                burn_support: true,
                wasm_support: true,
                edge_optimized: false,
            },
            performance_score: 9200.0,
        },
        ComputeNode {
            id: "apple-m3-max".to_string(),
            node_type: "Apple Silicon".to_string(),
            capabilities: NodeCapabilities {
                webgpu: true,
                vulkan: false,
                rocm: false,
                opencl: false,
                metal: true,
                cuda: false,
                oneapi: false,
                llama_cpp_backends: vec!["cpu".to_string(), "metal".to_string()],
                onnx_providers: vec!["cpu".to_string(), "coreml".to_string()],
                candle_support: true,
                burn_support: true,
                wasm_support: true,
                edge_optimized: true,
            },
            performance_score: 7800.0,
        },
        ComputeNode {
            id: "raspberry-pi-5".to_string(),
            node_type: "Edge Device".to_string(),
            capabilities: NodeCapabilities {
                webgpu: true,
                vulkan: true,
                rocm: false,
                opencl: true,
                metal: false,
                cuda: false,
                oneapi: false,
                llama_cpp_backends: vec!["cpu".to_string()],
                onnx_providers: vec!["cpu".to_string(), "webgpu".to_string()],
                candle_support: false,
                burn_support: false,
                wasm_support: true,
                edge_optimized: true,
            },
            performance_score: 1200.0,
        },
        ComputeNode {
            id: "legacy-cuda-only".to_string(),
            node_type: "Legacy Workstation".to_string(),
            capabilities: NodeCapabilities {
                webgpu: false,
                vulkan: false,
                rocm: false,
                opencl: false,
                metal: false,
                cuda: true,
                oneapi: false,
                llama_cpp_backends: vec!["cpu".to_string(), "cuda".to_string()],
                onnx_providers: vec!["cpu".to_string(), "cuda".to_string()],
                candle_support: false,
                burn_support: false,
                wasm_support: false,
                edge_optimized: false,
            },
            performance_score: 7000.0,
        },
    ]
}

pub fn print_ecosystem_overview(ecosystem: &[ComputeNode]) {
    println!("🌐 Diverse Compute Ecosystem");
    println!("============================");

    for node in ecosystem {
        println!(
            "📱 {} ({}) - Score: {:.0}",
            node.id, node.node_type, node.performance_score
        );

        let open_backends: Vec<String> = [
            node.capabilities.webgpu.then_some("WebGPU"),
            node.capabilities.vulkan.then_some("Vulkan"),
            node.capabilities.rocm.then_some("ROCm"),
            node.capabilities.metal.then_some("Metal"),
            node.capabilities.opencl.then_some("OpenCL"),
        ]
        .into_iter()
        .flatten()
        .map(std::string::ToString::to_string)
        .collect();

        let proprietary: Vec<String> = [
            node.capabilities.cuda.then_some("CUDA"),
            node.capabilities.oneapi.then_some("OneAPI"),
        ]
        .into_iter()
        .flatten()
        .map(std::string::ToString::to_string)
        .collect();

        println!(
            "   🌟 Open Standards: {}",
            if open_backends.is_empty() {
                "❌ None".to_string()
            } else {
                format!("✅ {}", open_backends.join(", "))
            }
        );
        println!(
            "   🔒 Proprietary: {}",
            if proprietary.is_empty() {
                "✅ None".to_string()
            } else {
                format!("⚠️ {}", proprietary.join(", "))
            }
        );

        let ai_frameworks: Vec<String> = [
            (!node.capabilities.llama_cpp_backends.is_empty()).then_some("llama.cpp"),
            (!node.capabilities.onnx_providers.is_empty()).then_some("ONNX"),
            node.capabilities.candle_support.then_some("Candle"),
            node.capabilities.burn_support.then_some("Burn"),
        ]
        .into_iter()
        .flatten()
        .map(std::string::ToString::to_string)
        .collect();

        println!(
            "   🤖 AI Frameworks: {}",
            if ai_frameworks.is_empty() {
                "None".to_string()
            } else {
                ai_frameworks.join(", ")
            }
        );
        println!();
    }
}
