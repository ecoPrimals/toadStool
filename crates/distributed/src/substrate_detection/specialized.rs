// SPDX-License-Identifier: AGPL-3.0-or-later
//! Specialized platform detection (WebAssembly runtimes).

use super::probe;
use super::types::PlatformType;
use toadstool::ToadStoolResult;

const WASM_RUNTIMES: &[(&str, &str)] = &[
    ("wasmtime", "Wasmtime"),
    ("wasmer", "Wasmer"),
    ("wasmedge", "WasmEdge"),
];

/// Detect specialized platforms (e.g. WebAssembly runtimes).
#[expect(
    clippy::unused_async,
    reason = "async signature required by trait/interface"
)] // Sync probe; async for API consistency with SubstrateDetector
pub async fn detect() -> ToadStoolResult<Vec<PlatformType>> {
    let mut platforms = Vec::new();

    for (command, name) in WASM_RUNTIMES {
        if probe::command_exists(command) {
            platforms.push(PlatformType::WebAssembly {
                runtime: (*name).to_string(),
            });
        }
    }

    Ok(platforms)
}
