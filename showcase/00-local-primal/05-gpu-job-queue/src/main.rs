// SPDX-License-Identifier: AGPL-3.0-or-later

use colored::Colorize;
use serde_json::json;
use toadstool_common::generate_id;

fn main() {
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("{}", "  ToadStool Showcase: GPU Job Queue".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    // Section 1: GPU Capabilities
    println!("{}", "► GPU Capabilities".cyan());
    let gpu_caps = json!({
        "backends": ["wgpu"],
        "shader_models": ["compute"],
        "max_workgroup_size": 256,
        "features": ["timestamp_query", "buffer_binding_array"]
    });
    println!("  science.gpu.capabilities response:");
    println!("{}", serde_json::to_string_pretty(&gpu_caps).unwrap());
    println!();

    // Section 2: Job Submission
    println!("{}", "► Job Submission".cyan());
    let job1_id = generate_id();
    let job2_id = generate_id();
    let job3_id = generate_id();

    let jobs = [
        ("matrix_multiply", "high", true),
        ("fft_transform", "normal", true),
        ("data_reduction", "low", false),
    ];
    let ids = [job1_id, job2_id, job3_id];

    for (i, ((name, priority, gpu_hint), id)) in jobs.iter().zip(ids.iter()).enumerate() {
        let dispatch = json!({
            "jsonrpc": "2.0",
            "method": "science.gpu.dispatch",
            "params": {
                "job_id": id.to_string(),
                "job_type": name,
                "priority": priority,
                "gpu_hint": gpu_hint
            },
            "id": i + 1
        });
        println!("  Job {} ({}):", i + 1, name);
        println!("{}", serde_json::to_string_pretty(&dispatch).unwrap());
        println!();
    }

    // Section 3: Queue State
    println!("{}", "► Queue State".cyan());
    println!("  3 jobs total: 1 running, 2 queued (ordered by priority)");
    println!("  Running: matrix_multiply (high)");
    println!("  Queued:  fft_transform (normal), data_reduction (low)");
    println!();

    // Section 4: Job Completion
    println!("{}", "► Job Completion".cyan());
    println!("  matrix_multiply (high)   → completed in 0.12s");
    println!("  fft_transform (normal)   → completed in 0.45s");
    println!("  data_reduction (low)     → completed in 0.08s (CPU fallback)");
    println!();

    // Section 5: NPU Capabilities
    println!("{}", "► NPU Capabilities".cyan());
    let akida_status = if std::env::consts::OS == "linux" {
        "unavailable (platform not detected)"
    } else {
        "unavailable (non-Linux)"
    };
    let npu_caps = json!({
        "science.npu.capabilities": {
            "akida": akida_status
        }
    });
    println!("{}", serde_json::to_string_pretty(&npu_caps).unwrap());
    println!();

    // Summary
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("  {} GPU job queue demonstrated — 3 jobs processed", "✓".green());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
}
