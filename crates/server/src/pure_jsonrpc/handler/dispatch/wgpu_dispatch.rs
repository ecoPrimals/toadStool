// SPDX-License-Identifier: AGPL-3.0-or-later
//! wgpu-based compute dispatch for DRM-bound GPUs.
//!
//! This provides a Vulkan/wgpu dispatch path that complements the VFIO
//! `local_cylinder` path. Used for GPUs that are bound to the nvidia/amdgpu
//! DRM driver (e.g. display GPUs or GPUs without VFIO passthrough).
//!
//! The wgpu path accepts compiled SPIR-V binaries and dispatches them
//! through the standard Vulkan compute pipeline with buffer readback.

#[cfg(feature = "gpu-discovery")]
use base64::Engine;

#[cfg(feature = "gpu-discovery")]
struct WgpuDispatchContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_name: String,
}


#[cfg(feature = "gpu-discovery")]
static WGPU_CTX: std::sync::OnceLock<Option<WgpuDispatchContext>> = std::sync::OnceLock::new();

#[cfg(feature = "gpu-discovery")]
fn get_or_init_wgpu() -> Option<&'static WgpuDispatchContext> {
    WGPU_CTX
        .get_or_init(|| {
            let rt = tokio::runtime::Runtime::new().ok()?;
            rt.block_on(init_wgpu())
        })
        .as_ref()
}

#[cfg(feature = "gpu-discovery")]
async fn init_wgpu() -> Option<WgpuDispatchContext> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    let adapters: Vec<_> = instance.enumerate_adapters(wgpu::Backends::VULKAN);
    let adapter = adapters
        .into_iter()
        .find(|a| a.get_info().device_type == wgpu::DeviceType::DiscreteGpu)?;

    let info = adapter.get_info();
    let adapter_name = info.name.clone();
    tracing::info!(
        adapter = %adapter_name,
        vendor = format_args!("0x{:x}", info.vendor),
        device = format_args!("0x{:x}", info.device),
        "wgpu dispatch: adapter selected"
    );

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("toadstool-wgpu-dispatch"),
                required_features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
                ..Default::default()
            },
            None,
        )
        .await
        .ok()?;

    Some(WgpuDispatchContext {
        device,
        queue,
        adapter_name,
    })
}

/// Attempt wgpu compute dispatch with the given SPIR-V binary and buffers.
///
/// Returns `Some(Ok(json))` on success, `Some(Err(msg))` on dispatch failure,
/// or `None` if wgpu is not available.
#[cfg(feature = "gpu-discovery")]
pub(super) fn try_wgpu_dispatch(
    binary: &[u8],
    workgroup_size: [u32; 3],
    buffer_descs: &serde_json::Value,
) -> Option<Result<serde_json::Value, String>> {
    let ctx = get_or_init_wgpu()?;
    Some(run_wgpu_dispatch(ctx, binary, workgroup_size, buffer_descs))
}

#[cfg(not(feature = "gpu-discovery"))]
pub(super) fn try_wgpu_dispatch(
    _binary: &[u8],
    _workgroup_size: [u32; 3],
    _buffer_descs: &serde_json::Value,
) -> Option<Result<serde_json::Value, String>> {
    None
}

#[cfg(feature = "gpu-discovery")]
fn run_wgpu_dispatch(
    ctx: &WgpuDispatchContext,
    binary: &[u8],
    workgroup_size: [u32; 3],
    buffer_descs: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let spirv_words: Vec<u32> = binary
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    #[allow(unsafe_code)]
    // SAFETY: SPIR-V has been compiled by coralReef (trusted primal).
    // SPIRV_SHADER_PASSTHROUGH feature is enabled on the device.
    let shader = unsafe {
        ctx.device
            .create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                label: Some("toadstool_wgpu_dispatch"),
                source: std::borrow::Cow::Borrowed(&spirv_words),
            })
    };

    let pipeline = ctx
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("toadstool_wgpu_pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        });

    let buf_arr = buffer_descs.as_array();
    let mut gpu_buffers: Vec<wgpu::Buffer> = Vec::new();
    let mut staging_buffers: Vec<Option<wgpu::Buffer>> = Vec::new();
    let mut readback_meta: Vec<(usize, u64, bool)> = Vec::new(); // (index, size, needs_readback)

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

            let needs_upload = matches!(direction, "in" | "inout");
            let needs_readback = matches!(direction, "out" | "inout");

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

            if needs_upload {
                if let Some(data) = desc.get("data").and_then(serde_json::Value::as_array) {
                    let bytes: Vec<u8> =
                        data.iter().map(|v| v.as_u64().unwrap_or(0) as u8).collect();
                    ctx.queue.write_buffer(&gpu_buf, 0, &bytes);
                }
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
        layout: &pipeline.get_bind_group_layout(0),
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
        if *needs_readback {
            if let Some(staging) = &staging_buffers[idx] {
                encoder.copy_buffer_to_buffer(&gpu_buffers[idx], 0, staging, 0, *size);
            }
        }
    }

    ctx.queue.submit(Some(encoder.finish()));
    ctx.device.poll(wgpu::Maintain::Wait);

    let mut readback_results: Vec<serde_json::Value> = Vec::new();
    for (idx, (_meta_idx, size, needs_readback)) in readback_meta.iter().enumerate() {
        if *needs_readback {
            if let Some(staging) = &staging_buffers[idx] {
                let slice = staging.slice(..);
                let (tx, rx) = std::sync::mpsc::channel();
                slice.map_async(wgpu::MapMode::Read, move |result| {
                    let _ = tx.send(result);
                });
                ctx.device.poll(wgpu::Maintain::Wait);

                let _ = ctx.device.poll(wgpu::Maintain::Wait);
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
    }

    Ok(serde_json::json!({
        "dispatch_path": "wgpu",
        "status": "completed",
        "adapter": ctx.adapter_name,
        "buffers": readback_results,
    }))
}
