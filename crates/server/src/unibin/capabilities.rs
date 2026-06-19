// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local capability detection and verification
//!
//! Pure Rust hardware detection for GPU, CPU, and memory capabilities.
//!
//! Returns `Vec<Arc<str>>` to avoid allocations when capabilities are shared
//! across handlers (clone is refcount bump, not memcpy).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

static CACHED_CAPABILITIES: std::sync::OnceLock<Vec<Arc<str>>> = std::sync::OnceLock::new();
static GPU_DISCOVERY_STARTED: AtomicBool = AtomicBool::new(false);

/// Query local GPU and compute capabilities.
///
/// Uses toadstool-sysmon and wgpu for pure Rust discovery (zero C).
/// Returns `Arc<str>` per capability — clone is cheap (refcount bump).
///
/// **Lazy GPU discovery (Wave 49 startup optimization):**
/// First call returns a fast baseline (CPU, memory, orchestration) and
/// spawns wgpu GPU enumeration in the background. Subsequent calls after
/// GPU discovery completes return the full cached result including GPU
/// capabilities. This prevents wgpu's Vulkan driver init (1–5s) from
/// blocking the server startup path.
pub async fn query_local_capabilities() -> Vec<Arc<str>> {
    if let Some(cached) = CACHED_CAPABILITIES.get() {
        return cached.clone();
    }

    // Kick off GPU discovery in background (once)
    if !GPU_DISCOVERY_STARTED.swap(true, Ordering::Relaxed) {
        tokio::spawn(async {
            let full = discover_all_capabilities().await;
            let _ = CACHED_CAPABILITIES.set(full);
        });
    }

    query_baseline_capabilities()
}

/// Headless query: returns baseline capabilities only (CPU, memory, orchestration).
/// Skips all GPU/NPU hardware probing for port-free VPS deployment.
#[expect(
    clippy::unused_async,
    reason = "async to match query_local_capabilities signature"
)]
pub async fn query_baseline_only() -> Vec<Arc<str>> {
    tracing::info!("Headless mode: returning CPU-only baseline capabilities (no GPU/NPU probes)");
    query_baseline_capabilities()
}

/// Fast baseline: CPU + memory + orchestration — no GPU probing.
fn query_baseline_capabilities() -> Vec<Arc<str>> {
    let mut capabilities: Vec<Arc<str>> = vec![Arc::from("compute"), Arc::from("cpu")];

    let cpus = toadstool_sysmon::cpu_count();
    if cpus >= 16 {
        capabilities.push(Arc::from("high-core-count"));
    }

    if let Ok(mem) = toadstool_sysmon::memory_info() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "total_memory u64 → f64 acceptable"
        )]
        let total_memory_gb = mem.total as f64 / 1024.0 / 1024.0 / 1024.0;
        if total_memory_gb >= 32.0 {
            capabilities.push(Arc::from("high-memory"));
        }
    }

    capabilities.push(Arc::from("orchestration"));
    capabilities
}

/// Full capability detection including GPU discovery via wgpu.
///
/// Runs in a background task to avoid blocking server startup.
#[expect(
    clippy::unused_async,
    reason = "async required when gpu-discovery feature enables wgpu adapter enumeration"
)]
async fn discover_all_capabilities() -> Vec<Arc<str>> {
    let mut capabilities = query_baseline_capabilities();

    tracing::info!("🔍 Starting GPU discovery (background)...");

    #[cfg(feature = "gpu-discovery")]
    {
        let adapters = wgpu::Instance::default().enumerate_adapters(wgpu::Backends::all());
        if adapters.is_empty() {
            tracing::info!("No GPUs detected (CPU-only mode)");
        } else {
            capabilities.push(Arc::from("gpu"));

            for adapter in adapters {
                let info = adapter.get_info();
                tracing::info!("✅ Detected GPU: {} ({:?})", info.name, info.backend);

                match info.backend {
                    wgpu::Backend::Vulkan
                        if !capabilities.iter().any(|c| c.as_ref() == "vulkan") =>
                    {
                        capabilities.push(Arc::from("vulkan"));
                    }
                    wgpu::Backend::Metal if !capabilities.iter().any(|c| c.as_ref() == "metal") => {
                        capabilities.push(Arc::from("metal"));
                    }
                    wgpu::Backend::Dx12 if !capabilities.iter().any(|c| c.as_ref() == "dx12") => {
                        capabilities.push(Arc::from("dx12"));
                    }
                    _ => {}
                }

                let name_lower = info.name.to_lowercase();
                if name_lower.contains("nvidia")
                    && !capabilities.iter().any(|c| c.as_ref() == "cuda")
                {
                    capabilities.push(Arc::from("cuda"));
                } else if name_lower.contains("amd")
                    && !capabilities.iter().any(|c| c.as_ref() == "rocm")
                {
                    capabilities.push(Arc::from("rocm"));
                } else if name_lower.contains("intel")
                    && !capabilities.iter().any(|c| c.as_ref() == "oneapi")
                {
                    capabilities.push(Arc::from("oneapi"));
                }
            }
        }
    }

    #[cfg(not(feature = "gpu-discovery"))]
    {
        tracing::info!("GPU discovery disabled (compile with --features gpu-discovery)");
    }

    tracing::info!("📊 Full capabilities: {:?}", capabilities);
    capabilities
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wgpu_safe_or_skip() -> bool {
        if toadstool_testing::gpu_guards::is_wgpu_safe() {
            return true;
        }
        eprintln!("{}", toadstool_testing::gpu_guards::wgpu_skip_reason());
        false
    }

    #[tokio::test(flavor = "current_thread")]
    async fn query_local_capabilities_returns_non_empty() {
        if !wgpu_safe_or_skip() {
            return;
        }
        let caps = query_local_capabilities().await;
        assert!(!caps.is_empty(), "capabilities should never be empty");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn query_local_capabilities_always_includes_compute_cpu_orchestration() {
        if !wgpu_safe_or_skip() {
            return;
        }
        let caps = query_local_capabilities().await;
        assert!(
            caps.iter().any(|c| c.as_ref() == "compute"),
            "should include 'compute': {caps:?}"
        );
        assert!(
            caps.iter().any(|c| c.as_ref() == "cpu"),
            "should include 'cpu': {caps:?}"
        );
        assert!(
            caps.iter().any(|c| c.as_ref() == "orchestration"),
            "should include 'orchestration': {caps:?}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn query_local_capabilities_no_duplicates() {
        if !wgpu_safe_or_skip() {
            return;
        }
        let caps = query_local_capabilities().await;
        let mut seen = std::collections::HashSet::new();
        for c in &caps {
            assert!(
                seen.insert(c.as_ref()),
                "duplicate capability '{c}' in {caps:?}"
            );
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn query_local_capabilities_all_non_empty() {
        if !wgpu_safe_or_skip() {
            return;
        }
        let caps = query_local_capabilities().await;
        for c in &caps {
            assert!(!c.is_empty(), "capability string should not be empty");
        }
    }
}
