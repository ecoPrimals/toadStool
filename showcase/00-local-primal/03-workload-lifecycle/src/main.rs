// SPDX-License-Identifier: AGPL-3.0-only

use colored::Colorize;
use serde_json::json;
use toadstool_common::generate_id;

fn main() {
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("{}", "  ToadStool Showcase: Workload Lifecycle".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    let job_id = generate_id();
    let cancel_job_id = generate_id();

    // 1. compute.submit request
    println!("{}", "► compute.submit".cyan());
    let submit_request = json!({
        "jsonrpc": "2.0",
        "method": "compute.submit",
        "params": {
            "job_type": "matrix_multiply",
            "data": [1, 2, 3, 4],
            "gpu_hint": true
        },
        "id": 1
    });
    println!("  Request:");
    println!("{}", serde_json::to_string_pretty(&submit_request).unwrap());
    let submit_response = json!({
        "jsonrpc": "2.0",
        "result": {
            "job_id": job_id.to_string(),
            "status": "queued"
        },
        "id": 1
    });
    println!("  Response:");
    println!("{}", serde_json::to_string_pretty(&submit_response).unwrap());
    println!();

    // 2. compute.status request
    println!("{}", "► compute.status".cyan());
    let status_request = json!({
        "jsonrpc": "2.0",
        "method": "compute.status",
        "params": {
            "job_id": job_id.to_string()
        },
        "id": 2
    });
    println!("  Request:");
    println!("{}", serde_json::to_string_pretty(&status_request).unwrap());
    let status_response = json!({
        "jsonrpc": "2.0",
        "result": {
            "job_id": job_id.to_string(),
            "status": "running"
        },
        "id": 2
    });
    println!("  Response:");
    println!("{}", serde_json::to_string_pretty(&status_response).unwrap());
    println!();

    // 3. compute.result request
    println!("{}", "► compute.result".cyan());
    let result_request = json!({
        "jsonrpc": "2.0",
        "method": "compute.result",
        "params": {
            "job_id": job_id.to_string()
        },
        "id": 3
    });
    println!("  Request:");
    println!("{}", serde_json::to_string_pretty(&result_request).unwrap());
    let result_response = json!({
        "jsonrpc": "2.0",
        "result": {
            "job_id": job_id.to_string(),
            "status": "completed",
            "data": [4, 6, 8, 12]
        },
        "id": 3
    });
    println!("  Response:");
    println!("{}", serde_json::to_string_pretty(&result_response).unwrap());
    println!();

    // 4. compute.cancel request (different job)
    println!("{}", "► compute.cancel".cyan());
    let cancel_request = json!({
        "jsonrpc": "2.0",
        "method": "compute.cancel",
        "params": {
            "job_id": cancel_job_id.to_string()
        },
        "id": 4
    });
    println!("  Request:");
    println!("{}", serde_json::to_string_pretty(&cancel_request).unwrap());
    let cancel_response = json!({
        "jsonrpc": "2.0",
        "result": {
            "job_id": cancel_job_id.to_string(),
            "status": "cancelled"
        },
        "id": 4
    });
    println!("  Response:");
    println!("{}", serde_json::to_string_pretty(&cancel_response).unwrap());
    println!();

    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("  {} Full workload lifecycle demonstrated", "✓".green());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
}
