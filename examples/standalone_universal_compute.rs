// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::struct_excessive_bools,
    clippy::unused_async,
    dead_code,
    unused_variables
)]
// Standalone Universal Compute Demonstration
// ToadStool's Open-First Strategy: Isolate → Abstract → Incentivize
//
// This demonstrates ToadStool's approach to breaking CUDA monopoly:
// 1. ISOLATE: Treat proprietary tech as specialized islands
// 2. ABSTRACT: Champion open alternatives (WebGPU, ROCm, llama.cpp)
// 3. INCENTIVIZE: Create pressure for open standards adoption

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🍄 ToadStool Universal Compute Demo: Breaking the CUDA Monopoly");
    println!("===============================================================");
    println!("Strategy: Isolate → Abstract → Incentivize");
    println!();

    // Step 1: Create diverse compute ecosystem
    let ecosystem = create_diverse_ecosystem().await;
    print_ecosystem_overview(&ecosystem).await;

    // Step 2: Demonstrate workload scheduling with open preferences
    demonstrate_ai_workloads(&ecosystem).await?;
    demonstrate_general_compute(&ecosystem).await?;
    demonstrate_cuda_isolation(&ecosystem).await?;

    // Step 3: Show strategic impact
    show_strategic_metrics(&ecosystem).await?;

    println!("\n🎯 Mission Accomplished!");
    println!("✅ Open standards prioritized over proprietary lock-in");
    println!("✅ Cross-platform compatibility maximized");
    println!("✅ Community frameworks championed");
    println!("✅ NVIDIA incentivized to join the open ecosystem");
    println!("\n💡 The message to NVIDIA: 'Your hardware is amazing. Your drivers could power the world.'");

    Ok(())
}

#[derive(Debug, Clone)]
struct ComputeNode {
    id: String,
    node_type: String,
    capabilities: NodeCapabilities,
    performance_score: f64,
}

#[derive(Debug, Clone)]
struct NodeCapabilities {
    // Open compute backends (strongly preferred)
    webgpu: bool,
    vulkan: bool,
    rocm: bool,
    opencl: bool,
    metal: bool,

    // Proprietary capabilities (isolated)
    cuda: bool,
    oneapi: bool,

    // AI frameworks with backend support
    llama_cpp_backends: Vec<String>,
    onnx_providers: Vec<String>,
    candle_support: bool,
    burn_support: bool,

    // Cross-platform features
    wasm_support: bool,
    edge_optimized: bool,
}

#[derive(Debug)]
struct WorkloadRequest {
    name: String,
    workload_type: WorkloadType,
    requires_proprietary: bool,
    preferred_frameworks: Vec<String>,
}

#[derive(Debug)]
enum WorkloadType {
    AiInference { model: String, framework: String },
    GeneralCompute { parallel: bool },
    MediaProcessing { codec: String },
    ScientificComputing { domain: String },
}

#[derive(Debug)]
struct SchedulingDecision {
    target_node: String,
    strategy: ExecutionStrategy,
    score: f64,
    reasoning: String,
    open_score_bonus: f64,
}

