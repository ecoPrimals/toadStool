# 🍄 Toadstool Display Backend - Implementation Roadmap

**Version**: 1.0  
**Date**: January 19, 2026  
**Timeline**: 6-8 weeks  
**Goal**: 100% Pure Rust display backend for petalTongue

---

## 📅 Overview

**Total Duration**: 8 weeks  
**Team Size**: 2-3 developers (Toadstool + petalTongue collaboration)  
**Complexity**: High (kernel interfaces, async, IPC)  
**Risk**: Medium (experimental dependencies, but validated)

---

## 🎯 Milestones

| Phase | Duration | Deliverable | Status |
|-------|----------|-------------|--------|
| Phase 0 | Week 1-2 | Proof of Concept | 🔄 In Progress |
| Phase 1 | Week 3-4 | Core API | ⏳ Pending |
| Phase 2 | Week 5-6 | Integration | ⏳ Pending |
| Phase 3 | Week 7-8 | Production | ⏳ Pending |

---

## 📦 Phase 0: Foundation (Week 1-2)

**Goal**: Validate pure Rust approach with working proof of concept

### Week 1: Research & Validation

#### Monday-Tuesday: Dependency Setup
- [ ] Add `linux-drm` to Cargo.toml
- [ ] Add `evdev` to Cargo.toml
- [ ] Add `rustix` as fallback
- [ ] Test basic compilation
- [ ] Document dependency rationale

#### Wednesday-Thursday: DRM Proof of Concept
- [ ] Open `/dev/dri/card0`
- [ ] Query DRM capabilities
- [ ] Create first dumb buffer
- [ ] Map buffer to memory
- [ ] Write test pattern (checkerboard)
- [ ] Display on screen!

**Code Goal**:
```rust
// examples/poc_drm.rs
fn main() -> Result<()> {
    let drm = linux_drm::open("/dev/dri/card0")?;
    let buffer = drm.create_dumb_buffer(1920, 1080, 32)?;
    let mapped = buffer.map()?;
    
    // Fill with test pattern
    for y in 0..1080 {
        for x in 0..1920 {
            let color = if (x / 32 + y / 32) % 2 == 0 {
                0xFF0000FF // Red
            } else {
                0xFF00FF00 // Green
            };
            mapped.write_pixel(x, y, color);
        }
    }
    
    drm.present(&buffer)?;
    println!("Check your screen!");
    std::thread::sleep(Duration::from_secs(5));
    Ok(())
}
```

#### Friday: Input Proof of Concept
- [ ] Enumerate `/dev/input/event*`
- [ ] Open keyboard device
- [ ] Read key events
- [ ] Print to console
- [ ] Test with mouse

**Code Goal**:
```rust
// examples/poc_input.rs
fn main() -> Result<()> {
    let devices = evdev::enumerate()?;
    println!("Found {} input devices:", devices.len());
    
    for device in devices {
        let mut dev = evdev::Device::open(&device)?;
        println!("  - {} ({})", dev.name()?, device.display());
        
        // Read events
        for event in dev.into_event_stream()? {
            match event.kind() {
                evdev::InputEventKind::Key(key) => {
                    println!("Key: {:?} = {:?}", key, event.value());
                }
                evdev::InputEventKind::RelAxis(axis) => {
                    println!("Mouse: {:?} = {}", axis, event.value());
                }
                _ => {}
            }
        }
    }
    Ok(())
}
```

### Week 2: Core Structure

#### Monday-Tuesday: Create Crate Structure
- [ ] Create `crates/runtime/display/`
- [ ] Set up module hierarchy (drm, input, window, ipc)
- [ ] Define public API types
- [ ] Write initial documentation
- [ ] Create example skeleton

#### Wednesday-Thursday: DRM Abstraction
- [ ] `DrmBackend` struct
- [ ] Device discovery
- [ ] Capability queries
- [ ] Buffer allocation
- [ ] Framebuffer management
- [ ] Error handling

