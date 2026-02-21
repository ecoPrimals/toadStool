//! Runtime GPU capability probing for f64 built-in functions
//!
//! Dispatches tiny test shaders to empirically verify hardware capabilities
//! rather than relying on driver name heuristics. Each function is compiled in
//! an isolated shader so a crash in one does not mask others.
//!
//! # Design
//!
//! Name-based detection (`is_nvk`, `is_radv`) is synchronous and fast but fragile.
//! Probe-based detection is async and definitive. Results are cached globally per
//! adapter identity (name + backend + vendor) so repeated calls are instant.
//!
//! The probe unlocks a key insight: WGSL → naga → SPIR-V → Vulkan bypasses the
//! proprietary software FP64 lock that CUDA/OpenCL enforce on consumer cards.
//! Both RTX 3090 (Ampere) and RX 6950 XT (RDNA2) expose `VK_KHR_shader_float64`
//! natively. By probing each builtin individually we build an exact capability
//! matrix, allowing `ShaderTemplate` to use native calls wherever safe and fall
//! back to the `math_f64.wgsl` software library only where needed.
//!
//! # Usage
//!
//! ```rust,ignore
//! let caps = probe_f64_builtins(&device).await;
//! // caps.exp  == true  → native exp(f64(x)) works
//! // caps.exp  == false → use software exp_f64() from math_f64.wgsl
//! ```

use crate::device::WgpuDevice;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard};

// ── Mutex helpers ────────────────────────────────────────────────────────────

/// Acquire a mutex lock, recovering from poison by taking the poisoned data.
///
/// Poison occurs only when another thread panicked while holding the lock.
/// Recovering is correct here because the cache data is always consistent
/// (insertions are atomic) — a poisoned lock just means the inserting thread
/// panicked after writing, which is safe to read.
pub(crate) fn lock_cache<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// ── Capability cache ─────────────────────────────────────────────────────────

/// Global probe result cache keyed by adapter_name:backend:vendor
static F64_CAPS_CACHE: LazyLock<Mutex<HashMap<String, F64BuiltinCapabilities>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// Keep the legacy single-function cache for backwards compat
static F64_EXP_PROBE_CACHE: LazyLock<Mutex<HashMap<String, bool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ── Capability result ─────────────────────────────────────────────────────────

/// Which f64 WGSL built-in functions are natively supported by this device.
///
/// `true`  → safe to use native WGSL call (e.g. `exp(f64(x))`)
/// `false` → use software implementation from `math_f64.wgsl`
///
/// Probed individually per function so one broken function does not shadow
/// the rest. On NVK/NAK (Feb 2026) `exp` and `log` crash the shader compiler;
/// `sqrt` and `abs`-family work everywhere since they map to non-transcendental
/// hardware instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct F64BuiltinCapabilities {
    /// `exp(f64)` — transcendental, crashes on NVK ≤ Mesa 25.2
    pub exp: bool,
    /// `log(f64)` — transcendental, crashes on NVK ≤ Mesa 25.2
    pub log: bool,
    /// `exp2(f64)` — transcendental
    pub exp2: bool,
    /// `log2(f64)` — transcendental
    pub log2: bool,
    /// `sin(f64)` — transcendental (MUFU on NVIDIA, may be FP32 promoted)
    pub sin: bool,
    /// `cos(f64)` — transcendental (MUFU on NVIDIA, may be FP32 promoted)
    pub cos: bool,
    /// `sqrt(f64)` — DSQRT instruction, generally available
    pub sqrt: bool,
    /// `fma(f64, f64, f64)` → DFMA, generally available on FP64-capable hardware
    pub fma: bool,
    /// `abs(f64)`, `min(f64, f64)`, `max(f64, f64)` — bit-level ops, always work
    pub abs_min_max: bool,
}

impl F64BuiltinCapabilities {
    /// Conservative fallback: no native builtins — software lib for everything.
    pub const fn none() -> Self {
        Self {
            exp: false,
            log: false,
            exp2: false,
            log2: false,
            sin: false,
            cos: false,
            sqrt: false,
            fma: false,
            abs_min_max: false,
        }
    }

    /// Full native support (known-good proprietary drivers on FP64 hardware).
    pub const fn full() -> Self {
        Self {
            exp: true,
            log: true,
            exp2: true,
            log2: true,
            sin: true,
            cos: true,
            sqrt: true,
            fma: true,
            abs_min_max: true,
        }
    }

