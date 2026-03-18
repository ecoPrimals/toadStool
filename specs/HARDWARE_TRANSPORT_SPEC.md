# Hardware Transport Specification

**Version**: 1.2  
**Status**: Implemented (S94b) + PCIe P2P implemented (S142) + VFIO interface (S150)  
**Crates**: `toadstool-core`, `toadstool-display`, `nvpmu`  
**License**: AGPL-3.0-or-later

## Overview

The Hardware Transport Layer enables **any hardware input to any hardware output**. ToadStool owns the physical pipe: PCIe, HDMI, NVLink, serial, capture. This spec defines the generic `HardwareTransport` trait and its concrete implementations.

### Relationship to VFIO

VFIO is **not** a `HardwareTransport`. The `HardwareTransport` trait covers
data-plane transports (frames in, frames out). VFIO is a control-plane
interface for device binding, BAR0/BAR1 access, and IOMMU management — handled
by `nvpmu::VfioBar0Access` and the `setup-gpu-sovereign.sh` tooling.

The distinction:
- **HardwareTransport**: data flow (HDMI, V4L2, PCIe P2P, serial)
- **VFIO interface**: device ownership and register access (BAR0 init, permissions, GPU binding)

## Architecture

```
Machine A (ToadStool)                    Machine B (ToadStool)
┌────────────────────┐                   ┌────────────────────┐
│ Data → Encoder →   │                   │   → Decoder → Data │
│ GPU Framebuffer →  │───HDMI Cable───▶  │ Capture Card →     │
│ HDMI/DP Out        │                   │ V4L2 Input         │
└────────────────────┘                   └────────────────────┘
```

All transports implement a single trait, and a `TransportRouter` connects any Rx to any Tx.

## Core Trait

Defined in `toadstool-core::hardware_transport`:

```rust
pub trait HardwareTransport: Send + Sync {
    fn info(&self) -> &TransportInfo;
    fn bandwidth_bps(&self) -> u64;
    fn is_available(&self) -> bool;
    fn send(&mut self, data: &[u8]) -> Result<usize, TransportError>;
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError>;
}
```

### TransportInfo

| Field       | Type                | Description                              |
|-------------|---------------------|------------------------------------------|
| `id`        | `String`            | Unique identifier (e.g. `/dev/dri/card0:HDMI-A-1`) |
| `label`     | `String`            | Human-readable name                      |
| `medium`    | `TransportMedium`   | Display, Capture, Serial, PCIe, NVLink   |
| `direction` | `TransportDirection`| Tx, Rx, or Bidirectional                 |

### TransportDirection

- `Tx` — send only (e.g. HDMI output)
- `Rx` — receive only (e.g. capture card)
- `Bidirectional` — both (e.g. serial, PCIe)

## Frame Protocol

Data is encoded into framebuffer pixels using a lightweight framing protocol:

```
┌──────────┬─────────┬──────────┬─────────────┬──────────┬─────────┐
│ Magic(4) │ Ver.(1) │ Seq.(4)  │ PayloadLen(4)│ CRC(4)   │ Payload │
│  "TSXP"  │   0x01  │ LE u32   │   LE u32    │ XOR-fold │  bytes  │
└──────────┴─────────┴──────────┴─────────────┴──────────┴─────────┘
```

- **Header**: 17 bytes total
- **Magic**: `TSXP` (ToadStool Transport)
- **Checksum**: XOR-fold of 4-byte words in payload (fast, sufficient for cable-error detection)
- **Encoding**: At 4K@60Hz RGBA8888: 3840 × 2160 × 4 × 60 ≈ 1.99 GB/s raw throughput

## Implementations

### DisplayTransport (Tx)

- **Crate**: `toadstool-display::transport`
- **Medium**: HDMI / DisplayPort output
- **Direction**: Tx only
- **Backend**: DRM/KMS via `drm` crate (Pure Rust)
- **Flow**: encode_frame → write to dumb buffer → page flip → physical connector

### CaptureTransport (Rx)

- **Crate**: `toadstool-display::capture_transport`
- **Medium**: V4L2 capture card
- **Direction**: Rx only
- **Backend**: V4L2 ioctls via `rustix` (Pure Rust, zero C deps for ioctl layer)
- **Flow**: V4L2 DQBUF → decode_frame → return payload

### SerialTransport (Bidirectional)

- **Crate**: `toadstool-display::serial_transport`
- **Medium**: USB serial / UART
- **Direction**: Bidirectional
- **Backend**: `serialport` crate (feature-gated: `serial-transport`)
- **Flow**: direct read/write to serial port fd

## TransportRouter

Defined in `toadstool-core::transport_router`:

- Registers any number of `HardwareTransport` implementations
- `find(filter)` — capability-based selection by direction, medium, bandwidth
- `route_once(rx_id, tx_id, buf_size)` — transfer one chunk from Rx to Tx
- `route_loop(rx_id, tx_id, buf_size, callback)` — continuous streaming

### Example Pipeline

```
capture card (Rx) → TransportRouter → serial port (Tx)
                                    → HDMI output (Tx)
```

## DRM Modesetting

Phase 1 foundation in `toadstool-display::drm`:

| Module          | Purpose                                         |
|-----------------|-------------------------------------------------|
| `connector.rs`  | Enumerate connectors, modes, EDID               |
| `modesetting.rs`| CRTC allocation, framebuffer attach, set_crtc    |
| `pageflip.rs`   | Double-buffered page flip with VSync events      |

## Planned: PcieTransport (GPU-to-GPU P2P)