**Target API**:
```rust
let drm = DrmBackend::open("/dev/dri/card0")?;
let caps = drm.get_capabilities()?;
let buffer = drm.create_dumb_buffer(1920, 1080, 32)?;
let fb = drm.add_framebuffer(&buffer)?;
drm.set_mode(crtc, &fb, mode)?;
```

#### Friday: Input Abstraction
- [ ] `InputManager` struct
- [ ] Device enumeration
- [ ] Event types (keyboard, mouse, touch)
- [ ] Async event stream
- [ ] Basic routing logic

**Target API**:
```rust
let input = InputManager::discover().await?;
let mut events = input.subscribe_events();
while let Some(event) = events.recv().await {
    match event {
        InputEvent::KeyPress { key, .. } => { /* ... */ }
        InputEvent::MouseMove { x, y, .. } => { /* ... */ }
        _ => {}
    }
}
```

**Week 2 Deliverable**: 
- ✅ Working DRM device wrapper
- ✅ Working input event stream
- ✅ Clean abstraction layer
- ✅ Examples demonstrating both

---

## 🏗️ Phase 1: Core API (Week 3-4)

**Goal**: Complete window manager and IPC protocol

### Week 3: Window Manager

#### Monday-Tuesday: Window Abstraction
- [ ] `Window` struct (wraps DRM buffer)
- [ ] `WindowId` type
- [ ] `WindowManager` for multi-window
- [ ] Create/destroy operations
- [ ] Resize operations
- [ ] Window info queries

**Target API**:
```rust
let mut manager = WindowManager::new(drm, input).await?;

let win1 = manager.create_window(CreateWindowRequest {
    width: 1920,
    height: 1080,
    title: Some("Main Window".into()),
    fullscreen: false,
}).await?;

let info = manager.get_window_info(win1)?;
assert_eq!(info.width, 1920);
assert_eq!(info.height, 1080);
```

#### Wednesday-Thursday: Framebuffer Operations
- [ ] Pixel format handling (RGBA, BGRA, etc.)
- [ ] Buffer mapping/unmapping
- [ ] Present operation (page flip)
- [ ] Double-buffering support
- [ ] VSync integration

**Target API**:
```rust
let fb = manager.get_framebuffer(win1).await?;
let mut pixels = fb.map()?;

// Render something
for y in 0..fb.height {
    for x in 0..fb.width {
        pixels.write_pixel(x, y, Color::rgba(255, 0, 0, 255));
    }
}

fb.present().await?;  // Page flip with VSync
```

#### Friday: Focus & Multi-Window
- [ ] Window focus management
- [ ] Z-order tracking
- [ ] Input routing to focused window
- [ ] Window switch handling
- [ ] Full-screen mode

### Week 4: IPC Protocol

#### Monday-Tuesday: JSON-RPC Protocol
- [ ] Define all RPC methods
- [ ] Request/response types
- [ ] Event notification format
- [ ] Protocol versioning
- [ ] Documentation

**Protocol Methods**:
```
display.createWindow
display.resizeWindow
display.destroyWindow
display.getWindowInfo
display.getFramebuffer
display.present
display.subscribeInput
display.pollEvents
display.getCapabilities
display.setFocus
```

#### Wednesday-Thursday: RPC Server
- [ ] `DisplayServer` struct
- [ ] Unix socket listener
- [ ] Request handler
- [ ] Response serialization
- [ ] Event subscription
- [ ] Error mapping

**Target**:
```rust
let server = DisplayServer::new(manager)
    .bind("/run/user/1000/toadstool/display.sock")
    .await?;

server.serve().await?;  // Handle requests forever
```

#### Friday: Client Library
- [ ] `DisplayClient` struct
- [ ] Async RPC client
- [ ] Type-safe method calls
- [ ] Event stream subscription
- [ ] Connection management
- [ ] Retry logic

