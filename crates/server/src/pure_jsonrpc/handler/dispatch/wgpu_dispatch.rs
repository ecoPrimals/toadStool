// SPDX-License-Identifier: AGPL-3.0-or-later
//! wgpu-based compute dispatch for DRM-bound GPUs.
//!
//! This provides a Vulkan/wgpu dispatch path that complements the VFIO
//! `local_cylinder` path. Used for GPUs that are bound to the nvidia/amdgpu
//! DRM driver (e.g. display GPUs or GPUs without VFIO passthrough).
//!
//! The wgpu path accepts WGSL source and dispatches it through the standard
//! Vulkan compute pipeline with buffer readback.
//!
//! NOTE: This crate is compiled with `panic = "abort"`. Every wgpu error that
//! could panic (device lost, invalid pipeline) must be detected and handled
//! *before* calling panicking APIs like `pipeline.get_bind_group_layout()`.
//! Vulkan loader availability is checked upfront via `vulkan_loader_available()`.

#[cfg(feature = "gpu-discovery")]
use base64::Engine;

#[cfg(feature = "gpu-discovery")]
const DEVICE_LOST_SETTLE: std::time::Duration = std::time::Duration::from_millis(50);

#[cfg(feature = "gpu-discovery")]
struct WgpuDispatchContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_name: String,
    #[expect(dead_code, reason = "included in dispatch response metadata")]
    adapter_index: usize,
    spirv_passthrough: bool,
    device_lost: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// Selector for choosing which adapter to dispatch on.
#[cfg(feature = "gpu-discovery")]
#[derive(Debug, Clone)]
pub(super) enum AdapterSelector {
    /// First discrete GPU (default, backward-compatible).
    Default,
    /// Adapter by enumeration index.
    Index(usize),
    /// Adapter by name substring match.
    Name(String),
}

/// Pool of lazily-initialized wgpu adapter contexts for multi-GPU dispatch.
#[cfg(feature = "gpu-discovery")]
struct WgpuAdapterPool {
    contexts: std::sync::Mutex<Vec<Option<std::sync::Arc<WgpuDispatchContext>>>>,
    init_done: std::sync::atomic::AtomicBool,
}

#[cfg(feature = "gpu-discovery")]
impl WgpuAdapterPool {
    const fn new() -> Self {
        Self {
            contexts: std::sync::Mutex::new(Vec::new()),
            init_done: std::sync::atomic::AtomicBool::new(false),
        }
    }

