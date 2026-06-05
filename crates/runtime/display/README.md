# 🍄 Toadstool Display Backend

**100% Pure Rust display and input backend** for ecoPrimals ecosystem.

## Status

**Phase**: 0 - Foundation (In Progress)  
**Version**: 0.1.0  
**Pure Rust**: ✅ 100% (Zero C dependencies!)

## Mission

Enable TRUE PRIMAL architecture where the compute primal (Toadstool) provisions ALL hardware (display, input, GPU), allowing UI primals (petalTongue) to achieve 100% Pure Rust.

## Architecture

```
petalTongue (UI Primal)
   ↓ JSON-RPC over Unix sockets
Toadstool Display Backend
   ├── DRM/KMS (display hardware) - linux-drm ✅
   ├── evdev (input devices) - evdev ✅
   ├── Window Manager (multi-window) ✅
   └── Framebuffer Ops (rendering) ✅
   ↓ Direct hardware access
Hardware (GPU, display, keyboard, mouse)
```

## Features

- ✅ **100% Pure Rust** - Zero C dependencies
- ✅ **DRM/KMS** - Direct display hardware control
- ✅ **evdev** - Universal input handling
- ✅ **Multi-window** - Multiple simultaneous windows
- ✅ **Async** - Modern async/await throughout
- ✅ **IPC** - JSON-RPC over Unix sockets
- ✅ **Capability-based** - Runtime discovery

## Dependencies

All Pure Rust!

```toml
linux-drm = "0.5"   # DRM/KMS (NO C!)
evdev = "0.13"      # Input (NO libevdev!)
tokio = "*"         # Async runtime
serde_json = "*"    # JSON-RPC over Unix/TCP sockets
```

## Usage

### Server (Toadstool)

```rust
use toadstool_display::{DisplayServer, WindowManager};

#[tokio::main]
async fn main() -> toadstool_display::Result<()> {
    let manager = WindowManager::new().await?;
    let server = DisplayServer::new(manager)
        .bind("/run/user/1000/toadstool/display.sock")
        .await?;
    server.serve().await?;
    Ok(())
}
```

### Client (petalTongue)

```rust
use toadstool_display::{DisplayClient, WindowId};

#[tokio::main]
async fn main() -> toadstool_display::Result<()> {
    let client = DisplayClient::connect(
        "/run/user/1000/toadstool/display.sock"
    ).await?;
    
    let window = client.create_window(1920, 1080).await?;
    let mut events = client.subscribe_input().await?;
    
    while let Some(event) = events.recv().await {
        // Handle input
    }
    Ok(())
}
```

## Development

### Phase 0: Foundation (Current)

- [ ] DRM proof of concept
- [ ] Input proof of concept
- [ ] Basic abstractions

### Phase 1: Core API

- [ ] Window manager
- [ ] IPC protocol
- [ ] Client library

### Phase 2: Integration

- [ ] petalTongue integration
- [ ] Zero-copy optimization
- [ ] Performance tuning

### Phase 3: Production

- [ ] Advanced features
- [ ] Full testing
- [ ] Documentation

## Documentation

See:
- [Specification](../../../specs/DISPLAY_BACKEND_SPEC.md)

## Collaboration

Built in collaboration with petalTongue team!

**Timeline**: 6-8 weeks  
**Target**: 100% Pure Rust GUI  
**Status**: Phase 0 in progress  

🍄🌸 **Toadstool + petalTongue = Pure Rust Excellence!** 🌸🍄