**Target (for petalTongue)**:
```rust
let client = DisplayClient::connect(
    "/run/user/1000/toadstool/display.sock"
).await?;

let window = client.create_window(1920, 1080).await?;
let mut events = client.subscribe_input().await?;

while let Some(event) = events.recv().await {
    // Handle input
}
```

**Week 3-4 Deliverable**:
- ✅ Complete window manager
- ✅ Working IPC protocol
- ✅ Client library for consumers
- ✅ Unit tests (80%+ coverage)

---

## 🔌 Phase 2: Integration (Week 5-6)

**Goal**: petalTongue running on Toadstool backend

### Week 5: petalTongue Integration

#### Monday: Capability Discovery
- [ ] Implement `DisplayCapabilities`
- [ ] Self-knowledge discovery
- [ ] Capability announcement (JSON file)
- [ ] Client-side discovery
- [ ] No hardcoded paths!

**Discovery Flow**:
```
Toadstool → Discovers own displays → Announces capability
petalTongue → Searches for "display" capability → Finds Toadstool → Connects
```

#### Tuesday-Wednesday: Backend Adapter
- [ ] petalTongue backend trait
- [ ] Toadstool backend implementation
- [ ] Event translation (Toadstool → egui)
- [ ] Coordinate mapping
- [ ] Scale factor handling

**petalTongue Side**:
```rust
impl UIBackend for ToadstoolBackend {
    async fn create_window(&mut self, width: u32, height: u32) -> Result<()> {
        self.window = self.client.create_window(width, height).await?;
        Ok(())
    }
    
    async fn poll_events(&mut self) -> Vec<Event> {
        self.client.poll_events().await
            .unwrap_or_default()
            .into_iter()
            .map(|e| self.convert_event(e))
            .collect()
    }
    
    async fn render(&mut self, pixels: &[u8]) -> Result<()> {
        self.client.present(self.window, pixels).await
    }
}
```

#### Thursday-Friday: End-to-End Testing
- [ ] petalTongue demo app on Toadstool
- [ ] Keyboard input working
- [ ] Mouse input working
- [ ] Touch input working
- [ ] Multiple windows working
- [ ] Performance measurement

**Demo App**: Simple GUI app that:
- Creates a window
- Draws some UI elements
- Responds to keyboard/mouse
- Measures FPS

### Week 6: Performance & Stability

#### Monday-Tuesday: Zero-Copy Optimization
- [ ] Shared memory (memfd) for framebuffers
- [ ] FD passing over Unix socket
- [ ] Memory mapping in client
- [ ] Direct rendering path
- [ ] Benchmark improvements

**Before/After**:
```
Before (copy): ~5ms per frame (memcpy overhead)
After (zero-copy): ~0.5ms per frame (direct GPU access)
```

#### Wednesday: VSync & Timing
- [ ] VSync synchronization
- [ ] Frame pacing
- [ ] Adaptive sync support
- [ ] Performance counters
- [ ] FPS stabilization

#### Thursday: Error Handling
- [ ] Graceful degradation
- [ ] Connection recovery
- [ ] Device hotplug handling
- [ ] Out-of-memory handling
- [ ] Comprehensive error messages

#### Friday: Documentation & Examples
- [ ] Integration guide for petalTongue
- [ ] API documentation complete
- [ ] Example applications
- [ ] Troubleshooting guide
- [ ] Performance tuning tips

**Week 5-6 Deliverable**:
- ✅ petalTongue running on Toadstool!
- ✅ Zero-copy rendering working
- ✅ 60+ FPS achieved
- ✅ Complete documentation

---

## 🚀 Phase 3: Production (Week 7-8)

**Goal**: Production-ready, tested, documented

### Week 7: Advanced Features

#### Monday: Multi-Monitor Support
- [ ] Enumerate all displays
- [ ] Per-display configuration
- [ ] Window placement across displays
- [ ] Display hotplug handling
- [ ] EDID parsing

