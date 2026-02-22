# GPU-Direct Display Architecture Exploration

**Date**: February 17, 2026  
**Status**: Conceptual Exploration  
**Origin**: Deep debt discussion — GPU-resident compute + gaming engine patterns

---

## The Insight

> "GPUs have an HDMI output and the CPU feeds input from mouse etc. Can we investigate gaming engine strategies? Conceptually can we output from the GPU if we choose? Newer even has USB-C. So maybe treating it as one-way can be a more efficient starting point?"

This observation highlights a fundamental asymmetry in GPU architecture:

```
┌─────────────────────────────────────────────────────────────────┐
│                      TRADITIONAL VIEW                           │
│                                                                 │
│  CPU ──PCIe──▶ GPU ──PCIe──▶ CPU ──RAM──▶ Display              │
│       upload      compute     readback    scanout               │
│                                                                 │
│  Problem: PCIe round-trip is the bottleneck (hotSpring 70× slow)│
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                   GAMING ENGINE VIEW                            │
│                                                                 │
│  CPU ──input──▶ GPU ══════════════════════▶ Display            │
│      (mouse/kb)     compute → render → scanout                  │
│                           (stays on GPU)                        │
│                                                                 │
│  Solution: Data stays GPU-resident, only outputs to display     │
└─────────────────────────────────────────────────────────────────┘
```

---

## Hardware Reality

### GPU Output Paths

| Path | Protocol | Bandwidth | CPU Involvement |
|------|----------|-----------|-----------------|
| HDMI 2.1 | TMDS/FRL | 48 Gbps | None (scanout direct) |
| DisplayPort 2.1 | DP | 77+ Gbps | None (scanout direct) |
| USB-C (DP Alt) | DisplayPort | 77+ Gbps | None (same as DP) |
| Thunderbolt | PCIe + DP | Variable | Minimal |

**Key insight**: The display output path **bypasses the CPU entirely**. The GPU writes to a framebuffer; the display controller scans it out to the physical connector.

### What Gaming Engines Do

1. **Upload once**: Textures, meshes, shaders → GPU memory
2. **Compute on GPU**: Physics, animation, culling (compute shaders)
3. **Render on GPU**: Rasterize to framebuffer
4. **Present directly**: Framebuffer → swapchain → display controller
5. **CPU only handles**: Input events, game logic decisions, audio

The CPU never sees pixel data during normal gameplay.

---

## Current ToadStool/BarraCuda Architecture

### What We Have

```
BarraCuda (Compute)                    Display Backend (DRM)
────────────────────                   ───────────────────────
wgpu::Device                           DRM Device
  └── Compute pipelines                  └── DumbBuffer (CPU-accessible)
  └── Storage buffers                    └── Framebuffer (DRM)
  └── Staging buffers (readback)         └── CRTC/Connector
                                         └── Mode setting
        │                                        │
        └──── GAP: No connection ────────────────┘
```

### The Gap

- BarraCuda creates devices with `compatible_surface: None` (compute-only)
- Display backend uses DRM "dumb buffers" (CPU-accessible, no GPU)
- No path from GPU compute → display without CPU readback

---

## Proposed Architecture: GPU-Direct Display

### Phase 1: Compute → Storage Texture

Add storage texture support to BarraCuda:

```rust
// New: Storage texture for 2D output
pub struct GpuTexture2D {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
}

impl GpuTexture2D {
    /// Create a storage texture for compute shader output
    pub fn new_storage(device: &WgpuDevice, width: u32, height: u32) -> Self {
        let texture = device.inner().create_texture(&wgpu::TextureDescriptor {
            label: Some("compute_output"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING 
                 | wgpu::TextureUsages::TEXTURE_BINDING
                 | wgpu::TextureUsages::COPY_SRC,
            ..Default::default()
        });
        // ...
    }
}
```

WGSL compute shader writing to texture:

```wgsl
@group(0) @binding(0) var output: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = vec2<i32>(gid.xy);
    let color = compute_pixel(pos);  // Your compute logic
    textureStore(output, pos, color);
}
```

### Phase 2: Surface + Swapchain (Optional Display)

Add optional display capability:

```rust
// Feature-gated display support
#[cfg(feature = "display")]
pub struct GpuDisplay {
    surface: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
}

impl WgpuDevice {
    /// Create device with display surface
    #[cfg(feature = "display")]
    pub async fn with_surface(window: &impl HasRawWindowHandle) -> Result<Self> {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window)?;
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),  // ← Key difference
            ..Default::default()
        }).await?;
        // ...
    }
}
```

### Phase 3: Compute → Display Pipeline

Zero-copy visualization path:

```rust
/// GPU-resident visualization pipeline
pub struct VisualizationPipeline {
    compute_texture: GpuTexture2D,
    render_pipeline: wgpu::RenderPipeline,  // Full-screen quad
    bind_group: wgpu::BindGroup,
}

impl VisualizationPipeline {
    /// Run compute and present to display (no CPU readback)
    pub fn compute_and_present(&self, device: &WgpuDevice, surface: &wgpu::Surface) {
        let mut encoder = device.create_command_encoder();
        
        // 1. Compute pass writes to storage texture
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.compute_pipeline);
            pass.set_bind_group(0, &self.compute_bind_group, &[]);
            pass.dispatch_workgroups(width / 16, height / 16, 1);
        }
        
        // 2. Render pass samples texture → swapchain (still on GPU!)
        let frame = surface.get_current_texture().unwrap();
        {
            let view = frame.texture.create_view(&Default::default());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: true },
                    ..Default::default()
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.render_pipeline);
            pass.set_bind_group(0, &self.texture_bind_group, &[]);
            pass.draw(0..6, 0..1);  // Full-screen quad
        }
        
        // 3. Submit and present (GPU → display, no CPU!)
        device.queue().submit(Some(encoder.finish()));
        frame.present();
    }
}
```

