# ToadStool Hardware Routing Layer

**Date**: February 17, 2026  
**Status**: Architectural Exploration  
**Origin**: Discussion on bidirectional GPU pipelines and hardware interconnect management

---

## The Vision

> "What would it look like if we used the HDMI to feed the GPU, and process then give to CPU? We are not rendering here so much as creating bidirectional pipelines that allow for more functional and controlled deployments... ToadStool can handle hardware routing (different than Songbird network routing)."

This proposes a **hardware routing layer** that treats all physical interconnects as managed data channels:

```
┌─────────────────────────────────────────────────────────────────────┐
│                     SEPARATION OF CONCERNS                          │
├─────────────────────────────────────────────────────────────────────┤
│  Songbird        │  Network routing (which machine, which endpoint) │
│  ToadStool       │  Hardware routing (which physical pipe)          │
│  BarraCUDA       │  Math (shaders, algorithms, compute)             │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Hardware Interconnect Inventory

### Current GPU Data Paths

| Path | Direction | Bandwidth | Typical Use | Latency |
|------|-----------|-----------|-------------|---------|
| **PCIe x16** | Bidirectional | 32-64 GB/s | Compute I/O | ~μs |
| **HDMI 2.1** | GPU → Display | 48 Gbps (6 GB/s) | Video out | ~ms (scanout) |
| **DP 2.1** | GPU → Display | 77 Gbps (9.6 GB/s) | Video out | ~ms (scanout) |
| **USB-C (DP Alt)** | GPU → Display | 77 Gbps | Video out | ~ms |
| **NVLink** | GPU ↔ GPU | 900 GB/s | Multi-GPU | ~ns |
| **Infinity Fabric** | GPU ↔ GPU | 800 GB/s | AMD Multi-GPU | ~ns |

### Capture Card Paths (GPU → System)

| Product | Input | Output | Bandwidth | GPU Direct? |
|---------|-------|--------|-----------|-------------|
| **Magewell Pro Capture** | HDMI/DP | PCIe x4 | ~2.4 GB/s | ✅ GPUDirect + DirectGMA |
| **Datapath VisionSC-DP2** | 2× DP 1.2 | PCIe x8 | 6 GB/s | ✅ |
| **Acasis Thunderbolt** | HDMI 4K60 | TB4 (40 Gbps) | 5 GB/s | Via TB |
| **Eco Capture M.2** | HDMI 4K60 | M.2 NVMe | 2.4 GB/s | ✅ |

**Key finding**: Capture cards with **GPUDirect/DirectGMA** can receive HDMI/DP input and write directly to GPU memory — bypassing CPU entirely.

---

## Bidirectional Pipeline Architecture

### Concept: Display Ports as Data Channels

```
┌─────────────────────────────────────────────────────────────────────┐
│                   GPU A (Compute)                                   │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  BarraCUDA Shaders                                           │  │
│  │  (eigensolve, forces, etc.)                                  │  │
│  └──────────────────────────────────────────────────────────────┘  │
│              │                                                      │
│              ▼                                                      │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  Pixel Encoder                                               │  │
│  │  (pack f64 data as RGBA pixels)                              │  │
│  └──────────────────────────────────────────────────────────────┘  │
│              │                                                      │
│              ▼ HDMI/DP OUT                                          │
└──────────────│──────────────────────────────────────────────────────┘
               │
               │ (Physical cable - continuous stream)
               │
┌──────────────▼──────────────────────────────────────────────────────┐
│                   Capture Card (PCIe)                               │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  GPUDirect Write                                             │  │
│  │  (direct to GPU B or system RAM)                             │  │
│  └──────────────────────────────────────────────────────────────┘  │
└──────────────│──────────────────────────────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────────────────────────────┐
│                   GPU B or CPU                                       │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  Pixel Decoder                                                │   │
│  │  (unpack RGBA to f64 data)                                    │   │
│  └──────────────────────────────────────────────────────────────┘   │
│              │                                                       │
│              ▼                                                       │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  Further Processing                                           │   │
│  └──────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────┘
```

### Why This Might Be Useful

1. **Continuous streaming**: Display outputs are optimized for constant data flow
2. **Parallel to PCIe**: Use display path while PCIe handles other traffic
3. **Latency characteristics**: Different from PCIe (async vs sync)
4. **Hardware isolation**: Separate path = separate failure domain

---

## ToadStool Hardware Router Design

### Abstraction Layer

```rust
/// Physical interconnect types managed by ToadStool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interconnect {
    /// PCIe direct memory access
    PcieDma { gen: u8, lanes: u8 },
    
    /// Display output as data channel
    DisplayOut { port: DisplayPort, resolution: Resolution },
    
    /// Capture card input (HDMI/DP → system)
    CaptureIn { card_id: u32, supports_gpu_direct: bool },
    
    /// NVLink (NVIDIA multi-GPU)
    NvLink { bridge_id: u32 },
    
    /// Infinity Fabric (AMD multi-GPU)
    InfinityFabric { link_id: u32 },
    
    /// Thunderbolt/USB4
    Thunderbolt { port: u8, gen: u8 },
    
    /// Network (delegated to Songbird)
    Network { endpoint: SocketAddr },
}

