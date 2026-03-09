// SPDX-License-Identifier: AGPL-3.0-only

use colored::Colorize;
use serde_json::json;
use std::path::Path;

#[tokio::main]
async fn main() {
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("{}", "  ToadStool Showcase: BearDog-Secured Compute".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    // Section: Security Flow
    println!("{}", "► Security Flow".cyan());
    println!("  1. Client authenticates with bearDog -> gets auth token");
    println!("  2. Client sends compute request to toadStool with bearer token");
    println!("  3. toadStool validates token with bearDog (zero-trust)");
    println!("  4. toadStool executes workload only if authorized");
    println!();

    // Section: Authentication
    println!("{}", "► Authentication".cyan());
    let auth_request = json!({
        "jsonrpc": "2.0",
        "method": "security.authenticate",
        "params": {
            "identity": "user@tower.local",
            "scope": "compute.submit"
        },
        "id": 1
    });
    println!("  JSON-RPC request to bearDog:");
    println!("{}", serde_json::to_string_pretty(&auth_request).unwrap());
    let auth_response = json!({
        "result": {
            "token": "bearer_eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
            "expires_at": "2025-03-10T12:00:00Z",
            "scope": "compute.submit"
        }
    });
    println!("  Response:");
    println!("{}", serde_json::to_string_pretty(&auth_response).unwrap());
    println!();

    // Section: Secured Workload Submission
    println!("{}", "► Secured Workload Submission".cyan());
    let submit_request = json!({
        "jsonrpc": "2.0",
        "method": "compute.submit",
        "params": {
            "bearer_token": "<token>",
            "job_type": "eigensolve",
            "data": [1.0, 2.0, 3.0, 4.0]
        },
        "id": 2
    });
    println!("  compute.submit request with authorization:");
    println!("{}", serde_json::to_string_pretty(&submit_request).unwrap());
    println!();

    // Section: Zero-Trust Validation
    println!("{}", "► Zero-Trust Validation".cyan());
    let validate_request = json!({
        "jsonrpc": "2.0",
        "method": "security.validate_token",
        "params": {
            "token": "<token>",
            "required_scope": "compute.submit"
        },
        "id": 3
    });
    println!("  toadStool's internal validation call to bearDog:");
    println!("{}", serde_json::to_string_pretty(&validate_request).unwrap());
    let validate_response = json!({
        "result": {
            "valid": true,
            "identity": "user@tower.local",
            "expires_at": "2025-03-10T12:00:00Z"
        }
    });
    println!("  Response:");
    println!("{}", serde_json::to_string_pretty(&validate_response).unwrap());
    println!();

    // Section: BearDog Socket Check
    println!("{}", "► BearDog Socket Check".cyan());
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/run/user/1000".to_string());
    let beardog_sock = format!("{}/biomeos/security.sock", runtime_dir);
    let beardog_found = Path::new(&beardog_sock).exists();
    if beardog_found {
        println!("  {} bearDog socket found: {}", "✓".green(), beardog_sock);
    } else {
        println!("  {} bearDog socket not found: {}", "○".yellow(), beardog_sock);
        println!("  (Simulated: primals may not be running)");
    }
    println!();

    // Section: Standalone Fallback
    println!("{}", "► Standalone Fallback".cyan());
    println!("  Without bearDog, toadStool operates in standalone mode");
    println!("  (no auth required). Capability-based discovery handles");
    println!("  this gracefully — clients can still submit workloads");
    println!("  when running in isolated/development environments.");
    println!();

    // Summary
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("  {} BearDog-secured compute demonstrated — zero-trust validation", "✓".green());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
}