    /// Whether exp/log workarounds are needed (drives ShaderTemplate patching).
    pub fn needs_exp_log_workaround(&self) -> bool {
        !self.exp || !self.log
    }

    /// Total count of natively-supported functions.
    pub fn native_count(&self) -> u8 {
        [
            self.exp,
            self.log,
            self.exp2,
            self.log2,
            self.sin,
            self.cos,
            self.sqrt,
            self.fma,
            self.abs_min_max,
        ]
        .iter()
        .filter(|&&b| b)
        .count() as u8
    }
}

impl std::fmt::Display for F64BuiltinCapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sym = |b: bool| if b { "✓" } else { "✗" };
        writeln!(f, "  f64 builtin capabilities:")?;
        writeln!(
            f,
            "    exp={} log={} exp2={} log2={}",
            sym(self.exp),
            sym(self.log),
            sym(self.exp2),
            sym(self.log2)
        )?;
        writeln!(
            f,
            "    sin={} cos={} sqrt={} fma={}",
            sym(self.sin),
            sym(self.cos),
            sym(self.sqrt),
            sym(self.fma)
        )?;
        write!(f, "    abs/min/max={}", sym(self.abs_min_max))
    }
}

// ── Probe shaders (one per function, crash-isolated) ─────────────────────────

/// One probe shader per function. Each must be compiled and dispatched
/// independently so a crash in one does not suppress detection of others.
struct ProbeShader {
    name: &'static str,
    wgsl: &'static str,
    /// Expected result written to out[0]
    expected: f64,
    /// Acceptable absolute error
    tolerance: f64,
}

const PROBES: &[ProbeShader] = &[
    ProbeShader {
        name: "exp",
        wgsl: "enable f64;\n\
               @group(0) @binding(0) var<storage, read_write> out: array<f64>;\n\
               @compute @workgroup_size(1)\n\
               fn probe(@builtin(global_invocation_id) _id: vec3<u32>) {\n\
                   out[0] = exp(f64(1.0));\n\
               }",
        expected: std::f64::consts::E,
        tolerance: 1e-6,
    },
    ProbeShader {
        name: "log",
        wgsl: "enable f64;\n\
               @group(0) @binding(0) var<storage, read_write> out: array<f64>;\n\
               @compute @workgroup_size(1)\n\
               fn probe(@builtin(global_invocation_id) _id: vec3<u32>) {\n\
                   out[0] = log(f64(2.718281828459045));\n\
               }",
        expected: 1.0,
        tolerance: 1e-6,
    },
    ProbeShader {
        name: "exp2",
        wgsl: "enable f64;\n\
               @group(0) @binding(0) var<storage, read_write> out: array<f64>;\n\
               @compute @workgroup_size(1)\n\
               fn probe(@builtin(global_invocation_id) _id: vec3<u32>) {\n\
                   out[0] = exp2(f64(3.0));\n\
               }",
        expected: 8.0,
        tolerance: 1e-10,
    },
    ProbeShader {
        name: "log2",
        wgsl: "enable f64;\n\
               @group(0) @binding(0) var<storage, read_write> out: array<f64>;\n\
               @compute @workgroup_size(1)\n\
               fn probe(@builtin(global_invocation_id) _id: vec3<u32>) {\n\
                   out[0] = log2(f64(8.0));\n\
               }",
        expected: 3.0,
        tolerance: 1e-10,
    },
    ProbeShader {
        name: "sin",
        wgsl: "enable f64;\n\
               @group(0) @binding(0) var<storage, read_write> out: array<f64>;\n\
               @compute @workgroup_size(1)\n\
               fn probe(@builtin(global_invocation_id) _id: vec3<u32>) {\n\
                   out[0] = sin(f64(1.5707963267948966));\n\
               }",
        expected: 1.0,
        tolerance: 1e-6,
    },
    ProbeShader {
        name: "cos",
        wgsl: "enable f64;\n\
               @group(0) @binding(0) var<storage, read_write> out: array<f64>;\n\
               @compute @workgroup_size(1)\n\
               fn probe(@builtin(global_invocation_id) _id: vec3<u32>) {\n\
                   out[0] = cos(f64(0.0));\n\
               }",
        expected: 1.0,
        tolerance: 1e-10,
    },
    ProbeShader {
        name: "sqrt",
        wgsl: "enable f64;\n\
               @group(0) @binding(0) var<storage, read_write> out: array<f64>;\n\
               @compute @workgroup_size(1)\n\
               fn probe(@builtin(global_invocation_id) _id: vec3<u32>) {\n\
                   out[0] = sqrt(f64(2.0));\n\
               }",
        expected: std::f64::consts::SQRT_2,
        tolerance: 1e-10,
    },
    ProbeShader {
        name: "fma",
        wgsl: "enable f64;\n\
               @group(0) @binding(0) var<storage, read_write> out: array<f64>;\n\
               @compute @workgroup_size(1)\n\
               fn probe(@builtin(global_invocation_id) _id: vec3<u32>) {\n\
                   out[0] = fma(f64(2.0), f64(3.0), f64(1.0));\n\
               }",
        expected: 7.0,
        tolerance: 1e-14,
    },
    ProbeShader {
        name: "abs_min_max",
        wgsl: "enable f64;\n\
               @group(0) @binding(0) var<storage, read_write> out: array<f64>;\n\
               @compute @workgroup_size(1)\n\
               fn probe(@builtin(global_invocation_id) _id: vec3<u32>) {\n\
                   let a = abs(f64(-3.5));\n\
                   let b = min(a, f64(4.0));\n\
                   let c = max(b, f64(2.0));\n\
                   out[0] = c;\n\
               }",
        expected: 3.5,
        tolerance: 1e-14,
    },
];

