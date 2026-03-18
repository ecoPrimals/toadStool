// SPDX-License-Identifier: AGPL-3.0-or-later

use colored::Colorize;
use serde_json::json;
use std::path::Path;

const WGSL_SOURCE: &str = r#"@group(0) @binding(0) var<storage, read_write> data: array<f32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    data[gid.x] = data[gid.x] * 2.0;
}
"#;

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

fn check_path(path: &str) -> bool {
    Path::new(path).exists()
}

#[tokio::main]
async fn main() {
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("{}", "  ToadStool Showcase: coralReef Shader Compilation".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    // Section: Discovery
    println!("{}", "► Discovery".cyan());
    let xdg = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| runtime_dir_fallback());

    let coralreef_url = std::env::var("CORALREEF_URL").ok();
    let eco_path = format!("{}/ecoPrimals/coralreef-core.json", xdg);
    let biome_path = format!("{}/biomeos/coralreef.sock", xdg);

    let locations: [(_, &str); 3] = [
        ("$CORALREEF_URL", coralreef_url.as_deref().unwrap_or("(unset)")),
        ("$XDG_RUNTIME_DIR/ecoPrimals/coralreef-core.json", &eco_path),
        ("$XDG_RUNTIME_DIR/biomeos/coralreef.sock", &biome_path),
    ];

    let mut coralreef_found = false;
    for (label, path) in &locations {
        let found = if path.starts_with("(unset)") {
            false
        } else {
            check_path(path)
        };
        if found {
            coralreef_found = true;
        }
        let status = if found { "found".green() } else { "not found".dimmed() };
        println!("  {}: {} ({})", label, path, status);
    }
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

    // Section: Compilation Response
    println!("{}", "► Compilation Response".cyan());
    if coralreef_found {
        println!("  coralReef socket found — toadStool would forward to coralReef.");
    } else {
        println!("  coralReef not found. Showing naga fallback response:");
        let fallback = json!({
            "result": {
                "status": "compiled",
                "backend": "naga",
                "spirv_size_bytes": 1024,
                "entry_points": ["main"]
            }
        });
        println!("{}", serde_json::to_string_pretty(&fallback).unwrap());
        println!();
        println!("  coralReef provides native binary compilation (SPIR-V -> GPU native).");
        println!("  When absent, naga provides WGSL -> SPIR-V.");
        println!("  The full pipeline: WGSL -> coralReef -> native binary.");
    }
    println!();

    // Section: SPIR-V Compilation
    println!("{}", "► SPIR-V Compilation".cyan());
    let spirv_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "shader.compile.spirv",
        "params": {
            "spirv": "<base64>",
            "target": "vulkan"
        }
    });
    println!("  shader.compile.spirv request format:");
    println!("{}", serde_json::to_string_pretty(&spirv_request).unwrap());
    println!();

    // Summary
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("  {} Shader pipeline: WGSL -> [coralReef | naga] -> SPIR-V -> native", "✓".green());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
}
