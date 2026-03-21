// SPDX-License-Identifier: AGPL-3.0-only
//! Runtime configuration overrides (container, WASM, Python).

use super::super::ConfigResult;
use super::parse;
use crate::ToadStoolConfig;

pub(super) fn apply(config: &mut ToadStoolConfig) -> ConfigResult<()> {
    if let Ok(container_runtime) = std::env::var("TOADSTOOL_CONTAINER_RUNTIME") {
        config.runtime.container.runtime = container_runtime;
    }

    if let Ok(registry) = std::env::var("TOADSTOOL_CONTAINER_REGISTRY") {
        config.runtime.container.default_registry = registry;
    }

    if let Ok(network_mode) = std::env::var("TOADSTOOL_CONTAINER_NETWORK_MODE") {
        config.runtime.container.network_mode = network_mode;
    }

    if let Ok(wasm_engine) = std::env::var("TOADSTOOL_WASM_ENGINE") {
        config.runtime.wasm.engine = wasm_engine;
    }

    if let Ok(max_memory) = std::env::var("TOADSTOOL_WASM_MAX_MEMORY") {
        config.runtime.wasm.max_memory = parse::parse_u64(&max_memory, "WASM max memory")?;
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_WASM_ENABLE_WASI") {
        config.runtime.wasm.enable_wasi = parse::parse_bool(&enabled);
    }

    if let Ok(python_exe) = std::env::var("TOADSTOOL_PYTHON_EXECUTABLE") {
        config.runtime.python.executable = python_exe;
    }

    if let Ok(venv_path) = std::env::var("TOADSTOOL_PYTHON_VENV_PATH") {
        config.runtime.python.venv_path = Some(venv_path);
    }

    if let Ok(index_url) = std::env::var("TOADSTOOL_PYTHON_INDEX_URL") {
        config.runtime.python.index_url = index_url;
    }

    if let Ok(max_memory) = std::env::var("TOADSTOOL_PYTHON_MAX_MEMORY") {
        config.runtime.python.max_memory = parse::parse_u64(&max_memory, "Python max memory")?;
    }

    Ok(())
}
