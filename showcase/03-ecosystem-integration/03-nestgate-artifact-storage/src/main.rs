// SPDX-License-Identifier: AGPL-3.0-only

use colored::Colorize;
use serde_json::json;
use std::path::Path;

const ARTIFACT_TYPES: &[&str] = &[
    "Compiled shaders (SPIR-V, native binaries)",
    "Model weights (from training runs)",
    "Benchmark results",
    "Job outputs (matrices, datasets)",
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
    println!("{}", "  ToadStool Showcase: NestGate Artifact Storage".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    // Section: Storage Flow
    println!("{}", "► Storage Flow".cyan());
    println!("  1. toadStool compiles shader or completes compute job");
    println!("  2. Results are stored in nestGate for persistence");
    println!("  3. Future requests can retrieve cached artifacts");
    println!("  4. nestGate provides dedup, compression, and snapshots");
    println!();

    // Section: Store Artifact
    println!("{}", "► Store Artifact".cyan());
    let store_request = json!({
        "jsonrpc": "2.0",
        "method": "storage.artifact.store",
        "params": {
            "artifact_type": "compiled_shader",
            "name": "vector_add_spirv",
            "data": "<base64_spirv>",
            "metadata": {
                "source_hash": "sha256:abc123...",
                "compiler": "naga",
                "entry_point": "main"
            }
        },
        "id": 1
    });
    println!("  storage.artifact.store request:");
    println!("{}", serde_json::to_string_pretty(&store_request).unwrap());
    let store_response = json!({
        "result": {
            "artifact_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
            "stored_at": "2025-03-09T10:00:00Z"
        }
    });
    println!("  Response:");
    println!("{}", serde_json::to_string_pretty(&store_response).unwrap());
    println!();

    // Section: Retrieve Artifact
    println!("{}", "► Retrieve Artifact".cyan());
    let retrieve_request = json!({
        "jsonrpc": "2.0",
        "method": "storage.artifact.retrieve",
        "params": {
            "artifact_id": "<uuid>"
        },
        "id": 2
    });
    println!("  storage.artifact.retrieve request:");
    println!("{}", serde_json::to_string_pretty(&retrieve_request).unwrap());
    let retrieve_response = json!({
        "result": {
            "data": "<base64_spirv>",
            "metadata": {
                "artifact_type": "compiled_shader",
                "name": "vector_add_spirv",
                "source_hash": "sha256:abc123...",
                "compiler": "naga",
                "entry_point": "main"
            }
        }
    });
    println!("  Response:");
    println!("{}", serde_json::to_string_pretty(&retrieve_response).unwrap());
    println!();

    // Section: Artifact Types
    println!("{}", "► Artifact Types".cyan());
    println!("  Compute artifacts toadStool might store:");
    for artifact_type in ARTIFACT_TYPES {
        println!("    • {}", artifact_type);
    }
    println!();

    // Section: NestGate Socket Check
    println!("{}", "► NestGate Socket Check".cyan());
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| runtime_dir_fallback());
    let nestgate_sock = format!("{}/biomeos/storage.sock", runtime_dir);
    let nestgate_found = Path::new(&nestgate_sock).exists();
    if nestgate_found {
        println!("  {} nestGate socket found: {}", "✓".green(), nestgate_sock);
    } else {
        println!("  {} nestGate socket not found: {}", "○".yellow(), nestgate_sock);
        println!("  (Simulated: primals may not be running)");
    }
    println!();

    // Section: ZFS Integration
    println!("{}", "► ZFS Integration".cyan());
    println!("  nestGate uses ZFS under the hood — artifacts get automatic");
    println!("  compression, deduplication, and snapshot-based versioning.");
    println!("  A compiled shader stored once is never duplicated.");
    println!();

    // Summary
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("  {} NestGate artifact storage demonstrated — persistent compute artifacts", "✓".green());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
}
