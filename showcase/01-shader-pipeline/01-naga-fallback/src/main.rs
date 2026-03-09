// SPDX-License-Identifier: AGPL-3.0-only

use colored::Colorize;
use serde_json::json;
use std::path::Path;
use tokio::net::UnixStream;

const WGSL_SOURCE: &str = r#"@group(0) @binding(0) var<storage, read_write> data: array<f32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    data[gid.x] = data[gid.x] * 2.0;
}
"#;

#[tokio::main]
async fn main() {
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("{}", "  ToadStool Showcase: Naga Shader Fallback".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    // Section: WGSL Source
    println!("{}", "► WGSL Source".cyan());
    println!("{}", WGSL_SOURCE);
    println!();

    // Section: Compilation Request
    println!("{}", "► Compilation Request".cyan());
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "shader.compile.wgsl",
        "params": {
            "source": WGSL_SOURCE,
            "entry_point": "main"
        }
    });
    println!("{}", serde_json::to_string_pretty(&request).unwrap());
    println!();

    // Section: Naga Compilation
    println!("{}", "► Naga Compilation".cyan());
    println!("  When coralReef is unavailable, toadStool uses naga for WGSL -> SPIR-V compilation.");
    let response = json!({
        "result": {
            "status": "compiled",
            "backend": "naga",
            "spirv_size_bytes": 1024,
            "entry_points": ["main"]
        }
    });
    println!("  Simulated response:");
    println!("{}", serde_json::to_string_pretty(&response).unwrap());
    println!();

    // Section: Socket Check
    println!("{}", "► Socket Check".cyan());
    let sock_path = std::env::var("XDG_RUNTIME_DIR")
        .map(|d| format!("{}/biomeos/toadstool.jsonrpc.sock", d))
        .unwrap_or_else(|_| "/run/user/1000/biomeos/toadstool.jsonrpc.sock".to_string());

    let live = Path::new(&sock_path).exists()
        && UnixStream::connect(&sock_path).await.is_ok();

    if live {
        println!("  {} Live server detected.", "✓".green());
    } else {
        println!("  Server not running -- showing simulated responses (this is normal for standalone demo).");
    }
    println!();

    // Summary
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("  {} Naga fallback compilation demonstrated — no coralReef needed", "✓".green());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
}
