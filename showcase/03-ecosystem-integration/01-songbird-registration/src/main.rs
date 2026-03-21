// SPDX-License-Identifier: AGPL-3.0-only

use colored::Colorize;
use serde_json::json;
use std::path::Path;

const CAPABILITIES: &[&str] = &[
    "compute",
    "gpu",
    "wasm",
    "container",
    "science",
    "shader",
    "ecology",
    "discovery",
    "deploy",
    "hardware_transport",
    "orchestration",
    "ai_local",
];

/// `XDG_RUNTIME_DIR` fallback: discovers UID from `/proc/self/status` (pure Rust, no libc).
fn runtime_dir_fallback() -> String {
    let uid = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Uid:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|u| u.parse::<u32>().ok())
        })
        .unwrap_or(1000);
    format!("/run/user/{uid}")
}

#[tokio::main]
async fn main() {
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("{}", "  ToadStool Showcase: SongBird Capability Registration".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    // Section: ToadStool Capabilities
    println!("{}", "► ToadStool Capabilities".cyan());
    println!("  Capabilities toadStool advertises:");
    for cap in CAPABILITIES {
        println!("    • {}", cap);
    }
    println!();

    // Section: Registration Request
    println!("{}", "► Registration Request".cyan());
    let registration_request = json!({
        "jsonrpc": "2.0",
        "method": "coordination.register",
        "params": {
            "primal": "toadstool",
            "capabilities": CAPABILITIES,
            "socket": "toadstool.jsonrpc.sock",
            "version": "0.1.0"
        },
        "id": 1
    });
    println!("  JSON-RPC request toadStool sends to songBird:");
    println!("{}", serde_json::to_string_pretty(&registration_request).unwrap());
    println!();

    // Section: SongBird Discovery
    println!("{}", "► SongBird Discovery".cyan());
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| runtime_dir_fallback());
    let songbird_sock = format!("{}/biomeos/coordination.sock", runtime_dir);
    let songbird_found = Path::new(&songbird_sock).exists();
    if songbird_found {
        println!("  {} songBird socket found: {}", "✓".green(), songbird_sock);
    } else {
        println!("  {} songBird socket not found: {}", "○".yellow(), songbird_sock);
        println!("  (Simulated: primals may not be running)");
    }
    println!();

    // Section: Cross-Tower Scenario
    println!("{}", "► Cross-Tower Scenario".cyan());
    println!("  songBird federates capability registration across towers.");
    println!("  Each tower's songBird discovers local primals and makes them");
    println!("  accessible cross-tower via tarpc multiplexing.");
    println!();
    println!("  Scenario:");
    println!("    Tower A: toadStool + barraCuda");
    println!("    Tower B: nestGate + bearDog");
    println!("  songBird on each tower discovers capabilities and federates");
    println!("  them — Tower A can discover nestGate/bearDog on Tower B.");
    println!();

    // Section: Health Registration
    println!("{}", "► Health Registration".cyan());
    let health_request = json!({
        "jsonrpc": "2.0",
        "method": "discovery.primal_health",
        "params": { "primal": "toadstool" },
        "id": 2
    });
    println!("  songBird periodically calls to check toadStool is alive:");
    println!("{}", serde_json::to_string_pretty(&health_request).unwrap());
    let health_response = json!({
        "result": { "alive": true, "primal": "toadstool" }
    });
    println!("  Response:");
    println!("{}", serde_json::to_string_pretty(&health_response).unwrap());
    println!();

    // Summary
    let n = CAPABILITIES.len();
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("  {} SongBird registration demonstrated — {} capabilities registered", "✓".green(), n);
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
}
