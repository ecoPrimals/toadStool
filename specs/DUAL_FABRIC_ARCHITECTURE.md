# Dual-Fabric Architecture: Hardware Backbone + Network Plane

**Version**: 1.0  
**Date**: March 3, 2026  
**Status**: Specified (S94b)  
**Depends on**: `HARDWARE_TRANSPORT_SPEC.md`, `ARCHITECTURE_DEMARCATION.md`, `PRIMAL_CAPABILITY_SYSTEM.md`  
**License**: AGPL-3.0-only

---

## Motivation

When barraCuda budded from toadStool into its own primal, toadStool's specialization became clear: **physical hardware orchestration**. Not just "which GPU runs this shader" but "how does data physically move between machines, GPUs, capture cards, serial links, and edge devices."

Songbird already owns the network wire — TCP, UDP, mDNS, NAT traversal. But network is one transport medium. Physical cables (HDMI, DisplayPort, serial, PCIe) are another. A cluster that uses both simultaneously gets properties neither can provide alone.

This spec defines the **dual-fabric architecture**: Songbird manages the network plane, toadStool manages the hardware plane, and the two operate in parallel across a multi-machine deployment.

---

## Architecture Overview

```
Machine A                        Machine B                        Machine C
┌─────────────────┐              ┌─────────────────┐              ┌─────────────────┐
│ GPU 0 ──HDMI────┼──────────▶   │ Capture 0       │              │                 │
│ GPU 1 ──HDMI────┼──────────▶   │ Capture 1       │              │                 │
│ GPU 2 ──HDMI────┼──────────▶   │ Capture 2 ──────┼──▶ process ──┼──HDMI──▶ Cap 0  │
│                 │              │                 │              │                 │
│ Serial ──USB────┼──────────▶   │ Serial          │              │                 │
│                 │              │                 │              │                 │
│ NIC ◀═══════LAN═══════════▶   │ NIC ◀══════LAN══════════════▶  │ NIC             │
│ (Songbird)      │              │ (Songbird)      │              │ (Songbird)      │
└─────────────────┘              └─────────────────┘              └─────────────────┘

─────── = hardware backbone (toadStool TransportRouter)
═══════ = network plane (Songbird)
```

Every machine runs both a Songbird primal (network) and a toadStool primal (hardware). Each discovers its own capabilities at runtime:

- **Songbird** discovers NICs, peers, services via mDNS.
- **toadStool** discovers GPUs (DRM), capture cards (V4L2), serial ports, PCIe topology.

Neither primal hardcodes the other's topology. They share knowledge via capability-based IPC.

---

## Two Fabrics, Different Properties

| Property | Network Plane (Songbird) | Hardware Plane (toadStool) |
|----------|--------------------------|----------------------------|
| Protocol | TCP/UDP/QUIC | Frame protocol over physical link |
| Direction | Bidirectional | Per-link (HDMI: unidirectional; serial: bidi) |
| Bandwidth | 1–100 Gbps (NIC) | ~6 GB/s per HDMI 2.1 link, additive with multiple GPUs |
| Latency | Variable (TCP stack, routing, congestion) | Fixed (~16ms at 60Hz, ~8ms at 120Hz) |
| Discovery | mDNS, DNS-SD, IP scan | Physical — cable present or not |
| Security model | Software (TLS, firewalls, ACLs) | **Physics** — HDMI signal flows one direction only |
| Failure mode | Software crash, congestion, misconfiguration | Cable unplugged |
| Scalability | Switches, routers, VLANs | One link per GPU output |
| Visibility | Visible to OS networking stack | Invisible — no IP, no packets, no firewall rules |

The two fabrics are complementary, not competing. Each excels where the other is weak.

---

## Routing Topologies

The `TransportRouter` in `toadstool-core` enables any topology by composing Tx and Rx transports. The following topologies are supported by combining multiple toadStool instances across machines, coordinated by Songbird.

### Fan-Out (One-to-Many)

One machine streams to many. Each GPU output carries a different data shard.

```
                    ┌──▶ Machine B (Capture 0) ── shard 0
Machine A ─ GPU 0 ──┤
                    └──▶ Machine C (Capture 0) ── shard 0 (replica)

Machine A ─ GPU 1 ────▶ Machine D (Capture 0) ── shard 1
Machine A ─ GPU 2 ────▶ Machine E (Capture 0) ── shard 2
```

Use case: distributing a large dataset or simulation state to worker nodes for parallel processing.

### Pipeline (Chain)

Each machine processes its partition and streams results to the next. Each hop adds one frame of latency (~16ms at 60Hz).

```
Machine A ──HDMI──▶ Machine B ──HDMI──▶ Machine C ──HDMI──▶ Machine D
 (stage 1)           (stage 2)           (stage 3)           (output)
```