**Status**: Specified (S142+)
**Medium**: PCIe peer-to-peer DMA
**Direction**: Bidirectional

GPU-to-GPU data transfer without CPU staging. PCIe P2P won't match NVLink
bandwidth, but it outperforms CPU roundtrip by 4-10× for large payloads.

### Discovery

```
/sys/bus/pci/devices/{addr}/
├── class         # 0x030000 = VGA, 0x030200 = 3D
├── vendor        # 0x10de = NVIDIA, 0x1002 = AMD
├── numa_node     # NUMA locality
└── iommu_group/  # IOMMU grouping (P2P requires same group or ACS)
```

- Discover PCIe topology via sysfs
- Detect shared PCIe switch (common parent bridge)
- NUMA-aware: prefer devices on same NUMA node
- IOMMU group check: P2P requires compatible IOMMU config

### Mechanism

- **AMD (RDNA2+)**: dma-buf export via DRM render node (`DRM_IOCTL_PRIME_HANDLE_TO_FD`),
  import on target device (`DRM_IOCTL_PRIME_FD_TO_HANDLE`). GEM buffer sharing.
- **NVIDIA (NVK/nouveau)**: dma-buf via DRM GEM export/import. Native NVLink where available.
- **Fallback**: CPU-staged copy via `PinnedMemory` (64-byte aligned DMA buffers)

### Bandwidth

| Interconnect | Theoretical | Practical | Latency |
|-------------|-------------|-----------|---------|
| PCIe 3.0 x16 | 16 GB/s | ~12 GB/s | ~1 µs |
| PCIe 4.0 x16 | 32 GB/s | ~25 GB/s | ~1 µs |
| PCIe 5.0 x16 | 64 GB/s | ~50 GB/s | ~1 µs |
| NVLink 3 (A100) | 600 GB/s | ~500 GB/s | ~0.7 µs |
| CPU staging | ~20 GB/s | ~8 GB/s | ~10 µs |

### Spring Use Case

hotSpring's brain architecture: RTX 3090 motor + Titan V pre-motor. Tensors
move GPU-to-GPU via PCIe P2P instead of GPU→CPU→GPU. For a 64MB tensor,
PCIe 3.0 P2P saves ~12ms per transfer vs CPU staging.

## Planned: Streaming Transport

**Status**: Specified (S142+)

`transport.stream` enables continuous streaming between any Rx and Tx:

```json
{ "method": "transport.stream",
  "params": { "rx_id": "/dev/video0", "tx_id": "pcie:0000:01:00.0→0000:02:00.0",
              "buf_size": 1048576 } }
→ { "stream_id": "...", "status": "streaming" }
```

- Background task with `CancellationToken`
- Throughput metrics via `transport.status`
- Auto-reconnect on transient errors
- Backpressure: if Tx is slower than Rx, buffer up to configurable limit

## Sovereignty Compliance

- All transports are **discovered at runtime** — no hardcoded device paths
- `TransportFilter` enables **capability-based selection** ("10+ Gbps Tx")
- **No barraCuda dependency** — pure hardware, no math/shaders needed
- **No vendor lock-in** — works with any DRM driver, any V4L2 capture card, any serial port

## JSON-RPC Methods

Implemented in `toadstool-server::pure_jsonrpc::handler`:

| Method               | Status        | Description                                |
|----------------------|---------------|--------------------------------------------|
| `transport.discover` | Implemented   | List all available transports (best-effort) |
| `transport.list`     | Implemented   | List transports registered in the router   |
| `transport.route`    | Implemented   | Route data once from Rx to Tx              |
| `transport.open`     | Future        | Open and register a specific transport     |
| `transport.stream`   | Future        | Continuous streaming between Rx and Tx     |
| `transport.status`   | Future        | Query active stream statistics             |

### `transport.discover`

Returns all hardware transports detectable on the host without opening devices.

```json
{ "method": "transport.discover" }
→ { "transports": [{ "id": "/dev/dri/card0:HDMI-A-1", "label": "HDMI-A-1", "medium": "Display", "direction": "Tx" }], "count": 1 }
```

### `transport.route`

Routes a single chunk from an Rx transport to a Tx transport. Both must be registered in the router.

```json
{ "method": "transport.route", "params": { "rx_id": "/dev/video0", "tx_id": "/dev/ttyUSB0", "buf_size": 65536 } }
→ { "bytes_transferred": 4096, "rx_id": "/dev/video0", "tx_id": "/dev/ttyUSB0" }
```

## CLI Commands

Implemented in `toadstool-cli::commands::transport`:

| Command                         | Description                                    |
|---------------------------------|------------------------------------------------|
| `toadstool transport discover`  | Discover hardware transports (table or JSON)   |
| `toadstool transport list`      | List discovered transports                     |
| `toadstool transport status`    | Summary counts by transport type               |

### Example Output

```
Hardware Transports
═══════════════════════════════════════════════════
ID                             Medium     Direction  Label
/dev/dri/card0:HDMI-A-1        Display    Tx         HDMI-A-1
/dev/video0                    Capture    Rx         V4L2:/dev/video0

Found: 2 transports (1 display, 1 capture, 0 serial)
```

## Dependencies

| Crate         | Purpose                  | Pure Rust |
|---------------|--------------------------|-----------|
| `drm`         | DRM/KMS ioctls           | Yes       |
| `rustix`      | System calls, mmap, V4L2 | Yes       |
| `serialport`  | Serial port I/O          | Mostly*   |
| `thiserror`   | Error types              | Yes       |

*`serialport` used with `default-features = false` to avoid libudev-sys; pure Rust enumeration (less detail).
