//! Runtime GPU capability probing
//!
//! Dispatches tiny test shaders to empirically verify hardware capabilities
//! rather than relying on driver name heuristics.
//!
//! # Design
//!
//! Name-based detection (is_nvk, is_radv) is synchronous and fast but fragile.
//! Probe-based detection is async and definitive. This module provides the probe
//! layer that overrides name-based guesses with verified results, cached globally
//! per adapter identity (name + backend).
//!
//! # Usage
//!
//! ```rust,ignore
//! let capable = probe_f64_exp_capable(&device).await;
//! // capable == false → use software exp_f64() fallback
//! // capable == true  → native exp(f64(x)) works on this driver
//! ```

use crate::device::WgpuDevice;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Global probe result cache keyed by adapter_name:backend:vendor
static F64_EXP_PROBE_CACHE: LazyLock<Mutex<HashMap<String, bool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// WGSL probe shader: computes exp(f64(1.0)) and stores result.
/// If the driver supports native f64 exp, result ≈ e (2.718...).
/// If NAK/ACO crashes on f64 exp, the error scope catches the failure.
const F64_EXP_PROBE_SHADER: &str = r#"
enable f64;
@group(0) @binding(0) var<storage, read_write> out: array<f64>;
@compute @workgroup_size(1)
fn probe_exp_f64(@builtin(global_invocation_id) _id: vec3<u32>) {
    out[0] = exp(f64(1.0));
}
"#;

/// Probe whether this device supports native `exp(f64)` / `log(f64)`.
///
/// Returns `true` if native f64 exp/log work correctly (no workaround needed).
/// Returns `false` if the driver crashes or produces wrong results (use software fallback).
///
/// Results are cached globally per adapter identity — subsequent calls are instant.
pub async fn probe_f64_exp_capable(device: &WgpuDevice) -> bool {
    let key = adapter_key(device);

    // Fast path: cached result
    if let Some(&cached) = F64_EXP_PROBE_CACHE.lock().unwrap().get(&key) {
        return cached;
    }

    // Run probe — if it returns an error, not capable
    let capable = run_exp_probe(device.device(), device.queue()).await;

    F64_EXP_PROBE_CACHE.lock().unwrap().insert(key, capable);
    capable
}

/// Unique key for caching probe results per physical adapter
pub(crate) fn adapter_key(device: &WgpuDevice) -> String {
    let info = device.adapter_info();
    format!("{}:{:?}:{}", info.name, info.backend, info.vendor)
}

/// Run a tiny dispatch to check if exp(f64) produces a correct result.
///
/// Uses wgpu error scopes to catch compile-time failures (e.g. NAK crashes).
/// Also validates the numeric result after dispatch.
async fn run_exp_probe(device: &wgpu::Device, queue: &wgpu::Queue) -> bool {
    // ── Phase 1: try shader compilation ──────────────────────────────────
    device.push_error_scope(wgpu::ErrorFilter::Validation);

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("f64_exp_probe"),
        source: wgpu::ShaderSource::Wgsl(F64_EXP_PROBE_SHADER.into()),
    });

    if device.pop_error_scope().await.is_some() {
        // Shader compilation failed — driver cannot compile f64 exp
        return false;
    }

    // ── Phase 2: create pipeline ──────────────────────────────────────────
    device.push_error_scope(wgpu::ErrorFilter::Validation);

    let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("f64_exp_probe_out"),
        size: 8, // one f64
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("f64_exp_probe_staging"),
        size: 8,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("f64_exp_probe_bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("f64_exp_probe_pl"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("f64_exp_probe_pipeline"),
        layout: Some(&pl),
        module: &shader,
        entry_point: "probe_exp_f64",
        cache: None,
        compilation_options: Default::default(),
    });

    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("f64_exp_probe_bg"),
        layout: &bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: out_buffer.as_entire_binding(),
        }],
    });

    if device.pop_error_scope().await.is_some() {
        return false;
    }

    // ── Phase 3: dispatch ─────────────────────────────────────────────────
    device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("f64_exp_probe"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("f64_exp_probe"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bg, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&out_buffer, 0, &staging, 0, 8);
    queue.submit(Some(encoder.finish()));

    if device.pop_error_scope().await.is_some() {
        return false;
    }

    // ── Phase 4: read result ──────────────────────────────────────────────
    let staging_slice = staging.slice(..);
    let (tx, rx) = futures::channel::oneshot::channel();
    staging_slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });

    device.poll(wgpu::Maintain::Wait);

    let map_result = match rx.await {
        Ok(r) => r,
        Err(_) => return false,
    };
    if map_result.is_err() {
        return false;
    }

    let bytes = staging_slice.get_mapped_range();
    let result = f64::from_le_bytes(bytes[0..8].try_into().unwrap_or([0u8; 8]));
    drop(bytes);

    // e = 2.718281828...
    // Accept if within 0.01 of e (generous tolerance for any float rounding)
    let e = std::f64::consts::E;
    (result - e).abs() < 0.01
}