Use case: multi-stage simulation (hotSpring lattice partitions), video processing pipelines, data transformation chains. Each stage has its own GPU for compute and uses HDMI out to forward results.

### Ring

Circular pipeline where the last node feeds back to the first. Requires Songbird for the return path (since HDMI is unidirectional).

```
Machine A ──HDMI──▶ Machine B ──HDMI──▶ Machine C
    ▲                                        │
    └────────── Songbird (network) ──────────┘
```

Use case: iterative solvers where the boundary conditions wrap around. The HDMI links carry the bulk data, the Songbird return path carries convergence signals and thin boundary data.

### Hybrid Data/Control Split

Bulk data on hardware backbone. Control and metadata on network plane.

```
       HDMI (data plane, ~6 GB/s)
Machine A ════════════════════▶ Machine B
       ◀─── Songbird (control) ──▶
            "start", "checkpoint",
            backpressure signals,
            health monitoring
```

Use case: any deployment where the data volume overwhelms the network but the control messages are small. The hardware plane carries tensor data, simulation frames, video streams. The network plane carries JSON-RPC commands, health checks, flow control.

### Redundant

Same data on both fabrics. If one fails, the other continues.

```
Machine A ══HDMI══▶ Machine B    (primary: hardware, ~6 GB/s)
Machine A ──LAN──▶  Machine B    (fallback: network)
```

Use case: high-availability deployments where data delivery must not stop. toadStool monitors the hardware link (`is_available()`), Songbird monitors the network link, and the application fails over automatically.

---

## Airgapped Deployments

The most security-critical topology. HDMI's physical unidirectionality becomes a **hardware data diode** — a property that no software vulnerability can defeat.

### Unidirectional Tether

```
Classified Network                 Air Gap              Unclassified Network
┌───────────────────┐                │                  ┌───────────────────┐
│                   │  HDMI (Tx)     │                  │                   │
│  GPU out ─────────┼────────────────┼──▶ Capture Card  │                   │
│                   │                │                  │                   │
│  NO capture card  │         ◀── NOTHING ──            │  NO GPU output    │
│  on this machine  │                │                  │  to that side     │
└───────────────────┘                │                  └───────────────────┘
                               Physics-enforced
                               one-way barrier
```

Data can only flow **out** of the classified network. The classified machine has GPU outputs but no capture cards. The unclassified machine has capture cards but no GPU outputs pointed at the classified side. No software exploit, kernel vulnerability, or firmware compromise can reverse the TMDS signaling in the HDMI cable. The direction is enforced by the physics of the connector.

This replaces purpose-built hardware data diodes (which cost $10,000+) with commodity GPUs and $20 capture cards.

### Controlled Bidirectional Airgap

Two separate HDMI cables, two independent physical channels. Each direction is a distinct toadStool transport instance with its own audit trail.

```
Classified  ──── HDMI Cable A ────▶  Unclassified    (export: bulk)
Classified  ◀─── HDMI Cable B ────  Unclassified    (import: policy-controlled)
```

The import channel can be bandwidth-limited (lower resolution mode), content-filtered (toadStool validates frame protocol headers before delivering payload), and independently monitored. The two cables are physically separate — compromising one does not affect the other.

### Zero-Network Deployment

A cluster with NO network connectivity at all. All communication via hardware backbone.

```
Machine A ──HDMI──▶ Machine B ──HDMI──▶ Machine C
Machine A ◀──HDMI── Machine B ◀──HDMI── Machine C
(using separate GPU/capture pairs for each direction)
```

No IP addresses, no DNS, no network stack, no firewall rules, no network-based attack surface. Every machine needs: N GPU outputs + N capture cards for N bidirectional links. Discovery is physical: toadStool enumerates its own DRM connectors and V4L2 devices at boot.

---

## Flow Control

HDMI is unidirectional and runs at a fixed frame rate. The sender pushes frames at 60Hz regardless of whether the receiver is ready. Flow control requires a back-channel.

### With Songbird (recommended)

The receiver sends backpressure signals over the network plane:

```json
{"jsonrpc": "2.0", "method": "transport.backpressure", "params": {"link_id": "hdmi-0", "state": "slow_down"}}
```

The sender's toadStool instance reduces its encoding rate (e.g., skip frames, reduce payload size) or pauses the `route_loop`.

### Without Songbird (airgapped)

The receiver uses a separate HDMI output (if available) to signal back, encoding a single status byte per frame. Or the sender runs open-loop, accepting that some frames may be processed late. The frame protocol's sequence numbers allow the receiver to detect gaps.

### Serial back-channel

A USB serial link (~115200 baud, ~11 KB/s) carries flow control while HDMI carries bulk data. The `TransportRouter` bridges both:

```
GPU (HDMI Tx, ~6 GB/s) ────────▶ Capture Card (Rx)
Serial (Rx, ~11 KB/s) ◀──────── Serial (Tx, flow control)
```

---

