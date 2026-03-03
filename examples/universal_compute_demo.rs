// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code)]
#![allow(unused_variables)]
// Universal Compute Demonstration - ToadStool's Open-First Strategy
//
// This demo showcases ToadStool's approach to universal compute:
// 1. ISOLATE: Treat proprietary tech (CUDA) as specialized islands
// 2. ABSTRACT: Champion open alternatives (WebGPU, ROCm, llama.cpp)
// 3. INCENTIVIZE: Create leverage for open standards adoption
//
// The goal: Be seamless, agnostic, and functional with MOST of the market

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🍄 ToadStool Universal Compute Demo: Open-First Strategy");
    println!("======================================================");
    println!("Demonstrating: Isolate → Abstract → Incentivize");
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

    println!("\n🎯 Universal Compute Strategy Complete!");
    println!("✅ Open standards prioritized over proprietary");
    println!("✅ Cross-platform compatibility maximized");
    println!("✅ Community frameworks championed");
    println!("✅ NVIDIA incentivized to open their ecosystem");

    Ok(())
}

#[derive(Debug, Clone)]
struct ComputeNode {
    id: String,
    capabilities: NodeCapabilities,
    performance_score: f64,
}

#[derive(Debug, Clone)]
struct NodeCapabilities {
    // Open compute backends (preferred)
    webgpu: bool,
    vulkan: bool,
    rocm: bool,
    opencl: bool,
    metal: bool,

    // Proprietary capabilities (isolated)
    cuda: bool,
    oneapi: bool,

    // AI frameworks
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
    #[allow(dead_code)]
    preferred_frameworks: Vec<String>,
}

#[derive(Debug)]
enum WorkloadType {
    AiInference {
        #[allow(dead_code)]
        model: String,
        framework: String,
    },
    GeneralCompute {
        #[allow(dead_code)]
        parallel: bool,
    },
    #[allow(dead_code)]
    MediaProcessing { codec: String },
    ScientificComputing {
        #[allow(dead_code)]
        domain: String,
    },
}

#[derive(Debug)]
struct SchedulingDecision {
    #[allow(dead_code)]
    target_node: String,
    strategy: ExecutionStrategy,
    score: f64,
    reasoning: String,
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
    OptimizedCPU,
    CudaIsolated { warning: String },
}

