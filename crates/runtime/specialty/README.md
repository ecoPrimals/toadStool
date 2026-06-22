# ToadStool Specialty Runtime

**Status**: Compiles, 145 lib tests passing | Feature-gated (`embedded-placeholder-impls`) | P3 priority

---

## Overview

Specialty runtime support for exotic and legacy hardware platforms:

- **Mainframe Systems**: IBM System/360, System/370, z/Series, VAX/VMS, AS/400
- **Embedded Systems**: 8-bit/16-bit microcontrollers (6502, Z80, 8080, 8051, 8086, 68000)
- **Industrial Control**: PLCs, SCADA systems, industrial protocols
- **Real-time Systems**: VxWorks, QNX, RTOS platforms
- **Legacy Networking**: NetBIOS, IPX, DECnet protocols

## Current State (S325+)

The crate compiles cleanly in the workspace and has 145 passing lib tests. Core trait
hierarchies (`EmbeddedToolchain`, `ProgrammerInterface`, `EmbeddedEmulator`) are defined
with opt-in placeholder implementations behind the `embedded-placeholder-impls` feature.

### Remaining Debt

- **D-EMBEDDED-PROGRAMMER**: Transport not fully wired for programmer interface
- **D-EMBEDDED-EMULATOR**: Decimal mode, full Z80 instruction set, peripheral simulation incomplete
- `native-bindings` and `cross-compilation` features declared but not yet implemented

### Testing

```bash
cargo test -p toadstool-runtime-specialty --lib
```

## Architecture

Specialty hardware dispatch is capability-based — the runtime advertises what it supports
and is discovered at runtime by the orchestration layer. No hardcoded primal references.