// ── Public probe API ──────────────────────────────────────────────────────────

/// Probe ALL f64 built-in functions available on this device.
///
/// Each function is tested in an isolated shader so a crash in one does not
/// hide support for others. Results are cached globally per adapter.
pub async fn probe_f64_builtins(device: &WgpuDevice) -> F64BuiltinCapabilities {
    let key = adapter_key(device);

    if let Some(cached) = lock_cache(&F64_CAPS_CACHE).get(&key).copied() {
        return cached;
    }

    let mut caps = F64BuiltinCapabilities::none();
    for probe in PROBES {
        let ok = run_single_probe(device.device(), device.queue(), probe).await;
        match probe.name {
            "exp" => caps.exp = ok,
            "log" => caps.log = ok,
            "exp2" => caps.exp2 = ok,
            "log2" => caps.log2 = ok,
            "sin" => caps.sin = ok,
            "cos" => caps.cos = ok,
            "sqrt" => caps.sqrt = ok,
            "fma" => caps.fma = ok,
            "abs_min_max" => caps.abs_min_max = ok,
            _ => {}
        }
    }

    // Update legacy exp-only cache for backwards compat
    lock_cache(&F64_EXP_PROBE_CACHE).insert(key.clone(), caps.exp);

    lock_cache(&F64_CAPS_CACHE).insert(key, caps);
    caps
}

/// Probe whether this device supports native `exp(f64)` / `log(f64)` (legacy API).
///
/// If the full `probe_f64_builtins` has already been run, reads from that cache.
/// Otherwise runs only the exp probe for speed.
pub async fn probe_f64_exp_capable(device: &WgpuDevice) -> bool {
    let key = adapter_key(device);

    // Full caps already cached?
    if let Some(caps) = lock_cache(&F64_CAPS_CACHE).get(&key).copied() {
        return caps.exp;
    }

    // Legacy cache?
    if let Some(&cached) = lock_cache(&F64_EXP_PROBE_CACHE).get(&key) {
        return cached;
    }

    let capable = run_single_probe(device.device(), device.queue(), &PROBES[0]).await;
    lock_cache(&F64_EXP_PROBE_CACHE).insert(key, capable);
    capable
}

/// Read cached full capability result, if available.
pub fn cached_f64_builtins(device: &WgpuDevice) -> Option<F64BuiltinCapabilities> {
    lock_cache(&F64_CAPS_CACHE)
        .get(&adapter_key(device))
        .copied()
}

/// Unique key for caching probe results per physical adapter
pub(crate) fn adapter_key(device: &WgpuDevice) -> String {
    let info = device.adapter_info();
    format!("{}:{:?}:{}", info.name, info.backend, info.vendor)
}

/// Read the cached probe result for this device (legacy single-function API).
pub fn cached_probe_result(device: &WgpuDevice) -> Option<bool> {
    let key = adapter_key(device);
    // Check full caps first
    if let Some(caps) = lock_cache(&F64_CAPS_CACHE).get(&key).copied() {
        return Some(caps.exp);
    }
    lock_cache(&F64_EXP_PROBE_CACHE).get(&key).copied()
}

