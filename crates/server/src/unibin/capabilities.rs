// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local capability detection and verification
//!
//! Pure Rust hardware detection for GPU, CPU, and memory capabilities.

/// Query local GPU and compute capabilities
///
/// Uses sysinfo and wgpu for pure Rust discovery.
#[allow(clippy::unused_async)] // API consistency; may add async I/O for GPU discovery in future
pub async fn query_local_capabilities() -> Vec<String> {
    let mut capabilities = vec!["compute".to_string(), "cpu".to_string()];

    let cpus = std::thread::available_parallelism()
        .map(std::num::NonZero::get)
        .unwrap_or(4);
    if cpus >= 16 {
        capabilities.push("high-core-count".to_string());
        tracing::info!("✅ High core count detected: {} cores", cpus);
    }

    let mut sys = sysinfo::System::new_all();
    sys.refresh_memory();
    #[allow(clippy::cast_precision_loss)]
    let total_memory_gb = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    if total_memory_gb >= 32.0 {
        capabilities.push("high-memory".to_string());
        tracing::info!("✅ High memory detected: {:.1} GB", total_memory_gb);
    }

    #[cfg(feature = "gpu-discovery")]
    {
        let adapters = wgpu::Instance::default().enumerate_adapters(wgpu::Backends::all());
        if !adapters.is_empty() {
            capabilities.push("gpu".to_string());

            for adapter in adapters {
                let info = adapter.get_info();
                tracing::info!("✅ Detected GPU: {} ({:?})", info.name, info.backend);

                match info.backend {
                    wgpu::Backend::Vulkan => {
                        if !capabilities.contains(&"vulkan".to_string()) {
                            capabilities.push("vulkan".to_string());
                        }
                    }
                    wgpu::Backend::Metal => {
                        if !capabilities.contains(&"metal".to_string()) {
                            capabilities.push("metal".to_string());
                        }
                    }
                    wgpu::Backend::Dx12 => {
                        if !capabilities.contains(&"dx12".to_string()) {
                            capabilities.push("dx12".to_string());
                        }
                    }
                    _ => {}
                }

                let name_lower = info.name.to_lowercase();
                if name_lower.contains("nvidia") && !capabilities.contains(&"cuda".to_string()) {
                    capabilities.push("cuda".to_string());
                } else if name_lower.contains("amd") && !capabilities.contains(&"rocm".to_string())
                {
                    capabilities.push("rocm".to_string());
                } else if name_lower.contains("intel")
                    && !capabilities.contains(&"oneapi".to_string())
                {
                    capabilities.push("oneapi".to_string());
                }
            }
        } else {
            tracing::info!("No GPUs detected (CPU-only mode)");
        }
    }

    #[cfg(not(feature = "gpu-discovery"))]
    {
        tracing::info!("GPU discovery disabled (compile with --features gpu-discovery)");
    }

    capabilities.push("orchestration".to_string());
    tracing::info!("📊 Local capabilities: {:?}", capabilities);
    capabilities
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn query_local_capabilities_returns_non_empty() {
        let caps = query_local_capabilities().await;
        assert!(!caps.is_empty(), "capabilities should never be empty");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn query_local_capabilities_always_includes_compute_cpu_orchestration() {
        let caps = query_local_capabilities().await;
        assert!(
            caps.contains(&"compute".to_string()),
            "should include 'compute': {caps:?}"
        );
        assert!(
            caps.contains(&"cpu".to_string()),
            "should include 'cpu': {caps:?}"
        );
        assert!(
            caps.contains(&"orchestration".to_string()),
            "should include 'orchestration': {caps:?}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn query_local_capabilities_no_duplicates() {
        let caps = query_local_capabilities().await;
        let mut seen = std::collections::HashSet::new();
        for c in &caps {
            assert!(seen.insert(c), "duplicate capability '{c}' in {caps:?}");
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn query_local_capabilities_all_non_empty() {
        let caps = query_local_capabilities().await;
        for c in &caps {
            assert!(!c.is_empty(), "capability string should not be empty");
        }
    }
}