#### Tuesday: Advanced Input
- [ ] Input device priorities
- [ ] Input grab (for games)
- [ ] Keyboard shortcuts
- [ ] Touch gestures
- [ ] Force feedback (haptics)

#### Wednesday: Production Hardening
- [ ] Permission handling (DRM master, seat)
- [ ] Systemd integration
- [ ] logind session management
- [ ] Security audit
- [ ] Resource limits

#### Thursday-Friday: Performance Optimization
- [ ] Profile hot paths
- [ ] Optimize allocations
- [ ] Reduce latency
- [ ] Memory usage tuning
- [ ] CPU usage optimization

### Week 8: Testing & Release

#### Monday-Tuesday: Comprehensive Testing
- [ ] Unit tests (all modules)
- [ ] Integration tests (E2E scenarios)
- [ ] Performance benchmarks
- [ ] Stress testing (many windows, high load)
- [ ] Chaos testing (failures, hotplug, etc.)

**Test Coverage Goals**:
- Unit tests: 80%+
- Integration tests: Major workflows
- Performance: All benchmarks passing
- Chaos: No panics, graceful degradation

#### Wednesday: Documentation Finalization
- [ ] API docs complete
- [ ] Integration guide polished
- [ ] Architecture documentation
- [ ] Performance characteristics
- [ ] Security considerations
- [ ] Migration guide (from winit/eframe)

#### Thursday: petalTongue Full Migration
- [ ] Remove winit dependency
- [ ] Remove eframe display backend
- [ ] Default to Toadstool backend
- [ ] Fallback strategy (if Toadstool unavailable)
- [ ] Feature flags for flexibility

**petalTongue Cargo.toml**:
```toml
[features]
default = ["ui-toadstool"]
ui-toadstool = ["toadstool-display-client"]
ui-eframe = ["eframe", "winit"]  # Fallback
```

#### Friday: Release & Celebration!
- [ ] Final code review
- [ ] Version tagging (v4.18.0)
- [ ] Changelog update
- [ ] GitHub release
- [ ] Announcement blog post
- [ ] 🎉 Celebrate 100% Pure Rust GUI!

**Week 7-8 Deliverable**:
- ✅ Production-ready backend
- ✅ Complete test suite
- ✅ Full documentation
- ✅ petalTongue migrated
- ✅ **100% Pure Rust GUI ACHIEVED!**

---

## 📊 Success Metrics

### Functionality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Window operations | 100% working | Manual + automated tests |
| Input routing | 100% accurate | Event validation tests |
| Multi-window | 8+ simultaneous | Stress tests |
| Display modes | All supported | Hardware capability tests |

### Performance Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Frame rate | 60+ FPS | Continuous rendering test |
| Input latency | < 8ms | High-speed camera |
| IPC latency | < 5ms | Timestamp comparison |
| Memory per window | < 20 MB | Profiling |
| CPU usage (idle) | < 1% | System monitor |

### Quality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Test coverage | 80%+ | cargo-tarpaulin |
| Unsafe blocks | < 20 | Manual audit |
| Compiler warnings | 0 | cargo build |
| Linter errors | 0 | cargo clippy |
| Memory leaks | 0 | Valgrind / MIRI |

### Documentation Metrics

| Metric | Target | Status |
|--------|--------|--------|
| API docs | 100% public items | cargo doc |
| Examples | 5+ working | examples/ |
| Integration guide | Complete | docs/ |
| Architecture docs | Complete | specs/ |

---

## 🎯 Risk Management

### Technical Risks

**Risk**: `linux-drm` crate instability
- **Mitigation**: Have `rustix` fallback ready
- **Contingency**: Contribute fixes upstream
- **Impact**: Medium

**Risk**: Permission/access issues (DRM master, seat)
- **Mitigation**: Document setup requirements
- **Contingency**: logind/systemd integration
- **Impact**: Low

