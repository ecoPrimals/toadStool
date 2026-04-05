// SPDX-License-Identifier: AGPL-3.0-or-later

use colored::Colorize;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("{}", "  ToadStool Showcase: Compilation Status Polling".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    let compilation_id = Uuid::new_v4();

    // Section: Submit Compilation
    println!("{}", "► Submit Compilation".cyan());
    let submit_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "shader.compile.wgsl",
        "params": {
            "source": "@compute @workgroup_size(64) fn main() {}",
            "entry_point": "main"
        }
    });
    println!("{}", serde_json::to_string_pretty(&submit_request).unwrap());
    println!("  Returns: compilation_id = {}", compilation_id);
    println!();

    // Section: Status Polling
    println!("{}", "► Status Polling".cyan());
    println!("  shader.compile.status request: {{ \"compilation_id\": \"<uuid>\" }}");
    println!();

    let polls = [
        json!({ "status": "compiling", "progress": 0.3 }),
        json!({ "status": "compiling", "progress": 0.7 }),
        json!({ "status": "completed", "progress": 1.0, "spirv_size_bytes": 2048 }),
    ];

    for (i, poll) in polls.iter().enumerate() {
        println!("  Poll {}: {}", i + 1, serde_json::to_string(poll).unwrap());
        sleep(Duration::from_millis(200)).await;
    }
    println!();

    // Section: Capabilities
    println!("{}", "► Capabilities".cyan());
    let capabilities = json!({
        "backends": ["naga", "coralreef"],
        "input_formats": ["wgsl", "spirv"],
        "target_formats": ["spirv", "vulkan_native"]
    });
    println!("  shader.compile.capabilities response:");
    println!("{}", serde_json::to_string_pretty(&capabilities).unwrap());
    println!();

    // Summary
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("  {} Async compilation polling demonstrated — 3 status checks", "✓".green());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
}
