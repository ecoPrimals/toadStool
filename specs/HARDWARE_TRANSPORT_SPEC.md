# Hardware Transport Specification

**Version**: 1.0  
**Status**: Implemented (S94b)  
**Crates**: `toadstool-core`, `toadstool-display`  
**License**: AGPL-3.0-or-later

## Overview

The Hardware Transport Layer enables **any hardware input to any hardware output**. ToadStool owns the physical pipe: PCIe, HDMI, NVLink, serial, capture. This spec defines the generic `HardwareTransport` trait and its concrete implementations.

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