**Risk**: Performance not meeting targets
- **Mitigation**: Early benchmarking, profiling
- **Contingency**: GPU acceleration, optimization
- **Impact**: Medium

**Risk**: Multi-window complexity
- **Mitigation**: Start simple, iterate
- **Contingency**: Limit initial window count
- **Impact**: Low

### Dependency Risks

**Risk**: `linux-drm` breaking changes
- **Mitigation**: Pin version, test upgrades
- **Contingency**: Fork if necessary
- **Impact**: Low

**Risk**: `evdev` API changes
- **Mitigation**: Stable API since 0.13
- **Contingency**: Abstraction layer isolates us
- **Impact**: Very Low

### Integration Risks

**Risk**: petalTongue integration blockers
- **Mitigation**: Weekly sync calls
- **Contingency**: Flexible API design
- **Impact**: Low

**Risk**: Ecosystem adoption slow
- **Mitigation**: Excellent docs, examples
- **Contingency**: Keep eframe fallback
- **Impact**: Low

---

## 📞 Collaboration Touchpoints

### Weekly Sync (Fridays 2pm UTC)

**Week 1**: Phase 0 kickoff, PoC demo  
**Week 2**: DRM/input abstractions review  
**Week 3**: Window manager API review  
**Week 4**: IPC protocol finalization  
**Week 5**: petalTongue integration start  
**Week 6**: Performance results, optimization  
**Week 7**: Production hardening review  
**Week 8**: Final review, release prep  

### Communication Channels

- **Real-time**: Discord #toadstool-display
- **Issues**: GitHub issues (tag: display-backend)
- **Docs**: Shared Google Docs for specs
- **Code Review**: GitHub PRs (require approval)

---

## 🎉 Release Checklist

### Code

- [ ] All features implemented
- [ ] All tests passing
- [ ] Zero compiler warnings
- [ ] Zero clippy lints
- [ ] No memory leaks (MIRI clean)
- [ ] Performance targets met

### Documentation

- [ ] README updated
- [ ] API docs complete (100%)
- [ ] Integration guide complete
- [ ] Examples working (all)
- [ ] Changelog updated
- [ ] Migration guide (from winit)

### Testing

- [ ] Unit tests (80%+ coverage)
- [ ] Integration tests (all scenarios)
- [ ] Performance benchmarks (passing)
- [ ] Stress tests (no failures)
- [ ] Chaos tests (graceful degradation)
- [ ] Cross-platform tests (various distros)

### Release

- [ ] Version bump (v4.18.0)
- [ ] Git tag created
- [ ] GitHub release
- [ ] crates.io publish
- [ ] Announcement blog post
- [ ] Discord announcement
- [ ] Update root docs

---

## 🏆 Definition of Done

**Phase 0 DONE when**:
- ✅ DRM device opens and creates buffer
- ✅ Input devices enumerated and events read
- ✅ Examples demonstrate both
- ✅ Code committed to `feature/display-backend` branch

**Phase 1 DONE when**:
- ✅ Full window manager API implemented
- ✅ IPC protocol complete and documented
- ✅ Client library working
- ✅ Unit tests 80%+ coverage

**Phase 2 DONE when**:
- ✅ petalTongue demo running on Toadstool
- ✅ 60+ FPS achieved
- ✅ Zero-copy working
- ✅ Integration docs complete

**Phase 3 DONE when**:
- ✅ All production features complete
- ✅ All tests passing
- ✅ Documentation complete
- ✅ petalTongue fully migrated
- ✅ Released to production (v4.18.0)
- ✅ **100% Pure Rust GUI ACHIEVED!** 🎉

---

**Status**: Ready to Start Phase 0  
**Next Step**: Create crate structure and begin DRM PoC  
**Timeline Start**: January 19, 2026  
**Expected Completion**: March 13, 2026 (8 weeks)

🍄🌸 **Let's build the first Pure Rust display backend!** 🌸🍄