#[derive(Debug, Clone, Copy)]
pub enum DisplayPort {
    Hdmi { version: HdmiVersion },
    DisplayPort { version: DpVersion },
    UsbC { dp_alt_mode: bool },
}
```

### Route Selection

```rust
/// Hardware route for data transfer
pub struct HardwareRoute {
    pub source: DeviceId,
    pub destination: DeviceId,
    pub interconnect: Interconnect,
    pub bandwidth_gbps: f32,
    pub latency_us: f32,
    pub encoding: Option<DataEncoding>,
}

/// Data encoding for display-path transfers
pub enum DataEncoding {
    /// Pack f32 as RGBA8 (4 bytes per pixel)
    F32AsRgba8,
    
    /// Pack f64 as RG32F (8 bytes per pixel, needs HDR)
    F64AsRg32F,
    
    /// Pack 2× f64 as RGBA32F (16 bytes per pixel)
    F64x2AsRgba32F,
    
    /// Raw binary (no encoding, direct capture)
    RawBinary,
}

impl HardwareRouter {
    /// Select optimal route for data transfer
    pub fn select_route(
        &self,
        source: DeviceId,
        destination: DeviceId,
        data_size: usize,
        latency_requirement: LatencyRequirement,
    ) -> Result<HardwareRoute> {
        let available_routes = self.enumerate_routes(source, destination)?;
        
        match latency_requirement {
            LatencyRequirement::Minimum => {
                // Prefer: NVLink > PCIe > Thunderbolt > Display
                available_routes.into_iter()
                    .min_by_key(|r| (r.latency_us * 1000.0) as u64)
            }
            LatencyRequirement::Throughput => {
                // Prefer: NVLink > DisplayPath (streaming) > PCIe
                available_routes.into_iter()
                    .max_by_key(|r| (r.bandwidth_gbps * 1000.0) as u64)
            }
            LatencyRequirement::Parallel => {
                // Use display path to avoid PCIe contention
                available_routes.into_iter()
                    .find(|r| matches!(r.interconnect, Interconnect::DisplayOut { .. }))
            }
        }
        .ok_or(HardwareError::NoRouteAvailable)
    }
}
```

### Display-Path Data Transfer

```rust
/// Encode compute results as display output
pub struct DisplayDataEncoder {
    width: u32,
    height: u32,
    encoding: DataEncoding,
    compute_pipeline: wgpu::ComputePipeline,
}

impl DisplayDataEncoder {
    /// Pack f64 buffer as RGBA texture for display output
    pub fn encode_f64_buffer(
        &self,
        device: &WgpuDevice,
        input: &GpuBuffer<f64>,
        output_texture: &GpuTexture2D,
    ) {
        // WGSL shader packs f64 values as pixel colors
        // Each f64 → 2 RGBA8 pixels (8 bytes)
        // Or each f64 → 1 RG32F pixel (needs HDR output)
        
        let mut encoder = device.create_command_encoder();
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.compute_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(
                (input.len() as u32 + 255) / 256,
                1,
                1,
            );
        }
        device.queue().submit(Some(encoder.finish()));
    }
}

