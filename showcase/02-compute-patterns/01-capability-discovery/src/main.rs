// SPDX-License-Identifier: AGPL-3.0-or-later

use colored::Colorize;
use serde_json::json;
use std::path::Path;

const SOCKETS: &[(&str, &str)] = &[
    ("toadstool.jsonrpc.sock", "toadStool server"),
    ("compute.sock", "barraCuda"),
    ("coralreef.sock", "coralReef"),
    ("coordination.sock", "songBird"),
    ("security.sock", "bearDog"),
    ("storage.sock", "nestGate"),
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
    println!("{}", "  ToadStool Showcase: Capability-Based Discovery".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    // Section: Socket Discovery
    println!("{}", "► Socket Discovery".cyan());
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| runtime_dir_fallback());
    let biomeos = format!("{}/biomeos", runtime_dir);

    let mut found = 0u32;
    for (sock_name, description) in SOCKETS {
        let path = format!("{}/{}", biomeos, sock_name);
        let exists = Path::new(&path).exists();
        if exists {
            found += 1;
            println!("  {} {} ({})", "✓".green(), description, path);
        } else {
            println!("  {} {} ({})", "○".yellow(), description, path);
        }
    }
    println!();

    // Section: Discovery Protocol
    println!("{}", "► Discovery Protocol".cyan());
    let discovery_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "discovery.primals",
        "params": {}
    });
    println!("  Request:");
    println!("{}", serde_json::to_string_pretty(&discovery_request).unwrap());
    let discovery_response = json!({
        "result": {
            "primals": [
                {
                    "name": "toadstool",
                    "capabilities": ["compute", "gpu", "science"],
                    "socket": format!("{}/biomeos/toadstool.jsonrpc.sock", runtime_dir)
                }
            ]
        }
    });
    println!("  Response:");
    println!("{}", serde_json::to_string_pretty(&discovery_response).unwrap());
    println!();

    // Section: Topology
    println!("{}", "► Topology".cyan());
    let topology_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "discovery.topology",
        "params": {}
    });
    println!("  Request:");
    println!("{}", serde_json::to_string_pretty(&topology_request).unwrap());
    let topology_response = json!({
        "result": {
            "nodes": [
                {"id": "toadstool", "capabilities": ["compute", "orchestrate"]},
                {"id": "coralreef", "capabilities": ["compile", "wgsl", "spirv"]},
                {"id": "barracuda", "capabilities": ["execute", "gpu", "wgpu"]}
            ],
            "edges": [
                {"from": "coralreef", "to": "toadstool"},
                {"from": "toadstool", "to": "barracuda"}
            ]
        }
    });
    println!("  Response:");
    println!("{}", serde_json::to_string_pretty(&topology_response).unwrap());
    println!();

    // Section: Compute Triangle
    println!("{}", "► Compute Triangle".cyan());
    println!("  coralReef (compile) ----> toadStool (orchestrate) ----> barraCuda (execute)");
    println!("       WGSL/SPIR-V              WHERE to run               WHAT to compute");
    println!();

    // Summary
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("  {} {}/6 primals discovered on this host", "✓".green(), found);
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
}