## Multi-GPU Aggregate Bandwidth

Each GPU output is an independent transport link. Aggregate bandwidth scales linearly with GPU count:

| GPUs | HDMI Version | Per-Link | Aggregate |
|------|-------------|----------|-----------|
| 1 | HDMI 2.0 | ~1.5 GB/s | 1.5 GB/s |
| 4 | HDMI 2.0 | ~1.5 GB/s | 6.0 GB/s |
| 1 | HDMI 2.1 | ~6.0 GB/s | 6.0 GB/s |
| 4 | HDMI 2.1 | ~6.0 GB/s | 24.0 GB/s |
| 8 | HDMI 2.1 | ~6.0 GB/s | 48.0 GB/s |

At 8x HDMI 2.1, the hardware backbone exceeds 100GbE networking. The `TransportRouter` can stripe data across multiple links for aggregate throughput, similar to RAID 0 but for transport.

---

## Primal Responsibilities

```
┌─────────────────────────────────────────────────────────────────┐
│                     Multi-Machine Cluster                       │
│                                                                 │
│  Songbird: DISCOVERS network peers, ROUTES network traffic      │
│            Owns: TCP, UDP, mDNS, NAT traversal, TLS             │
│            Provides: transport.network.* JSON-RPC methods        │
│                                                                 │
│  toadStool: DISCOVERS hardware links, ROUTES physical data      │
│             Owns: DRM, V4L2, serial, PCIe, frame protocol       │
│             Provides: transport.hardware.* JSON-RPC methods      │
│                                                                 │
│  barraCuda: CONSUMES compute, REQUESTS dispatch                 │
│             Owns: math, shaders, GPU compute                    │
│             Uses: toadStool for "where" to compute              │
│             Uses: toadStool for hardware backbone if available   │
│                                                                 │
│  bearDog:   SECURES transport (TLS certs for Songbird,          │
│             frame signing for toadStool hardware links)          │
│                                                                 │
│  nestGate:  STORES artifacts, CACHES intermediate results       │
│             Uses: either fabric for bulk data movement           │
└─────────────────────────────────────────────────────────────────┘
```

No primal hardcodes another's topology. Songbird doesn't know about HDMI cables. toadStool doesn't know about IP addresses. The application (or a Spring) queries both and decides which fabric to use for each data flow.

---

## Relationship to Existing Specs

| Spec | Relationship |
|------|-------------|
| `ARCHITECTURE_DEMARCATION.md` | Extends the "hardware streaming" section. HDMI/capture is a new substrate alongside GPU/NPU/CPU. |
| `HARDWARE_TRANSPORT_SPEC.md` | Implementation detail. Defines the `HardwareTransport` trait, frame protocol, and concrete transport types used by this architecture. |
| `PRIMAL_CAPABILITY_SYSTEM.md` | toadStool registers `transport_hdmi`, `transport_capture`, `transport_serial` as capabilities alongside existing `compute_gpu`, `compute_npu`. |
| `SOVEREIGN_COMPUTE_EVOLUTION.md` | The airgapped deployment model is the ultimate expression of sovereign compute — user controls every physical data path. |
| `BARRACUDA_PRIMAL_BUDDING.md` | The budding freed toadStool to specialize in hardware. This spec is a direct consequence. |

---

## Implementation Status

| Component | Status | Crate |
|-----------|--------|-------|
| `HardwareTransport` trait | Implemented | `toadstool-core` |
| Frame protocol (encode/decode) | Implemented | `toadstool-core` |
| `TransportRouter` | Implemented | `toadstool-core` |
| `DisplayTransport` (HDMI/DP Tx) | Implemented | `toadstool-display` |
| `CaptureTransport` (V4L2 Rx) | Implemented | `toadstool-display` |
| `SerialTransport` (USB/UART) | Implemented (feature-gated) | `toadstool-display` |
| DRM connector enumeration | Implemented | `toadstool-display` |
| DRM modesetting + CRTC | Implemented | `toadstool-display` |
| DRM page flip + VSync | Implemented | `toadstool-display` |
| Multi-link striping | Future | — |
| Songbird topology sharing | Future | — |
| `PcieTransport` | Future | — |
| `NvLinkTransport` | Future | — |
| JSON-RPC `transport.*` methods | Future | — |
| Frame signing (bearDog) | Future | — |

---

## Summary

Budding barraCuda into its own primal allowed toadStool to focus on what it does best: hardware. The result is a compute platform where physical cables are first-class data transports, not just display outputs. A cluster can simultaneously use network (Songbird) and hardware (toadStool) fabrics, choosing the right medium for each data flow based on bandwidth, latency, directionality, and security requirements.

The airgapped use case — a hardware data diode built from commodity GPUs and capture cards — is only possible because toadStool owns the physical layer end-to-end, with no dependency on math (barraCuda) or network (Songbird) to function.