    fn ensure_enumerated(&self) {
        if self.init_done.load(std::sync::atomic::Ordering::Acquire) {
            return;
        }

        let mut contexts = self.contexts.lock().unwrap_or_else(|e| e.into_inner());
        if self.init_done.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        let adapter_count = std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().ok()?;
            rt.block_on(async {
                if !toadstool_runtime_gpu::vulkan_loader_available() {
                    return Some(0);
                }
                let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                    backends: wgpu::Backends::VULKAN,
                    ..Default::default()
                });
                let adapters = instance.enumerate_adapters(wgpu::Backends::VULKAN).await;
                Some(adapters.len())
            })
        })
        .join()
        .ok()
        .flatten()
        .unwrap_or(0);

        contexts.resize_with(adapter_count, || None);
        self.init_done.store(true, std::sync::atomic::Ordering::Release);
        tracing::info!(adapter_count, "wgpu adapter pool: enumerated adapters");
    }

    fn get_or_init(&self, selector: &AdapterSelector) -> Option<std::sync::Arc<WgpuDispatchContext>> {
        self.ensure_enumerated();

        let pool_size = {
            let contexts = self.contexts.lock().unwrap_or_else(|e| e.into_inner());
            contexts.len()
        };

        if pool_size == 0 {
            return None;
        }

        let idx = match selector {
            AdapterSelector::Default => {
                // Return first already-initialized, or init index 0
                let contexts = self.contexts.lock().unwrap_or_else(|e| e.into_inner());
                contexts
                    .iter()
                    .position(|c| c.is_some())
                    .unwrap_or(0)
            }
            AdapterSelector::Index(i) => {
                if *i >= pool_size {
                    return None;
                }
                *i
            }
            AdapterSelector::Name(name) => {
                let contexts = self.contexts.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(pos) = contexts.iter().position(|c| {
                    c.as_ref()
                        .is_some_and(|ctx| ctx.adapter_name.contains(name.as_str()))
                }) {
                    pos
                } else {
                    drop(contexts);
                    self.init_adapter_by_name(name)?
                }
            }
        };

        self.init_adapter(idx)
    }

    fn init_adapter(&self, idx: usize) -> Option<std::sync::Arc<WgpuDispatchContext>> {
        {
            let contexts = self.contexts.lock().unwrap_or_else(|e| e.into_inner());
            if idx >= contexts.len() {
                return None;
            }
            if let Some(ref arc) = contexts[idx] {
                return Some(std::sync::Arc::clone(arc));
            }
        }

        let ctx = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().ok()?;
            rt.block_on(init_wgpu_at_index(idx))
        })
        .join()
        .ok()??;

        let arc = std::sync::Arc::new(ctx);
        let mut contexts = self.contexts.lock().unwrap_or_else(|e| e.into_inner());
        if idx >= contexts.len() {
            return None;
        }
        contexts[idx] = Some(std::sync::Arc::clone(&arc));
        Some(arc)
    }

    fn init_adapter_by_name(&self, name: &str) -> Option<usize> {
        let pool_size = {
            let contexts = self.contexts.lock().unwrap_or_else(|e| e.into_inner());
            contexts.len()
        };

        for i in 0..pool_size {
            if let Some(ctx) = self.init_adapter(i) {
                if ctx.adapter_name.contains(name) {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Number of adapters discovered (0 if not yet enumerated).
    fn adapter_count(&self) -> usize {
        self.ensure_enumerated();
        let contexts = self.contexts.lock().unwrap_or_else(|e| e.into_inner());
        contexts.len()
    }

    /// List adapter info for all initialized adapters.
    fn list_adapters(&self) -> Vec<(usize, String)> {
        self.ensure_enumerated();
        let contexts = self.contexts.lock().unwrap_or_else(|e| e.into_inner());
        contexts
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.as_ref().map(|ctx| (i, ctx.adapter_name.clone())))
            .collect()
    }
}

#[cfg(feature = "gpu-discovery")]
static WGPU_POOL: WgpuAdapterPool = WgpuAdapterPool::new();

#[cfg(feature = "gpu-discovery")]
async fn init_wgpu_at_index(target_idx: usize) -> Option<WgpuDispatchContext> {
    if !toadstool_runtime_gpu::vulkan_loader_available() {
        tracing::info!("wgpu dispatch unavailable (Vulkan loader not found)");
        return None;
    }

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    let adapters: Vec<_> = instance.enumerate_adapters(wgpu::Backends::VULKAN).await;

    // For AdapterSelector::Default (target_idx == 0 from the old path), prefer
    // the first discrete GPU. For explicit indices, use the indexed adapter.
    let adapter = if target_idx == 0 {
        adapters
            .into_iter()
            .enumerate()
            .find(|(_, a)| a.get_info().device_type == wgpu::DeviceType::DiscreteGpu)
            .map(|(_, a)| a)?
    } else {
        adapters.into_iter().nth(target_idx)?
    };

    let info = adapter.get_info();
    let adapter_name = info.name.clone();
    let supported_features = adapter.features();
    let spirv_passthrough =
        supported_features.contains(wgpu::Features::EXPERIMENTAL_PASSTHROUGH_SHADERS);
    tracing::info!(
        adapter = %adapter_name,
        index = target_idx,
        vendor = format_args!("0x{:x}", info.vendor),
        device = format_args!("0x{:x}", info.device),
        spirv_passthrough,
        "wgpu dispatch: adapter selected"
    );

    let mut required_features = wgpu::Features::empty();
    if spirv_passthrough {
        required_features |= wgpu::Features::EXPERIMENTAL_PASSTHROUGH_SHADERS;
    } else {
        tracing::warn!(
            adapter = %adapter_name,
            "EXPERIMENTAL_PASSTHROUGH_SHADERS not supported — wgpu dispatch will use naga/WGSL path"
        );
    }

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("toadstool-wgpu-dispatch"),
            required_features,
            ..Default::default()
        })
        .await
        .ok()?;

    let device_lost = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let lost_flag = std::sync::Arc::clone(&device_lost);

    device.set_device_lost_callback(move |reason, msg| {
        tracing::error!(?reason, message = %msg, "wgpu device lost");
        lost_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    device.on_uncaptured_error(std::sync::Arc::new(|error| {
        tracing::error!(error = %error, "wgpu uncaptured device error");
    }));

    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    tokio::time::sleep(DEVICE_LOST_SETTLE).await;

    if device_lost.load(std::sync::atomic::Ordering::SeqCst) {
        tracing::error!(
            adapter = %adapter_name,
            "wgpu device was lost immediately after creation — disabling this adapter"
        );
        return None;
    }

    tracing::info!(
        adapter = %adapter_name,
        index = target_idx,
        spirv_passthrough,
        "wgpu dispatch: device initialized"
    );

    Some(WgpuDispatchContext {
        device,
        queue,
        adapter_name,
        adapter_index: target_idx,
        spirv_passthrough,
        device_lost,
    })
}

/// Attempt wgpu compute dispatch with the given SPIR-V binary and buffers.
///
/// Returns `Some(Ok(json))` on success, `Some(Err(msg))` on dispatch failure,
/// or `None` if wgpu is not available. Uses the default adapter (first discrete GPU).
#[cfg(feature = "gpu-discovery")]
#[expect(dead_code, reason = "convenience wrapper; non-featured path uses this directly")]
pub(super) fn try_wgpu_dispatch(
    binary: &[u8],
    wgsl_source: Option<&str>,
    workgroup_size: [u32; 3],
    buffer_descs: &serde_json::Value,
) -> Option<Result<serde_json::Value, String>> {
    try_wgpu_dispatch_on_adapter(&AdapterSelector::Default, binary, wgsl_source, workgroup_size, buffer_descs)
}

#[cfg(not(feature = "gpu-discovery"))]
pub(super) fn try_wgpu_dispatch(
    _binary: &[u8],
    _wgsl_source: Option<&str>,
    _workgroup_size: [u32; 3],
    _buffer_descs: &serde_json::Value,
) -> Option<Result<serde_json::Value, String>> {
    None
}

/// Dispatch on a specific adapter selected by index or name.
#[cfg(feature = "gpu-discovery")]
pub(super) fn try_wgpu_dispatch_on_adapter(
    selector: &AdapterSelector,
    binary: &[u8],
    wgsl_source: Option<&str>,
    workgroup_size: [u32; 3],
    buffer_descs: &serde_json::Value,
) -> Option<Result<serde_json::Value, String>> {
    let ctx = WGPU_POOL.get_or_init(selector)?;

    if ctx.device_lost.load(std::sync::atomic::Ordering::SeqCst) {
        return Some(Err(format!(
            "wgpu device lost on adapter {} — cannot dispatch",
            ctx.adapter_name
        )));
    }

    Some(run_wgpu_dispatch(
        &ctx,
        binary,
        wgsl_source,
        workgroup_size,
        buffer_descs,
    ))
}

/// Number of available wgpu adapters.
#[cfg(feature = "gpu-discovery")]
pub(super) fn wgpu_adapter_count() -> usize {
    WGPU_POOL.adapter_count()
}

/// List initialized adapters (index, name).
#[cfg(feature = "gpu-discovery")]
pub(super) fn wgpu_list_adapters() -> Vec<(usize, String)> {
    WGPU_POOL.list_adapters()
}

#[cfg(feature = "gpu-discovery")]
fn run_wgpu_dispatch(
    ctx: &WgpuDispatchContext,
    binary: &[u8],
    wgsl_source: Option<&str>,
    workgroup_size: [u32; 3],
    buffer_descs: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Validate SPIR-V magic (0x07230203 LE) before using passthrough.
    // The shader compiler may return its internal binary format instead of SPIR-V;
    // feeding non-SPIR-V to the Vulkan driver causes immediate device loss.
    let is_valid_spirv = binary.len() >= 4 && {
        let magic = u32::from_le_bytes([binary[0], binary[1], binary[2], binary[3]]);
        magic == 0x0723_0203
    };

    let shader = if ctx.spirv_passthrough && is_valid_spirv {
        tracing::info!(
            spirv_bytes = binary.len(),
            "wgpu dispatch: SPIR-V passthrough"
        );
        toadstool_runtime_gpu::shader_spirv::create_spirv_shader_module(
            &ctx.device,
            "toadstool_wgpu_dispatch",
            binary,
        )
        .map_err(|e| format!("SPIR-V validation failed: {e}"))?
    } else if let Some(wgsl) = wgsl_source {
        if !is_valid_spirv && !binary.is_empty() {
            tracing::info!(
                binary_magic = format_args!(
                    "0x{:08x}",
                    if binary.len() >= 4 {
                        u32::from_le_bytes([binary[0], binary[1], binary[2], binary[3]])
                    } else {
                        0
                    }
                ),
                "wgpu dispatch: binary is not SPIR-V — using naga/WGSL path"
            );
        }
        ctx.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("toadstool_wgpu_dispatch_wgsl"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(wgsl)),
            })
    } else {
        return Err("Binary is not SPIR-V and no WGSL source provided for naga compilation".into());
    };

    // Use an explicit PipelineLayout rather than relying on
    // pipeline.get_bind_group_layout() which panics if the pipeline is invalid
    // (fatal under panic=abort).
    let bind_group_layout = ctx
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("toadstool_wgpu_bgl"),
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

    let pipeline_layout = ctx
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("toadstool_wgpu_pl"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

    let pipeline = ctx
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("toadstool_wgpu_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

    // Check device lost after pipeline creation (driver errors surface here).
    let _ = ctx.device.poll(wgpu::PollType::wait_indefinitely());
    if ctx.device_lost.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("wgpu device lost during pipeline creation".into());
    }

    let buf_arr = buffer_descs.as_array();
    let mut gpu_buffers: Vec<wgpu::Buffer> = Vec::new();
    let mut staging_buffers: Vec<Option<wgpu::Buffer>> = Vec::new();
    let mut readback_meta: Vec<(usize, u64, bool)> = Vec::new();

    if let Some(descs) = buf_arr {
        for (i, desc) in descs.iter().enumerate() {
            let size = desc
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if size == 0 {
                continue;
            }

            let direction = desc
                .get("direction")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("inout");

            let needs_upload = matches!(direction, "in" | "inout" | "readwrite");
            let needs_readback = matches!(direction, "out" | "inout" | "readwrite");

            let mut usage = wgpu::BufferUsages::STORAGE;
            if needs_readback {
                usage |= wgpu::BufferUsages::COPY_SRC;
            }
            if needs_upload {
                usage |= wgpu::BufferUsages::COPY_DST;
            }

            let gpu_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("buf_{i}")),
                size,
                usage,
                mapped_at_creation: false,
            });

            if needs_upload
                && let Some(data) = desc.get("data").and_then(serde_json::Value::as_array)
            {
                let bytes: Vec<u8> = data.iter().map(|v| v.as_u64().unwrap_or(0) as u8).collect();
                ctx.queue.write_buffer(&gpu_buf, 0, &bytes);
            }

            let staging = if needs_readback {
                Some(ctx.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("staging_{i}")),
                    size,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }))
            } else {
                None
            };

            readback_meta.push((i, size, needs_readback));
            gpu_buffers.push(gpu_buf);
            staging_buffers.push(staging);
        }
    }

    let bind_group_entries: Vec<wgpu::BindGroupEntry> = gpu_buffers
        .iter()
        .enumerate()
        .map(|(i, buf)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: buf.as_entire_binding(),
        })
        .collect();

    let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &bind_group_entries,
    });

    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(workgroup_size[0], workgroup_size[1], workgroup_size[2]);
    }

    for (idx, (_meta_idx, size, needs_readback)) in readback_meta.iter().enumerate() {
        if *needs_readback && let Some(staging) = &staging_buffers[idx] {
            encoder.copy_buffer_to_buffer(&gpu_buffers[idx], 0, staging, 0, *size);
        }
    }

    ctx.queue.submit(Some(encoder.finish()));
    let _ = ctx.device.poll(wgpu::PollType::wait_indefinitely());

    if ctx.device_lost.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("wgpu device lost during dispatch".into());
    }

    let mut readback_results: Vec<serde_json::Value> = Vec::new();
    for (idx, (_meta_idx, size, needs_readback)) in readback_meta.iter().enumerate() {
        if *needs_readback && let Some(staging) = &staging_buffers[idx] {
            let slice = staging.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
            let _ = ctx.device.poll(wgpu::PollType::wait_indefinitely());

            match rx.recv() {
                Ok(Ok(())) => {
                    let data = slice.get_mapped_range();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&*data);
                    drop(data);
                    staging.unmap();
                    readback_results.push(serde_json::json!({
                        "size": size,
                        "data_b64": b64,
                    }));
                }
                Ok(Err(e)) => {
                    readback_results.push(serde_json::json!({
                        "size": size,
                        "error": format!("map failed: {e}"),
                    }));
                }
                Err(e) => {
                    readback_results.push(serde_json::json!({
                        "size": size,
                        "error": format!("recv failed: {e}"),
                    }));
                }
            }
        }
    }

    Ok(serde_json::json!({
        "dispatch_path": "wgpu",
        "status": "completed",
        "adapter": ctx.adapter_name,
        "buffers": readback_results,
    }))
}