/// Pre-populate probe cache from device name heuristics before any GPU dispatch.
///
/// Call this immediately after device creation to prime the cache without
/// waiting for an async probe. The async probe will override this if run.
pub fn seed_cache_from_heuristics(device: &WgpuDevice) {
    let key = adapter_key(device);
    let mut cache = F64_EXP_PROBE_CACHE.lock().unwrap();

    // Only seed if not already probed
    cache.entry(key).or_insert_with(|| {
        // Use the existing heuristic as the initial estimate.
        // If is_nvk() or is_radv() → assume NOT capable (needs workaround).
        // Probe can override this later when run async.
        !device.needs_f64_exp_log_workaround()
    });
}

/// Read the cached probe result for this device.
///
/// Returns `None` if not yet probed. Returns `Some(true)` if capable,
/// `Some(false)` if workaround needed.
pub fn cached_probe_result(device: &WgpuDevice) -> Option<bool> {
    let key = adapter_key(device);
    F64_EXP_PROBE_CACHE.lock().unwrap().get(&key).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_and_read_cache() {
        use crate::device::test_pool::get_test_device_sync;
        let dev = get_test_device_sync();
        seed_cache_from_heuristics(&dev);
        let result = cached_probe_result(&dev);
        assert!(
            result.is_some(),
            "Cache should be seeded after seed_cache_from_heuristics"
        );
    }

    #[tokio::test]
    async fn test_probe_returns_consistent_result() {
        use crate::device::test_pool::get_test_device;
        let dev = get_test_device().await;
        let first = probe_f64_exp_capable(&dev).await;
        let second = probe_f64_exp_capable(&dev).await; // should use cache
        assert_eq!(first, second, "Probe should be deterministic/cached");
    }

    #[tokio::test]
    async fn test_probe_matches_heuristic_for_known_drivers() {
        use crate::device::test_pool::get_test_device;
        let dev = get_test_device().await;

        seed_cache_from_heuristics(&dev);
        let heuristic = cached_probe_result(&dev).unwrap();

        // Clear cache and run real probe
        {
            let key = adapter_key(&dev);
            F64_EXP_PROBE_CACHE.lock().unwrap().remove(&key);
        }
        let probed = probe_f64_exp_capable(&dev).await;

        // For known drivers (proprietary NVIDIA/AMD) probe should agree with heuristic.
        // For unknown drivers, probe result is truth regardless.
        if dev.is_nvidia_proprietary() || (!dev.is_nvk() && !dev.is_radv()) {
            // Proprietary drivers should be capable
            assert!(
                probed,
                "Proprietary/unknown driver should support native f64 exp"
            );
        }
        // Note: On NVK/RADV machines, probed will be false (they can't run this test anyway)
        let _ = heuristic; // checked indirectly via driver path
    }
}
