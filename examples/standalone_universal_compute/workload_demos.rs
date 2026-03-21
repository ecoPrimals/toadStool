// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::nursery, dead_code)]
//! Workload demonstration functions

use super::scheduling::{has_open_alternatives, schedule_workload};
use super::types::{
    ComputeNode, ExecutionStrategy, SchedulingDecision, WorkloadRequest, WorkloadType,
};

pub fn demonstrate_ai_workloads(ecosystem: &[ComputeNode]) {
    println!("🤖 AI Inference Workloads - Open Framework Priority");
    println!("===================================================");

    let llama_request = WorkloadRequest {
        name: "Llama 2 7B Chat Inference".to_string(),
        workload_type: WorkloadType::AiInference {
            model: "llama2-7b-chat".to_string(),
            framework: "llama.cpp".to_string(),
        },
        requires_proprietary: false,
        preferred_frameworks: vec![
            "vulkan".to_string(),
            "rocm".to_string(),
            "metal".to_string(),
        ],
    };
    println!("🦙 Scheduling: {}", llama_request.name);
    let decision = schedule_workload(&llama_request, ecosystem);
    print_decision(&decision);

    let onnx_request = WorkloadRequest {
        name: "BERT Base ONNX Inference".to_string(),
        workload_type: WorkloadType::AiInference {
            model: "bert-base-uncased".to_string(),
            framework: "onnx".to_string(),
        },
        requires_proprietary: false,
        preferred_frameworks: vec!["vulkan".to_string(), "rocm".to_string()],
    };
    println!("\n🔥 Scheduling: {}", onnx_request.name);
    let decision = schedule_workload(&onnx_request, ecosystem);
    print_decision(&decision);

    let candle_request = WorkloadRequest {
        name: "Rust ML with Candle".to_string(),
        workload_type: WorkloadType::AiInference {
            model: "custom-rust-model".to_string(),
            framework: "candle".to_string(),
        },
        requires_proprietary: false,
        preferred_frameworks: vec!["webgpu".to_string(), "wgpu".to_string()],
    };
    println!("\n🦀 Scheduling: {}", candle_request.name);
    let decision = schedule_workload(&candle_request, ecosystem);
    print_decision(&decision);
}

pub fn demonstrate_general_compute(ecosystem: &[ComputeNode]) {
    println!("\n⚡ General Compute Workloads - WebGPU Universal Standard");
    println!("========================================================");

    let webgpu_request = WorkloadRequest {
        name: "Matrix Multiplication (WebGPU)".to_string(),
        workload_type: WorkloadType::GeneralCompute { parallel: true },
        requires_proprietary: false,
        preferred_frameworks: vec!["webgpu".to_string(), "wgpu".to_string()],
    };
    println!("🌐 Scheduling: {}", webgpu_request.name);
    let decision = schedule_workload(&webgpu_request, ecosystem);
    print_decision(&decision);
}

pub fn demonstrate_cuda_isolation(ecosystem: &[ComputeNode]) {
    println!("\n🔒 CUDA Isolation Demo - The Proprietary Island");
    println!("===============================================");

    let cuda_only_request = WorkloadRequest {
        name: "Legacy CUDA-Only Application".to_string(),
        workload_type: WorkloadType::ScientificComputing {
            domain: "legacy-molecular-simulation".to_string(),
        },
        requires_proprietary: true,
        preferred_frameworks: vec!["cuda".to_string()],
    };
    println!("🚨 Scheduling: {}", cuda_only_request.name);
    let decision = schedule_workload(&cuda_only_request, ecosystem);
    print_decision(&decision);
}

fn print_decision(decision: &SchedulingDecision) {
    println!("   🎯 Strategy: {:?}", decision.strategy);
    println!("   📊 Final Score: {:.1}", decision.score);
    println!("   🌟 Open Bonus: +{:.1}", decision.open_score_bonus);
    println!("   💭 Reasoning: {}", decision.reasoning);

    match &decision.strategy {
        ExecutionStrategy::CudaIsolated { .. } => {
            println!("   💡 Action Item: Migrate to open alternatives for ecosystem benefits");
            println!("   🔄 Migration Path: CUDA → Vulkan/ROCm + llama.cpp/ONNX");
        }
        ExecutionStrategy::LlamaCppVulkan | ExecutionStrategy::ONNXVulkan => {
            println!("   🏆 Excellence: Open AI framework + open GPU standard");
        }
        ExecutionStrategy::LlamaCppROCm | ExecutionStrategy::ONNXROCm => {
            println!("   🥇 Outstanding: Breaking CUDA monopoly with open alternatives");
        }
        ExecutionStrategy::WebGPU | ExecutionStrategy::CandleWGPU | ExecutionStrategy::BurnWGPU => {
            println!("   ⭐ Perfect: Universal standard - works on every device");
        }
        _ => {}
    }
}

