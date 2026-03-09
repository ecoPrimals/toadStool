// SPDX-License-Identifier: AGPL-3.0-only

use colored::Colorize;
use serde_json::json;

#[tokio::main]
async fn main() {
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("{}", "  ToadStool Showcase: Science Dispatch".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    // Section: GPU Capabilities Query
    println!("{}", "► GPU Capabilities Query".cyan());
    let gpu_caps_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "science.gpu.capabilities",
        "params": {}
    });
    println!("  Request:");
    println!("{}", serde_json::to_string_pretty(&gpu_caps_request).unwrap());
    let gpu_caps_response = json!({
        "result": {
            "backend": "wgpu",
            "adapter": "Vulkan",
            "features": ["compute", "storage_buffer", "float64"],
            "limits": {"max_compute_workgroups_per_dimension": 65535}
        }
    });
    println!("  Simulated response:");
    println!("{}", serde_json::to_string_pretty(&gpu_caps_response).unwrap());
    println!();

    // Section: Compute Job Submission
    println!("{}", "► Compute Job Submission".cyan());
    let submit_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "science.compute.submit",
        "params": {
            "operation": "matrix_multiply",
            "dimensions": [1024, 1024],
            "precision": "f32",
            "job_id": "550e8400-e29b-41d4-a716-446655440000"
        }
    });
    println!("  Request:");
    println!("{}", serde_json::to_string_pretty(&submit_request).unwrap());
    println!();

    // Section: GPU Dispatch
    println!("{}", "► GPU Dispatch".cyan());
    let gpu_dispatch_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "science.gpu.dispatch",
        "params": {
            "operation": "matrix_multiply",
            "dimensions": [1024, 1024],
            "precision": "f32",
            "gpu_hint": true
        }
    });
    println!("  Request:");
    println!("{}", serde_json::to_string_pretty(&gpu_dispatch_request).unwrap());
    let gpu_dispatch_response = json!({
        "result": {
            "status": "dispatched",
            "elapsed_ms": 1.2,
            "elements_processed": 1048576,
            "throughput_gflops": 873.8
        }
    });
    println!("  Simulated response:");
    println!("{}", serde_json::to_string_pretty(&gpu_dispatch_response).unwrap());
    println!();

    // Section: NPU Dispatch
    println!("{}", "► NPU Dispatch".cyan());
    let npu_dispatch_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "science.npu.dispatch",
        "params": {
            "operation": "inference",
            "model": "resnet50",
            "batch_size": 32
        }
    });
    println!("  Request:");
    println!("{}", serde_json::to_string_pretty(&npu_dispatch_request).unwrap());
    let npu_dispatch_response = json!({
        "result": {
            "status": "completed",
            "substrate": "cpu",
            "note": "Akida NPU not present — fallback to CPU inference",
            "elapsed_ms": 45.2
        }
    });
    println!("  Simulated response:");
    println!("{}", serde_json::to_string_pretty(&npu_dispatch_response).unwrap());
    println!();

    // Section: Substrate Discovery
    println!("{}", "► Substrate Discovery".cyan());
    let substrate_request = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "science.substrate.discover",
        "params": {}
    });
    println!("  Request:");
    println!("{}", serde_json::to_string_pretty(&substrate_request).unwrap());
    let substrate_response = json!({
        "result": {
            "substrates": [
                {"id": "cpu", "type": "cpu", "available": true},
                {"id": "gpu", "type": "gpu", "available": true, "backend": "wgpu"},
                {"id": "npu", "type": "npu", "available": false, "note": "Akida not detected"}
            ]
        }
    });
    println!("  Simulated response:");
    println!("{}", serde_json::to_string_pretty(&substrate_response).unwrap());
    println!();

    // Summary
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("  {} Science dispatch demonstrated — GPU, NPU, and substrate discovery", "✓".green());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
}