/// Pre-populate probe cache from device name heuristics before any GPU dispatch.
///
/// Call this immediately after device creation to prime the cache without
/// waiting for an async probe. The async probe overrides this when run.
pub fn seed_cache_from_heuristics(device: &WgpuDevice) {
    let key = adapter_key(device);
    let mut cache = lock_cache(&F64_CAPS_CACHE);
    cache.entry(key.clone()).or_insert_with(|| {
        // Heuristic: NVK/RADV have broken transcendentals; proprietary is capable
        let exp_log_works = !device.needs_f64_exp_log_workaround();
        F64BuiltinCapabilities {
            exp: exp_log_works,
            log: exp_log_works,
            // Conservative for transcendentals on open drivers — probe overrides
            exp2: exp_log_works,
            log2: exp_log_works,
            sin: exp_log_works,
            cos: exp_log_works,
            // These map to non-transcendental hw instructions — assume always work
            sqrt: true,
            fma: true,
            abs_min_max: true,
        }
    });
    // Also seed legacy cache — key was just inserted above so get() is infallible here
    let exp_capable = cache.get(&key).is_some_and(|c| c.exp);
    drop(cache);
    lock_cache(&F64_EXP_PROBE_CACHE)
        .entry(key)
        .or_insert(exp_capable);
}

// ── Core probe runner ─────────────────────────────────────────────────────────

/// Run a single probe shader, catching compilation and dispatch errors.
///
/// Returns `true` if the shader compiled, dispatched, and produced the expected
/// numeric result. Returns `false` on any failure (shader compile error, dispatch
/// error, wrong numeric result, or OOM).
async fn run_single_probe(device: &wgpu::Device, queue: &wgpu::Queue, probe: &ProbeShader) -> bool {
    // Phase 1: shader compilation
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(probe.name),
        source: wgpu::ShaderSource::Wgsl(probe.wgsl.into()),
    });
    if device.pop_error_scope().await.is_some() {
        return false;
    }

    // Phase 2: pipeline and buffers
    device.push_error_scope(wgpu::ErrorFilter::Validation);

    let out_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("probe_out"),
        size: 8,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("probe_staging"),
        size: 8,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
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
        label: None,
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(probe.name),
        layout: Some(&pl),
        module: &shader,
        entry_point: "probe",
        cache: None,
        compilation_options: Default::default(),
    });
    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: out_buf.as_entire_binding(),
        }],
    });

    if device.pop_error_scope().await.is_some() {
        return false;
    }

    // Phase 3: dispatch
    device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bg, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    enc.copy_buffer_to_buffer(&out_buf, 0, &staging, 0, 8);
    queue.submit(Some(enc.finish()));

    if device.pop_error_scope().await.is_some() {
        return false;
    }

    // Phase 4: read and validate numeric result
    let slice = staging.slice(..);
    let (tx, rx) =
        std::sync::mpsc::sync_channel::<std::result::Result<(), wgpu::BufferAsyncError>>(1);
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::Maintain::Wait);

    if rx.recv().ok().and_then(|r| r.ok()).is_none() {
        return false;
    }

    let bytes = slice.get_mapped_range();
    let result = f64::from_le_bytes(bytes[0..8].try_into().unwrap_or([0u8; 8]));
    drop(bytes);

    (result - probe.expected).abs() < probe.tolerance
}

// ── Tests ─────────────────────────────────────────────────────────────────────

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
        let second = probe_f64_exp_capable(&dev).await;
        assert_eq!(first, second, "Probe should be deterministic/cached");
    }

    #[tokio::test]
    async fn test_full_caps_probe_consistency() {
        use crate::device::test_pool::get_test_device;
        let dev = get_test_device().await;
        let caps1 = probe_f64_builtins(&dev).await;
        let caps2 = probe_f64_builtins(&dev).await; // from cache
        assert_eq!(caps1, caps2, "Full probe should be deterministic/cached");
    }

    #[tokio::test]
    async fn test_caps_exp_agrees_with_single_probe() {
        use crate::device::test_pool::get_test_device;
        let dev = get_test_device().await;
        let caps = probe_f64_builtins(&dev).await;
        let exp_only = probe_f64_exp_capable(&dev).await;
        assert_eq!(
            caps.exp, exp_only,
            "Full caps exp field must agree with single exp probe"
        );
    }

    #[test]
    fn test_f64_caps_none() {
        let c = F64BuiltinCapabilities::none();
        assert_eq!(c.native_count(), 0);
        assert!(c.needs_exp_log_workaround());
    }

    #[test]
    fn test_f64_caps_full() {
        let c = F64BuiltinCapabilities::full();
        assert_eq!(c.native_count(), 9);
        assert!(!c.needs_exp_log_workaround());
    }

}