pub fn show_strategic_metrics(ecosystem: &[ComputeNode]) {
    println!("\n📊 Strategic Impact Analysis");
    println!("============================");

    let total_nodes = ecosystem.len();
    let webgpu_nodes = ecosystem.iter().filter(|n| n.capabilities.webgpu).count();
    let vulkan_nodes = ecosystem.iter().filter(|n| n.capabilities.vulkan).count();
    let rocm_nodes = ecosystem.iter().filter(|n| n.capabilities.rocm).count();
    let cuda_only_nodes = ecosystem
        .iter()
        .filter(|n| n.capabilities.cuda && !has_open_alternatives(n))
        .count();
    let mixed_nodes = ecosystem
        .iter()
        .filter(|n| n.capabilities.cuda && has_open_alternatives(n))
        .count();
    let pure_open_nodes = ecosystem
        .iter()
        .filter(|n| !n.capabilities.cuda && has_open_alternatives(n))
        .count();

    println!("🌐 Node Distribution:");
    println!(
        "   WebGPU capable: {}/{} ({:.1}%)",
        webgpu_nodes,
        total_nodes,
        webgpu_nodes as f64 / total_nodes as f64 * 100.0
    );
    println!(
        "   Vulkan capable: {}/{} ({:.1}%)",
        vulkan_nodes,
        total_nodes,
        vulkan_nodes as f64 / total_nodes as f64 * 100.0
    );
    println!(
        "   ROCm capable: {}/{} ({:.1}%)",
        rocm_nodes,
        total_nodes,
        rocm_nodes as f64 / total_nodes as f64 * 100.0
    );
    println!(
        "   Pure open source: {}/{} ({:.1}%)",
        pure_open_nodes,
        total_nodes,
        pure_open_nodes as f64 / total_nodes as f64 * 100.0
    );
    println!(
        "   Mixed (CUDA + Open): {}/{} ({:.1}%)",
        mixed_nodes,
        total_nodes,
        mixed_nodes as f64 / total_nodes as f64 * 100.0
    );
    println!(
        "   CUDA-only (problematic): {}/{} ({:.1}%)",
        cuda_only_nodes,
        total_nodes,
        cuda_only_nodes as f64 / total_nodes as f64 * 100.0
    );

    println!("\n🤖 AI Framework Diversity:");
    let llama_cpp_nodes = ecosystem
        .iter()
        .filter(|n| !n.capabilities.llama_cpp_backends.is_empty())
        .count();
    let onnx_nodes = ecosystem
        .iter()
        .filter(|n| !n.capabilities.onnx_providers.is_empty())
        .count();
    let rust_ml_nodes = ecosystem
        .iter()
        .filter(|n| n.capabilities.candle_support || n.capabilities.burn_support)
        .count();

    println!(
        "   llama.cpp support: {}/{} ({:.1}%)",
        llama_cpp_nodes,
        total_nodes,
        llama_cpp_nodes as f64 / total_nodes as f64 * 100.0
    );
    println!(
        "   ONNX support: {}/{} ({:.1}%)",
        onnx_nodes,
        total_nodes,
        onnx_nodes as f64 / total_nodes as f64 * 100.0
    );
    println!(
        "   Rust ML frameworks: {}/{} ({:.1}%)",
        rust_ml_nodes,
        total_nodes,
        rust_ml_nodes as f64 / total_nodes as f64 * 100.0
    );

    println!("\n🎯 Strategic Success Metrics:");
    let open_coverage =
        (webgpu_nodes + vulkan_nodes + rocm_nodes) as f64 / (total_nodes * 3) as f64 * 100.0;
    let cuda_dependency = (cuda_only_nodes + mixed_nodes) as f64 / total_nodes as f64 * 100.0;
    let cross_platform = ecosystem
        .iter()
        .filter(|n| n.capabilities.wasm_support)
        .count() as f64
        / total_nodes as f64
        * 100.0;

    println!("   ✅ Open Standards Coverage: {open_coverage:.1}%");
    println!("   ⚠️ CUDA Dependency: {cuda_dependency:.1}% (goal: minimize)");
    println!("   🌐 Cross-Platform Ready: {cross_platform:.1}%");

    let ecosystem_health = if cuda_only_nodes == 0 {
        "🌟 Excellent"
    } else if cuda_only_nodes == 1 {
        "✅ Good"
    } else {
        "⚠️ Needs Work"
    };
    println!("   🏥 Ecosystem Health: {ecosystem_health}");

    println!("\n💡 The NVIDIA Proposition:");
    println!("   Current Reality:");
    println!("     • {mixed_nodes} NVIDIA nodes have both CUDA and open alternatives");
    println!("     • {cuda_only_nodes} nodes are trapped in CUDA-only ecosystem");
    println!("     • Millions of NVIDIA gaming GPUs sit idle while AI needs compute");
    println!();
    println!("   The Opportunity:");
    println!("     • Open NVIDIA drivers = instant access to global federated compute");
    println!("     • Every gaming PC becomes an AI inference node");
    println!("     • First-mover advantage in the distributed AI economy");
    println!("     • Hardware sales increase as idle GPUs get monetized");
    println!();
    println!("   The Message:");
    println!("     📢 'NVIDIA: Your silicon is revolutionary. Your openness could be too.'");

    println!("\n🚀 Future Vision:");
    println!("   Today: Fragmented ecosystem with proprietary islands");
    println!("   Tomorrow: Universal compute network with open standards");
    println!("   End Goal: Every GPU contributes to global AI progress");
    println!("   The Winner: Whoever enables this transformation first");

    let potential_open_nodes = cuda_only_nodes + mixed_nodes;
    let market_expansion = potential_open_nodes as f64 * 1000.0;

    println!("\n📈 Market Impact Projection:");
    println!(
        "   Current addressable nodes: {}",
        pure_open_nodes + mixed_nodes
    );
    println!("   Potential with open NVIDIA drivers: {total_nodes}");
    println!(
        "   Estimated gaming PC unlock: {:.0}k+ nodes",
        market_expansion / 1000.0
    );
    println!(
        "   🎯 Network effect: More nodes → better load balancing → happier users → more adoption"
    );
}