---

## Use Cases

### 1. Real-Time MD Visualization

```
Particle Positions (GPU buffer)
        │
        ▼
Compute Shader: positions → colors → texture
        │
        ▼
Storage Texture (RGBA)
        │
        ▼
Full-Screen Quad → Swapchain → Display

CPU involvement: NONE (until user wants to save/export)
```

### 2. Scientific Heatmaps

```
Simulation Grid (GPU buffer)
        │
        ▼
Compute Shader: values → colormap → texture
        │
        ▼
Display (continuous update)

Use case: Temperature fields, density plots, flow visualization
```

### 3. Real-Time Optimization Progress

```
Optimization State (GPU)
        │
        ▼
Compute Shader: fitness landscape → texture
        │
        ▼
Display (shows convergence live)

Use case: Nelder-Mead, genetic algorithms, training curves
```

---

## Integration with Display Backend

### Option A: wgpu Surface (Wayland/X11)

wgpu can create surfaces for windowed display:

```rust
// winit or raw-window-handle integration
let window = winit::window::Window::new(&event_loop)?;
let surface = instance.create_surface(&window)?;
```

**Pros**: Cross-platform, well-tested  
**Cons**: Requires window system (X11/Wayland compositor)

### Option B: wgpu + DRM (Headless/Kiosk)

For direct scanout without compositor:

```rust
// Hypothetical: wgpu with DRM backend
let drm_device = toadstool_display::drm::Device::open("/dev/dri/card0")?;
let surface = wgpu_drm::Surface::from_drm(drm_device)?;
```

**Pros**: No compositor overhead, direct scanout  
**Cons**: Requires wgpu-drm integration (not standard)

### Option C: Hybrid (Current + Future)

Keep DRM backend for software rendering (egui, petalTongue), add wgpu surface for GPU-accelerated visualization.

```
┌─────────────────────────────────────────────────────────┐
│                    ToadStool Display                    │
├─────────────────────────────────────────────────────────┤
│  Path A: Software Rendering (current)                   │
│    CPU → DumbBuffer → DRM Framebuffer → Display         │
│    Use case: UI (egui), text, 2D graphics               │
│                                                         │
│  Path B: GPU-Direct (new)                               │
│    GPU Compute → Storage Texture → Swapchain → Display  │
│    Use case: Scientific visualization, simulations      │
└─────────────────────────────────────────────────────────┘
```

---

## Performance Impact

### Current (CPU Readback)

```
hotSpring L2: GPU eigensolve → CPU readback → CPU physics → repeat
             └── 40.9 min (70× slower than pure CPU)
```

### With GPU-Direct Display

```
MD Visualization: GPU physics → GPU render → Display
                 └── No readback on hot path
                 └── CPU only for: input, save to disk, parameter changes
```

### Bandwidth Comparison

| Path | Bandwidth | Latency |
|------|-----------|---------|
| GPU VRAM | 1-3 TB/s | ~ns |
| PCIe 5.0 x16 | 64 GB/s | ~μs |
| Display (4K60 RGBA) | 2 GB/s | N/A (async scanout) |

The display path is **not the bottleneck** — it's the PCIe readback.

---

## Implementation Roadmap

### Phase 0: Research (Current)
- [x] Document gaming engine patterns
- [x] Identify wgpu capabilities
- [x] Design GPU-direct architecture

### Phase 1: Storage Textures
- [ ] Add `GpuTexture2D` to BarraCuda
- [ ] WGSL templates for texture output
- [ ] Test compute → texture path

### Phase 2: Optional Surface
- [ ] Feature-gate display support
- [ ] winit integration for windowed mode
- [ ] Full-screen quad shader

### Phase 3: Visualization Primitives
- [ ] Colormap shaders (viridis, plasma, etc.)
- [ ] Particle renderer (instanced points)
- [ ] Heatmap renderer (interpolated grid)

### Phase 4: DRM Integration (Advanced)
- [ ] wgpu-drm backend exploration
- [ ] Zero-copy scanout
- [ ] VSync/frame pacing

---

## Key Insight Summary

**The one-way model is correct**: Treat GPU as a compute+output device, with CPU only providing input (parameters, user events). This matches:

1. **Hardware reality**: GPU has direct display output
2. **Gaming engine success**: Decades of optimization around this model
3. **Scientific compute needs**: Visualization without readback penalty

**Next step**: Implement Phase 1 (storage textures) as foundation for GPU-direct visualization.

---

## References

- wgpu texture storage: https://docs.rs/wgpu/latest/wgpu/enum.TextureUsages.html
- Vulkan swapchain: https://www.khronos.org/registry/vulkan/specs/1.3/html/vkspec.html#_wsi_swapchain
- DRM/KMS: https://www.kernel.org/doc/html/latest/gpu/drm-kms.html
- NVIDIA GPUDirect: https://developer.nvidia.com/gpudirect

---

*From the ToadStool evolution desk*