/// WGSL shader for encoding f64 as pixels
const ENCODE_F64_WGSL: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<vec2<u32>>;  // f64 as u32 pair
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(256)
fn encode(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= arrayLength(&input)) { return; }
    
    let f64_bits = input[idx];  // High and low 32 bits
    
    // Pack into 2 RGBA8 pixels (8 bytes total)
    let x = idx * 2u;
    let y = 0u;
    
    // Pixel 0: low 32 bits as RGBA
    let lo = f64_bits.x;
    textureStore(output, vec2<i32>(i32(x), i32(y)), vec4<f32>(
        f32((lo >> 0u) & 0xFFu) / 255.0,
        f32((lo >> 8u) & 0xFFu) / 255.0,
        f32((lo >> 16u) & 0xFFu) / 255.0,
        f32((lo >> 24u) & 0xFFu) / 255.0,
    ));
    
    // Pixel 1: high 32 bits as RGBA
    let hi = f64_bits.y;
    textureStore(output, vec2<i32>(i32(x + 1u), i32(y)), vec4<f32>(
        f32((hi >> 0u) & 0xFFu) / 255.0,
        f32((hi >> 8u) & 0xFFu) / 255.0,
        f32((hi >> 16u) & 0xFFu) / 255.0,
        f32((hi >> 24u) & 0xFFu) / 255.0,
    ));
}
"#;
```

---

## Use Cases

### 1. Parallel Data Paths

```
GPU doing eigensolve (uses PCIe for setup)
    │
    ├── PCIe: Control messages, small results
    │
    └── HDMI: Continuous stream of intermediate states
              (doesn't compete with PCIe bandwidth)
```

### 2. Multi-GPU Without NVLink

```
GPU A (RTX 4070)                  GPU B (RTX 3090)
    │                                   │
    └── HDMI OUT ──► Capture ──► GPUDirect
    
    No NVLink needed! Display path as interconnect.
    6 GB/s continuous stream between GPUs.
```

### 3. Hardware Isolation

```
Production Compute Path          Monitoring/Debug Path
────────────────────────         ─────────────────────
GPU ◄──PCIe──► CPU               GPU ──HDMI──► Capture ──► Monitoring
(mission critical)               (isolated, can't affect compute)
```

### 4. Latency-Aware Streaming

```
Low-Latency Path (PCIe DMA):
  - Eigenvalue convergence checks
  - Parameter updates
  - Small control data

High-Throughput Path (Display Stream):
  - Particle positions (continuous)
  - Field data (megabytes per frame)
  - Visualization pipeline
```

---

## Hardware Requirements

### Minimum Setup

```
┌─────────────────────────────────────────────────┐
│  System                                         │
│  ├── GPU with HDMI/DP output                    │
│  ├── Capture card (Magewell recommended)        │
│  │   └── GPUDirect support                      │
│  └── ToadStool managing routes                  │
└─────────────────────────────────────────────────┘
```

### Recommended Setup

```
┌─────────────────────────────────────────────────┐
│  System                                         │
│  ├── GPU A: Compute (RTX 3090/4090)             │
│  │   └── HDMI 2.1 / DP 2.0 output               │
│  ├── GPU B: Visualization (any)                 │
│  │   └── GPUDirect capture input                │
│  ├── Capture Card: Magewell Pro / Datapath     │
│  │   └── PCIe x4/x8, GPUDirect enabled          │
│  └── ToadStool: Route management                │
└─────────────────────────────────────────────────┘
```

---

## Integration with Existing Systems

### Relationship to Songbird

```
Songbird: "Where should this data go?" (machine/endpoint)
    │
    ▼
ToadStool Router: "How should it get there?" (physical path)
    │
    ├── Same machine → PCIe / Display / NVLink
    └── Different machine → Network (back to Songbird)
```

### Relationship to BarraCUDA

```
BarraCUDA: Compute (math happens here)
    │
    ▼
ToadStool Router: Data movement (move results)
    │
    ├── encode_for_display() ─► Display path
    ├── dma_transfer() ─► PCIe path
    └── network_send() ─► Songbird
```

---

## Implementation Phases

### Phase 0: Research (Current)
- [x] Document concept
- [x] Identify hardware (Magewell GPUDirect)
- [ ] Benchmark display-path bandwidth

### Phase 1: Interconnect Abstraction
- [ ] Define `Interconnect` enum
- [ ] Route enumeration for current system
- [ ] Basic route selection

### Phase 2: Display-Path Encoding
- [ ] F64 → RGBA encoder shader
- [ ] Texture → Display output
- [ ] Capture card integration (Magewell SDK)

### Phase 3: GPUDirect Integration
- [ ] Magewell GPUDirect API
- [ ] Direct GPU-to-GPU via display path
- [ ] Benchmark vs PCIe

### Phase 4: Unified Router
- [ ] Automatic route selection
- [ ] Latency-aware dispatch
- [ ] Integration with Songbird for cross-machine

---

## Key Insight

**The display output is underutilized infrastructure.** Every GPU has HDMI/DP outputs capable of 6-10 GB/s continuous streaming, but we only use them for monitors. With capture cards supporting GPUDirect, we can:

1. **Create parallel data paths** that don't compete with PCIe
2. **Stream data continuously** (display is designed for this)
3. **Enable GPU-to-GPU communication** without NVLink
4. **Isolate monitoring/debug** from production compute paths

ToadStool becomes the **hardware routing layer** — deciding not just what to compute, but *how to move the data*.

---

## References

- Magewell GPUDirect: https://www.magewell.com/kb/000020025
- NVIDIA GPUDirect: https://developer.nvidia.com/gpudirect
- AMD DirectGMA: https://gpuopen.com/directgma/
- Datapath capture cards: https://www.datapath.co.uk/

---

*From the ToadStool evolution desk — hardware routing exploration*
