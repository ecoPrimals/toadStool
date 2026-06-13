# ToadStool Daemon Mode — Architecture

> **Fossilized (S170).** The January 2026 daemon evolution proposal has been archived to
> `ecoPrimals/infra/wateringHole/fossilRecord/TOADSTOOL_DAEMON_MODE_EVOLUTION_JAN2026.md`.
>
> That proposal was for the HTTP-based daemon. The daemon is now fully IPC-first
> (JSON-RPC 2.0 over Unix sockets) as of S169.

## Current Architecture (S170→S310)

```
toadstool server
    ├── Unix socket: $BIOMEOS_SOCKET_DIR/compute.sock (JSON-RPC) + compute-tarpc.sock (tarpc)
    ├── TCP (optional): --port <PORT> for JSON-RPC over TCP
    ├── JSON-RPC 2.0 methods (111 direct + semantic registry)
    └── Graceful shutdown via SIGINT/SIGTERM
```

### Key Design Decisions

- **No HTTP server** — HTTP traffic routed through Songbird (S169)
- **Unix socket primary** — TCP is opt-in fallback via `--port`
- **Port 0 default** — OS-assigned ephemeral port when TCP is enabled
- **Capability-based discovery** — env vars (`COORDINATION_PORT`, etc.), XDG manifests, socket directory scan
- **Zero production sleeps** — exponential backoff for polling, `tokio::time::Instant` for TTL

### ADRs

- [ADR-001: wgpu over OpenCL/CUDA](adrs/ADR-001-wgpu-over-opencl-cuda.md)
- [ADR-002: Feature-gate TPU support](adrs/ADR-002-feature-gate-tpu-support.md)
- [ADR-003: NTT for FHE polynomial multiplication](adrs/ADR-003-ntt-for-fhe-polynomial-multiplication.md)
- [ADR-004: Capability-based service discovery](adrs/ADR-004-capability-based-service-discovery.md)