#[derive(Debug)]
enum ExecutionStrategy {
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

async fn create_diverse_ecosystem() -> Vec<ComputeNode> {
    vec![
        // Gaming PC with RTX 4080 - mixed ecosystem
        ComputeNode {
            id: "gaming-rtx4080".to_string(),
            node_type: "Gaming PC".to_string(),
            capabilities: NodeCapabilities {
                webgpu: true,
                vulkan: true,
                rocm: false,
                opencl: true,
                metal: false,
                cuda: true, // Has CUDA but also open alternatives
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
        // AMD RX 7900 XT workstation - open source champion
        ComputeNode {
            id: "amd-rx7900xt".to_string(),
            node_type: "AMD Workstation".to_string(),
            capabilities: NodeCapabilities {
                webgpu: true,
                vulkan: true,
                rocm: true, // AMD's open alternative to CUDA
                opencl: true,
                metal: false,
                cuda: false, // Pure open stack!
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
            performance_score: 9200.0, // Higher due to open optimization
        },
        // Apple M3 Max - Metal + emerging frameworks
        ComputeNode {
            id: "apple-m3-max".to_string(),
            node_type: "Apple Silicon".to_string(),
            capabilities: NodeCapabilities {
                webgpu: true,
                vulkan: false, // Apple doesn't support Vulkan
                rocm: false,
                opencl: false, // Deprecated on macOS
                metal: true,   // Apple's GPU framework
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
        // Raspberry Pi 5 - Edge WebGPU champion
        ComputeNode {
            id: "raspberry-pi-5".to_string(),
            node_type: "Edge Device".to_string(),
            capabilities: NodeCapabilities {
                webgpu: true, // The universal standard
                vulkan: true, // Mali GPU supports Vulkan
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
        // Legacy CUDA-only workstation
        ComputeNode {
            id: "legacy-cuda-only".to_string(),
            node_type: "Legacy Workstation".to_string(),
            capabilities: NodeCapabilities {
                webgpu: false,
                vulkan: false,
                rocm: false,
                opencl: false,
                metal: false,
                cuda: true, // CUDA-only - the problem case
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

async fn print_ecosystem_overview(ecosystem: &[ComputeNode]) {
    println!("🌐 Diverse Compute Ecosystem");
    println!("============================");

    for node in ecosystem {
        println!(
            "📱 {} ({}) - Score: {:.0}",
            node.id, node.node_type, node.performance_score
        );

        let open_backends: Vec<String> = vec![
            if node.capabilities.webgpu {
                Some("WebGPU")
            } else {
                None
            },
            if node.capabilities.vulkan {
                Some("Vulkan")
            } else {
                None
            },
            if node.capabilities.rocm {
                Some("ROCm")
            } else {
                None
            },
            if node.capabilities.metal {
                Some("Metal")
            } else {
                None
            },
            if node.capabilities.opencl {
                Some("OpenCL")
            } else {
                None
            },
        ]
        .into_iter()
        .flatten()
        .map(std::string::ToString::to_string)
        .collect();

        let proprietary: Vec<String> = vec![
            if node.capabilities.cuda {
                Some("CUDA")
            } else {
                None
            },
            if node.capabilities.oneapi {
                Some("OneAPI")
            } else {
                None
            },
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

        // Show AI framework support
        let ai_frameworks: Vec<String> = vec![
            if node.capabilities.llama_cpp_backends.is_empty() {
                None
            } else {
                Some("llama.cpp")
            },
            if node.capabilities.onnx_providers.is_empty() {
                None
            } else {
                Some("ONNX")
            },
            if node.capabilities.candle_support {
                Some("Candle")
            } else {
                None
            },
            if node.capabilities.burn_support {
                Some("Burn")
            } else {
                None
            },
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

async fn demonstrate_ai_workloads(
    ecosystem: &[ComputeNode],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 AI Inference Workloads - Open Framework Priority");
    println!("===================================================");

    // Llama 2 inference - champion of open AI
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
    let decision = schedule_workload(&llama_request, ecosystem).await;
    print_decision(&decision);

    // ONNX model - cross-platform AI standard
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
    let decision = schedule_workload(&onnx_request, ecosystem).await;
    print_decision(&decision);

    // Rust-native ML with Candle
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
    let decision = schedule_workload(&candle_request, ecosystem).await;
    print_decision(&decision);

    Ok(())
}

async fn demonstrate_general_compute(
    ecosystem: &[ComputeNode],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚡ General Compute Workloads - WebGPU Universal Standard");
    println!("========================================================");

    let webgpu_request = WorkloadRequest {
        name: "Matrix Multiplication (WebGPU)".to_string(),
        workload_type: WorkloadType::GeneralCompute { parallel: true },
        requires_proprietary: false,
        preferred_frameworks: vec!["webgpu".to_string(), "wgpu".to_string()],
    };

    println!("🌐 Scheduling: {}", webgpu_request.name);
    let decision = schedule_workload(&webgpu_request, ecosystem).await;
    print_decision(&decision);

    Ok(())
}

async fn demonstrate_cuda_isolation(
    ecosystem: &[ComputeNode],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔒 CUDA Isolation Demo - The Proprietary Island");
    println!("===============================================");

    let cuda_only_request = WorkloadRequest {
        name: "Legacy CUDA-Only Application".to_string(),
        workload_type: WorkloadType::ScientificComputing {
            domain: "legacy-molecular-simulation".to_string(),
        },
        requires_proprietary: true, // Forces CUDA usage
        preferred_frameworks: vec!["cuda".to_string()],
    };

    println!("🚨 Scheduling: {}", cuda_only_request.name);
    let decision = schedule_workload(&cuda_only_request, ecosystem).await;
    print_decision(&decision);

    Ok(())
}

async fn schedule_workload(
    request: &WorkloadRequest,
    ecosystem: &[ComputeNode],
) -> SchedulingDecision {
    let mut scored_nodes = Vec::new();

    for node in ecosystem {
        let (score, open_bonus) = calculate_node_score(request, node).await;
        scored_nodes.push((node, score, open_bonus));
    }

    // Sort by score (open solutions get massive preference)
    scored_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let (best_node, best_score, open_bonus) = scored_nodes.first().unwrap();
    let strategy = determine_execution_strategy(request, best_node);
    let reasoning = generate_reasoning(request, best_node, &strategy);

    SchedulingDecision {
        target_node: best_node.id.clone(),
        strategy,
        score: *best_score,
        reasoning,
        open_score_bonus: *open_bonus,
    }
}

async fn calculate_node_score(request: &WorkloadRequest, node: &ComputeNode) -> (f64, f64) {
    let mut score = node.performance_score;
    let mut open_bonus = 0.0;

    // MASSIVE bonuses for open standards (this is our weapon against CUDA)
    if node.capabilities.webgpu {
        open_bonus += 800.0; // WebGPU gets the highest bonus - universal standard
        score += 800.0;
    }
    if node.capabilities.vulkan {
        open_bonus += 600.0; // Vulkan - industry standard
        score += 600.0;
    }
    if node.capabilities.rocm {
        open_bonus += 700.0; // ROCm gets extra for directly competing with CUDA
        score += 700.0;
    }
    if node.capabilities.opencl {
        open_bonus += 400.0; // OpenCL - mature but older
        score += 400.0;
    }
    if node.capabilities.metal {
        open_bonus += 500.0; // Metal - Apple's open framework
        score += 500.0;
    }

    // HUGE bonuses for open AI frameworks (community champions)
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

    // Rust-native ML frameworks get innovation bonus
    if node.capabilities.candle_support {
        open_bonus += 600.0;
        score += 600.0;
    }
    if node.capabilities.burn_support {
        open_bonus += 550.0;
        score += 550.0;
    }

    // Cross-platform and edge bonuses
    if node.capabilities.wasm_support {
        open_bonus += 300.0;
        score += 300.0;
    }
    if node.capabilities.edge_optimized {
        open_bonus += 250.0;
        score += 250.0;
    }

    // HEAVY penalties for proprietary lock-in (unless absolutely required)
    if !request.requires_proprietary {
        if node.capabilities.cuda && !has_open_alternatives(node) {
            score -= 2000.0; // Massive penalty for CUDA-only
        } else if node.capabilities.cuda {
            score -= 300.0; // Light penalty if open alternatives exist
        }
    }

    (score, open_bonus)
}

fn has_open_alternatives(node: &ComputeNode) -> bool {
    node.capabilities.webgpu
        || node.capabilities.vulkan
        || node.capabilities.rocm
        || node.capabilities.metal
        || node.capabilities.opencl
}

fn determine_execution_strategy(
    request: &WorkloadRequest,
    node: &ComputeNode,
) -> ExecutionStrategy {
    match &request.workload_type {
        WorkloadType::AiInference { framework, .. } => {
            match framework.as_str() {
                "llama.cpp" => {
                    // Strongly prefer open backends for llama.cpp
                    if node
                        .capabilities
                        .llama_cpp_backends
                        .contains(&"vulkan".to_string())
                    {
                        ExecutionStrategy::LlamaCppVulkan
                    } else if node
                        .capabilities
                        .llama_cpp_backends
                        .contains(&"metal".to_string())
                    {
                        ExecutionStrategy::LlamaCppMetal
                    } else if node
                        .capabilities
                        .llama_cpp_backends
                        .contains(&"rocm".to_string())
                    {
                        ExecutionStrategy::LlamaCppROCm
                    } else if node.capabilities.cuda && request.requires_proprietary {
                        ExecutionStrategy::CudaIsolated {
                            warning: "🚨 Using proprietary CUDA - migration to open standards recommended".to_string() 
                        }
                    } else {
                        ExecutionStrategy::OptimizedCPU
                    }
                }
                "onnx" => {
                    // Prefer open providers for ONNX
                    if node
                        .capabilities
                        .onnx_providers
                        .contains(&"vulkan".to_string())
                    {
                        ExecutionStrategy::ONNXVulkan
                    } else if node
                        .capabilities
                        .onnx_providers
                        .contains(&"rocm".to_string())
                    {
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
            }
        }
        WorkloadType::GeneralCompute { .. } => {
            // WebGPU first for general compute - the universal standard
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
                    warning:
                        "🔒 Legacy application locked to CUDA - consider open alternative migration"
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

fn generate_reasoning(
    _request: &WorkloadRequest,
    node: &ComputeNode,
    strategy: &ExecutionStrategy,
) -> String {
    match strategy {
        ExecutionStrategy::WebGPU => format!(
            "✅ WebGPU on {} - Universal standard that works everywhere",
            node.id
        ),
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

async fn show_strategic_metrics(
    ecosystem: &[ComputeNode],
) -> Result<(), Box<dyn std::error::Error>> {
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

    // Calculate potential impact
    let potential_open_nodes = cuda_only_nodes + mixed_nodes;
    let market_expansion = potential_open_nodes as f64 * 1000.0; // Rough estimate of gaming PCs per enterprise node

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

    Ok(())
}