async fn create_diverse_ecosystem() -> Vec<ComputeNode> {
    vec![
        // Gaming PC with RTX 4080 - has both open and proprietary
        ComputeNode {
            id: "gaming-rtx4080".to_string(),
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
        // AMD RX 7900 XT workstation - pure open source champion
        ComputeNode {
            id: "amd-rx7900xt".to_string(),
            capabilities: NodeCapabilities {
                webgpu: true,
                vulkan: true,
                rocm: true,
                opencl: true,
                metal: false,
                cuda: false, // No CUDA!
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
            performance_score: 9200.0, // Slightly higher due to open optimization
        },
        // Apple M3 Max - Metal + emerging Rust frameworks
        ComputeNode {
            id: "apple-m3-max".to_string(),
            capabilities: NodeCapabilities {
                webgpu: true,
                vulkan: false, // Apple doesn't support Vulkan
                rocm: false,
                opencl: false, // Deprecated on macOS
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
        // Raspberry Pi 5 - Edge WebGPU champion
        ComputeNode {
            id: "raspberry-pi-5".to_string(),
            capabilities: NodeCapabilities {
                webgpu: true,
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
    ]
}

async fn print_ecosystem_overview(ecosystem: &[ComputeNode]) {
    println!("🌐 Diverse Compute Ecosystem");
    println!("============================");
    for node in ecosystem {
        println!("  {} (Score: {:.0})", node.id, node.performance_score);
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
        ]
        .into_iter()
        .flatten()
        .map(|s| s.to_string())
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
        .map(|s| s.to_string())
        .collect();

        println!(
            "    Open: {}",
            if open_backends.is_empty() {
                "None".to_string()
            } else {
                open_backends.join(", ")
            }
        );
        println!(
            "    Proprietary: {}",
            if proprietary.is_empty() {
                "None".to_string()
            } else {
                proprietary.join(", ")
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

    // Llama 2 inference - should prefer open backends
    let llama_request = WorkloadRequest {
        name: "Llama 2 7B Inference".to_string(),
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

    // ONNX model - cross-platform standard
    let onnx_request = WorkloadRequest {
        name: "BERT Base ONNX".to_string(),
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

    Ok(())
}

async fn demonstrate_general_compute(
    ecosystem: &[ComputeNode],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚡ General Compute Workloads - WebGPU First");
    println!("==========================================");

    let webgpu_request = WorkloadRequest {
        name: "Matrix Multiplication".to_string(),
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
    println!("\n🔒 CUDA Isolation Demo - Proprietary Island");
    println!("==========================================");

    let cuda_only_request = WorkloadRequest {
        name: "Legacy CUDA Application".to_string(),
        workload_type: WorkloadType::ScientificComputing {
            domain: "legacy-simulation".to_string(),
        },
        requires_proprietary: true, // This forces CUDA
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
        let score = calculate_node_score(request, node).await;
        scored_nodes.push((node, score));
    }

    // Sort by score (open solutions get higher scores)
    scored_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let (best_node, best_score) = scored_nodes.first().unwrap();
    let strategy = determine_execution_strategy(request, best_node);
    let reasoning = generate_reasoning(request, best_node, &strategy);

    SchedulingDecision {
        target_node: best_node.id.clone(),
        strategy,
        score: *best_score,
        reasoning,
    }
}

async fn calculate_node_score(request: &WorkloadRequest, node: &ComputeNode) -> f64 {
    let mut score = node.performance_score;

    // Open standards bonus (these get HUGE preference)
    if node.capabilities.webgpu {
        score += 500.0;
    } // WebGPU gets highest bonus - universal
    if node.capabilities.vulkan {
        score += 400.0;
    } // Vulkan - industry standard
    if node.capabilities.rocm {
        score += 450.0;
    } // ROCm gets extra for being CUDA alternative
    if node.capabilities.opencl {
        score += 300.0;
    } // OpenCL - mature but older
    if node.capabilities.metal {
        score += 350.0;
    } // Metal - Apple ecosystem

    // Community framework bonuses (champion open AI)
    if node
        .capabilities
        .llama_cpp_backends
        .iter()
        .any(|b| b != "cuda")
    {
        score += 600.0; // Huge bonus for non-CUDA llama.cpp
    }
    if node.capabilities.onnx_providers.iter().any(|p| p != "cuda") {
        score += 500.0; // ONNX cross-platform bonus
    }
    if node.capabilities.candle_support {
        score += 400.0;
    } // Rust ML bonus
    if node.capabilities.burn_support {
        score += 350.0;
    }

    // Cross-platform bonuses (universal compatibility)
    if node.capabilities.wasm_support {
        score += 200.0;
    }
    if node.capabilities.edge_optimized {
        score += 150.0;
    }

    // Proprietary penalty (unless absolutely required)
    if !request.requires_proprietary {
        if node.capabilities.cuda && !has_open_alternatives(node) {
            score -= 1000.0; // Heavy penalty for CUDA-only
        } else if node.capabilities.cuda {
            score -= 200.0; // Light penalty if open alternatives exist
        }
    }

    score
}

fn has_open_alternatives(node: &ComputeNode) -> bool {
    node.capabilities.webgpu
        || node.capabilities.vulkan
        || node.capabilities.rocm
        || node.capabilities.metal
}

fn determine_execution_strategy(
    request: &WorkloadRequest,
    node: &ComputeNode,
) -> ExecutionStrategy {
    match &request.workload_type {
        WorkloadType::AiInference { framework, .. } => {
            match framework.as_str() {
                "llama.cpp" => {
                    // Prefer open backends for llama.cpp
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
                            warning: "Using proprietary CUDA - consider open alternatives"
                                .to_string(),
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
                _ => ExecutionStrategy::OptimizedCPU,
            }
        }
        WorkloadType::GeneralCompute { .. } => {
            // WebGPU first for general compute
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
                    warning: "Legacy application requires CUDA - recommend migration to open alternatives".to_string() 
                }
            } else if node.capabilities.vulkan {
                ExecutionStrategy::Vulkan
            } else {
                ExecutionStrategy::OptimizedCPU
            }
        }
        _ => ExecutionStrategy::OptimizedCPU,
    }
}

fn generate_reasoning(
    _request: &WorkloadRequest,
    node: &ComputeNode,
    strategy: &ExecutionStrategy,
) -> String {
    match strategy {
        ExecutionStrategy::WebGPU => format!(
            "Selected WebGPU on {} - universal standard, works everywhere",
            node.id
        ),
        ExecutionStrategy::Vulkan => format!(
            "Selected Vulkan on {} - open standard, high performance",
            node.id
        ),
        ExecutionStrategy::ROCm => format!(
            "Selected ROCm on {} - open alternative to CUDA, AMD ecosystem",
            node.id
        ),
        ExecutionStrategy::LlamaCppVulkan => format!(
            "Selected llama.cpp with Vulkan on {} - open source AI + open GPU standard",
            node.id
        ),
        ExecutionStrategy::LlamaCppMetal => format!(
            "Selected llama.cpp with Metal on {} - optimized for Apple Silicon",
            node.id
        ),
        ExecutionStrategy::LlamaCppROCm => format!(
            "Selected llama.cpp with ROCm on {} - open source stack end-to-end",
            node.id
        ),
        ExecutionStrategy::ONNXVulkan => format!(
            "Selected ONNX with Vulkan on {} - cross-platform AI standard + open GPU",
            node.id
        ),
        ExecutionStrategy::ONNXROCm => format!(
            "Selected ONNX with ROCm on {} - industry standard AI + open compute",
            node.id
        ),
        ExecutionStrategy::CandleWGPU => format!(
            "Selected Candle with WGPU on {} - pure Rust ML stack",
            node.id
        ),
        ExecutionStrategy::OptimizedCPU => format!(
            "Selected optimized CPU on {} - universal compatibility",
            node.id
        ),
        ExecutionStrategy::CudaIsolated { warning } => {
            format!("⚠️ Selected CUDA on {} (ISOLATED) - {}", node.id, warning)
        }
    }
}

fn print_decision(decision: &SchedulingDecision) {
    println!("   Strategy: {:?}", decision.strategy);
    println!("   Score: {:.1}", decision.score);
    println!("   Reasoning: {}", decision.reasoning);

    match &decision.strategy {
        ExecutionStrategy::CudaIsolated { .. } => {
            println!("   💡 Recommendation: Migrate to open alternatives like Vulkan + llama.cpp");
        }
        ExecutionStrategy::LlamaCppVulkan | ExecutionStrategy::ONNXVulkan => {
            println!("   ✅ Excellent: Open source AI framework + open GPU standard");
        }
        ExecutionStrategy::LlamaCppROCm | ExecutionStrategy::ONNXROCm => {
            println!("   ⭐ Outstanding: Breaking CUDA dependency with open alternatives");
        }
        ExecutionStrategy::WebGPU => {
            println!("   🌟 Perfect: Universal standard that works everywhere");
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

    println!("Node Distribution:");
    println!(
        "  WebGPU capable: {}/{} ({:.1}%)",
        webgpu_nodes,
        total_nodes,
        webgpu_nodes as f64 / total_nodes as f64 * 100.0
    );
    println!(
        "  Vulkan capable: {}/{} ({:.1}%)",
        vulkan_nodes,
        total_nodes,
        vulkan_nodes as f64 / total_nodes as f64 * 100.0
    );
    println!(
        "  ROCm capable: {}/{} ({:.1}%)",
        rocm_nodes,
        total_nodes,
        rocm_nodes as f64 / total_nodes as f64 * 100.0
    );
    println!(
        "  CUDA-only: {}/{} ({:.1}%)",
        cuda_only_nodes,
        total_nodes,
        cuda_only_nodes as f64 / total_nodes as f64 * 100.0
    );
    println!(
        "  Mixed (CUDA + Open): {}/{} ({:.1}%)",
        mixed_nodes,
        total_nodes,
        mixed_nodes as f64 / total_nodes as f64 * 100.0
    );

    println!("\nFramework Support:");
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
        "  llama.cpp: {}/{} ({:.1}%)",
        llama_cpp_nodes,
        total_nodes,
        llama_cpp_nodes as f64 / total_nodes as f64 * 100.0
    );
    println!(
        "  ONNX: {}/{} ({:.1}%)",
        onnx_nodes,
        total_nodes,
        onnx_nodes as f64 / total_nodes as f64 * 100.0
    );
    println!(
        "  Rust ML: {}/{} ({:.1}%)",
        rust_ml_nodes,
        total_nodes,
        rust_ml_nodes as f64 / total_nodes as f64 * 100.0
    );

    println!("\n🎯 Strategic Goals Achievement:");
    let open_coverage =
        (webgpu_nodes + vulkan_nodes + rocm_nodes) as f64 / (total_nodes * 3) as f64 * 100.0;
    let cuda_isolation = cuda_only_nodes as f64 / total_nodes as f64 * 100.0;
    let cross_platform = ecosystem
        .iter()
        .filter(|n| n.capabilities.wasm_support)
        .count() as f64
        / total_nodes as f64
        * 100.0;

    println!("  Open Standards Coverage: {open_coverage:.1}%");
    println!("  CUDA Dependency: {cuda_isolation:.1}% (lower is better)");
    println!("  Cross-Platform Ready: {cross_platform:.1}%");

    println!("\n💡 The NVIDIA Incentive:");
    println!("  Current reality: {mixed_nodes} nodes have CUDA but also open alternatives");
    println!(
        "  Missed opportunity: {cuda_only_nodes} CUDA-only nodes could join universal network with open drivers"
    );
    println!("  Market potential: Millions of gaming GPUs idle while AI demands compute");
    println!("  Strategic advantage: First-mover in federated AI compute gets the ecosystem");

    println!("\n🔮 Future Vision:");
    println!("  Today: Mixed ecosystem with proprietary islands");
    println!("  Tomorrow: Universal compatibility through open standards");
    println!("  End goal: Every GPU contributes to global AI compute network");
    println!("  Winner: Whoever enables this transition first");

    Ok(())
}
