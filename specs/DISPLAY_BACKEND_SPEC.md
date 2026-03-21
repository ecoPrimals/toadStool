# Toadstool Display Backend Specification

**Version**: 1.0  
**Date**: January 19, 2026  
**Status**: Phase 1 Complete (DRM/KMS backend, JSON-RPC IPC, V4L2 capture, input manager)  
**Compliance**: Deep Debt Principles (S++ Grade)

---

## 🎯 Mission

Provide a **100% Pure Rust display and input backend** for GUI primals (petalTongue, etc.), enabling TRUE PRIMAL architecture where the compute primal provisions ALL hardware.

---

## 🏗️ Architecture Principles

### Deep Debt Compliance

1. ✅ **100% Pure Rust** - Zero C dependencies
2. ✅ **Capability-Based Discovery** - No hardcoding
3. ✅ **Self-Knowledge Only** - Toadstool discovers own hardware
4. ✅ **Runtime Discovery** - Clients find Toadstool via capabilities
5. ✅ **Modern Async Rust** - Full async/await, streams
6. ✅ **Complete Implementation** - No mocks in production
7. ✅ **Safe Abstractions** - Unsafe isolated and documented

### Primal Responsibilities

**Toadstool (Compute/Hardware Primal)**:
- Hardware provisioning (display, input, GPU)
- Resource management (memory, buffers)
- Capability advertisement
- Direct hardware access

**petalTongue (UI Primal)**:
- User interface rendering
- Application logic
- User experience
- NO hardware knowledge

---

## 📦 Crate Structure

```
crates/
└── runtime/
    └── display/
        ├── Cargo.toml
        ├── src/
        │   ├── lib.rs              # Public API
        │   ├── drm/
        │   │   ├── mod.rs          # DRM/KMS abstraction
        │   │   ├── device.rs       # DRM device management
        │   │   ├── buffer.rs       # Dumb buffer allocation
        │   │   └── modesetting.rs  # Display configuration
        │   ├── input/
        │   │   ├── mod.rs          # Input abstraction
        │   │   ├── evdev.rs        # evdev device handling
        │   │   ├── events.rs       # Event types
        │   │   └── router.rs       # Event routing to windows
        │   ├── window/
        │   │   ├── mod.rs          # Window abstraction
        │   │   ├── manager.rs      # Multi-window management
        │   │   ├── framebuffer.rs  # Framebuffer operations
        │   │   └── focus.rs        # Focus management
        │   ├── ipc/
        │   │   ├── mod.rs          # JSON-RPC protocol
        │   │   ├── server.rs       # Display server
        │   │   └── types.rs        # Protocol types
        │   └── capabilities.rs     # Capability discovery
        ├── examples/
        │   ├── simple_window.rs    # Basic window example
        │   ├── multi_window.rs     # Multiple windows
        │   └── event_handling.rs   # Input events
        └── tests/
            ├── integration/
            │   ├── window_tests.rs
            │   ├── input_tests.rs
            │   └── ipc_tests.rs
            └── unit/
```

---

## 🔧 Core Components

### 1. DRM/KMS Layer

**Purpose**: Direct display hardware control

**Dependencies**:
```toml
linux-drm = "0.5"  # Primary choice
rustix = "0.38"    # Alternative/fallback
```

**API**:
```rust
pub struct DrmBackend {
    device: DrmDevice,
    connectors: Vec<Connector>,
    crtcs: Vec<Crtc>,
}

impl DrmBackend {
    pub fn open(path: &Path) -> Result<Self>;
    pub fn get_resources(&self) -> Result<Resources>;
    pub fn create_dumb_buffer(&self, width: u32, height: u32, bpp: u32) 
        -> Result<DumbBuffer>;
    pub fn add_framebuffer(&self, buffer: &DumbBuffer) 
        -> Result<Framebuffer>;
    pub fn set_mode(&self, crtc: Crtc, fb: Framebuffer, mode: Mode) 
        -> Result<()>;
}

pub struct DumbBuffer {
    handle: u32,
    width: u32,
    height: u32,
    stride: u32,
    size: u64,
}

impl DumbBuffer {
    pub fn map(&self) -> Result<MappedBuffer>;
}

pub struct MappedBuffer<'a> {
    data: &'a mut [u8],
    // Auto-unmap on drop
}
```

**Safety**:
- All unsafe isolated to device operations
- Proper lifetime management for mapped buffers
- Error handling for all ioctls
- Resource cleanup on drop

---

### 2. Input Layer

**Purpose**: Universal input device handling

**Dependencies**:
```toml
evdev = "0.13"  # Pure Rust evdev
```

**API**:
```rust
pub struct InputManager {
    devices: HashMap<DeviceId, InputDevice>,
    event_tx: mpsc::Sender<InputEvent>,
}

impl InputManager {
    pub async fn discover_devices() -> Result<Self>;
    pub fn subscribe_events(&self) -> mpsc::Receiver<InputEvent>;
    pub async fn poll_events(&self) -> Result<Vec<InputEvent>>;
    pub fn route_to_window(&mut self, window: WindowId);
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    KeyPress { key: KeyCode, modifiers: Modifiers, window: WindowId },
    KeyRelease { key: KeyCode, modifiers: Modifiers, window: WindowId },
    MouseMove { x: i32, y: i32, window: WindowId },
    MouseButton { button: MouseButton, pressed: bool, x: i32, y: i32, window: WindowId },
    MouseWheel { delta_x: f32, delta_y: f32, window: WindowId },
    Touch { id: u32, phase: TouchPhase, x: i32, y: i32, window: WindowId },
    WindowFocused { window: WindowId },
    WindowUnfocused { window: WindowId },
    WindowResized { window: WindowId, width: u32, height: u32 },
    WindowClosed { window: WindowId },
}

pub struct KeyCode {
    // Map to standard virtual keycodes
}

pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub logo: bool,
}
```

**Safety**:
- File descriptor management
- Async event processing
- Hotplug handling (udev)
- Permission checking

---

### 3. Window Manager

**Purpose**: Multi-window abstraction

**API**:
```rust
pub struct WindowManager {
    windows: HashMap<WindowId, Window>,
    drm: Arc<DrmBackend>,
    input: Arc<InputManager>,
    focused: Option<WindowId>,
}

impl WindowManager {
    pub async fn create_window(&mut self, req: CreateWindowRequest) 
        -> Result<WindowId>;
    pub async fn resize_window(&mut self, id: WindowId, size: Size) 
        -> Result<()>;
    pub async fn destroy_window(&mut self, id: WindowId) 
        -> Result<()>;
    pub fn get_window_info(&self, id: WindowId) 
        -> Result<WindowInfo>;
    pub fn set_focus(&mut self, id: WindowId);
    pub fn get_focused(&self) -> Option<WindowId>;
}

pub struct Window {
    id: WindowId,
    framebuffer: DumbBuffer,
    width: u32,
    height: u32,
    scale_factor: f32,
    focused: bool,
}

#[derive(Debug, Clone)]
pub struct CreateWindowRequest {
    pub width: u32,
    pub height: u32,
    pub title: Option<String>,
    pub fullscreen: bool,
}

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
    pub focused: bool,
}
```

**Features**:
- Multi-window support
- Focus management
- Automatic cleanup
- Event routing

---

### 4. Framebuffer Operations

**Purpose**: Pixel data management

**API**:
```rust
pub trait FramebufferOps {
    async fn get_framebuffer(&self, window: WindowId) 
        -> Result<FramebufferHandle>;
    async fn present(&self, window: WindowId, pixels: &[u8]) 
        -> Result<()>;
}

pub struct FramebufferHandle {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
    // Shared memory handle for zero-copy
    pub memfd: Option<OwnedFd>,
}

#[derive(Debug, Clone, Copy)]
pub enum PixelFormat {
    RGBA8888,
    BGRA8888,
    RGB888,
    RGB565,
}

impl FramebufferHandle {
    pub fn map(&self) -> Result<&mut [u8]>;
    pub async fn present(&self) -> Result<()>;
}
```

**Optimization**:
- Zero-copy via shared memory (memfd)
- Double-buffering
- VSync support
- Async present

---

### 5. IPC Protocol (JSON-RPC 2.0)

**Purpose**: Client-server communication

**Transport**: Unix domain sockets

**API**:
```rust
pub struct DisplayServer {
    manager: Arc<Mutex<WindowManager>>,
    socket_path: PathBuf,
}

impl DisplayServer {
    pub async fn serve(socket_path: PathBuf) -> Result<Self>;
    pub async fn handle_request(&self, req: JsonRpcRequest) 
        -> Result<JsonRpcResponse>;
}

// JSON-RPC Methods:
// - display.createWindow
// - display.resizeWindow
// - display.destroyWindow
// - display.getWindowInfo
// - display.getFramebuffer
// - display.present
// - display.subscribeInput
// - display.pollEvents
// - display.getCapabilities
```

**Protocol**:
```json
// Request
{
  "jsonrpc": "2.0",
  "method": "display.createWindow",
  "params": {
    "width": 1920,
    "height": 1080,
    "title": "My Window"
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "windowId": "uuid-here"
  },
  "id": 1
}

// Event Notification
{
  "jsonrpc": "2.0",
  "method": "display.inputEvent",
  "params": {
    "event": {
      "type": "KeyPress",
      "key": "A",
      "modifiers": {"shift": true},
      "window": "uuid-here"
    }
  }
}
```

---

### 6. Capability Discovery

**Purpose**: Self-knowledge and advertisement

**API**:
```rust
pub struct DisplayCapabilities {
    pub primal_id: String,
    pub socket_path: PathBuf,
    pub max_windows: usize,
    pub supported_formats: Vec<PixelFormat>,
    pub has_gpu_acceleration: bool,
    pub vsync_available: bool,
    pub input_devices: Vec<InputDeviceInfo>,
    pub displays: Vec<DisplayInfo>,
}

pub struct InputDeviceInfo {
    pub name: String,
    pub device_type: InputDeviceType,
    pub capabilities: Vec<InputCapability>,
}

pub struct DisplayInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f32,
    pub connected: bool,
}

impl DisplayCapabilities {
    pub async fn discover_self() -> Result<Self>;
    pub async fn announce() -> Result<()>;
    pub async fn find_display_backends() -> Result<Vec<DisplayCapabilities>>;
}
```

**Advertisement**:
```json
// /tmp/ecoPrimals/discovery/toadstool-display.json
{
  "primal_id": "toadstool-primary",
  "primal_type": "compute",
  "capabilities": ["compute", "gpu", "display", "input"],
  "socket_path": "/run/user/1000/toadstool/display.sock",
  "resources": {
    "displays": [
      {
        "name": "eDP-1",
        "width": 1920,
        "height": 1080,
        "refresh_rate": 60.0
      }
    ],
    "input_devices": [
      {
        "name": "AT Translated Set 2 keyboard",
        "type": "keyboard"
      }
    ]
  },
  "metadata": {
    "version": "4.18.0",
    "pure_rust": true
  }
}
```

---

## 🔒 Safety & Error Handling

### Unsafe Code Guidelines

**Allowed Uses**:
1. DRM ioctl calls (kernel interface)
2. Memory mapping (mmap for framebuffers)
3. File descriptor operations (raw FD handling)

**Requirements**:
- Every `unsafe` block MUST have SAFETY comment
- Minimize unsafe surface area
- Wrap in safe abstractions
- Document invariants

**Example**:
```rust
pub fn map_buffer(&self) -> Result<&mut [u8]> {
    // SAFETY: DRM guarantees valid memory region for mapped dumb buffer.
    // - handle is validated by kernel
    // - size is returned by CREATE_DUMB ioctl
    // - lifetime tied to DumbBuffer (unmapped on drop)
    unsafe {
        let ptr = libc::mmap(/* ... */);
        if ptr == libc::MAP_FAILED {
            return Err(Error::MapFailed);
        }
        std::slice::from_raw_parts_mut(ptr as *mut u8, self.size)
    }
}
```

### Error Handling

**Error Types**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum DisplayError {
    #[error("DRM device not found: {0}")]
    DeviceNotFound(PathBuf),
    
    #[error("Failed to open DRM device: {0}")]
    OpenFailed(#[from] std::io::Error),
    
    #[error("DRM ioctl failed: {0}")]
    IoctlFailed(String),
    
    #[error("Buffer allocation failed")]
    AllocationFailed,
    
    #[error("Window not found: {0}")]
    WindowNotFound(WindowId),
    
    #[error("Input device error: {0}")]
    InputError(String),
    
    #[error("IPC error: {0}")]
    IpcError(String),
}
```

**Error Strategy**:
- Never panic in library code
- Comprehensive error types
- Context propagation (anyhow in examples)
- Graceful degradation when possible

---

## 📊 Performance Requirements

### Latency Targets

- **Input latency**: < 8ms (keyboard/mouse to event)
- **Frame present**: < 16ms (60 FPS target)
- **IPC roundtrip**: < 5ms (Unix socket overhead)

### Throughput Targets

- **Frame rate**: 60+ FPS sustained
- **Event rate**: 1000+ events/second
- **Window count**: 8+ simultaneous windows

### Memory Usage

- **Per window**: ~10-20 MB (framebuffer + state)
- **Base overhead**: < 50 MB
- **Leak-free**: Zero leaks under stress testing

---

## 🧪 Testing Strategy

### Unit Tests

- DRM device operations
- Buffer allocation/deallocation
- Input event parsing
- Window manager operations
- IPC protocol serialization

### Integration Tests

- Full window lifecycle
- Multi-window scenarios
- Input routing correctness
- Client-server interaction
- Capability discovery

### Performance Tests

- Frame rate benchmarks
- Input latency measurements
- Memory usage profiling
- Stress testing (many windows)

### Chaos Tests

- Device hotplug/unplug
- Permission changes
- Out-of-memory scenarios
- Concurrent client stress
- Network failures (Unix socket)

---

## 📚 Documentation Requirements

### API Documentation

- All public items documented
- Examples for common patterns
- Safety notes for unsafe code
- Error conditions explained

### Integration Guide

- Setup instructions
- Client library usage
- petalTongue integration
- Troubleshooting guide

### Architecture Documentation

- Design decisions
- Performance characteristics
- Security considerations
- Future evolution path

---

## 🚀 Rollout Strategy

### Phase 0: Foundation (Week 1-2)
- Proof of concept
- Validate pure Rust viability
- Basic DRM + evdev working

### Phase 1: Core API (Week 3-4)
- Window management complete
- Input handling complete
- IPC protocol defined

### Phase 2: Integration (Week 5-6)
- petalTongue integration
- Zero-copy optimizations
- Polish and stability

### Phase 3: Production (Week 7-8)
- Full testing
- Documentation
- Performance validation
- Production deployment

---

## ✅ Acceptance Criteria

### Functionality

- [x] 100% Pure Rust (validated)
- [x] Creates and destroys windows (DRM dumb buffer backend)
- [ ] Renders pixels correctly (Phase 2: modesetting + page flip)
- [x] Routes input events (InputManager with evdev)
- [x] Supports multiple windows (WindowManager)
- [x] Handles errors gracefully (DRM ioctl errors → JsonRpcError)
- [x] Works on standard Linux (DRM/KMS)

### Quality

- [ ] 80%+ test coverage (current: tracked in D-COV)
- [x] Zero unsafe in public API (unsafe isolated to v4l2/drm backends)
- [x] All unsafe documented
- [x] No compiler warnings (clippy pedantic clean)
- [x] No linter errors
- [ ] Memory leak free (needs valgrind/miri validation)

### Performance

- [ ] 60+ FPS sustained (Phase 2: page flip pipeline)
- [ ] < 8ms input latency (Phase 2: evdev polling optimization)
- [x] < 5ms IPC latency (Unix socket JSON-RPC)
- [x] < 100 MB memory overhead

### Documentation

- [x] All public APIs documented (rustdoc clean)
- [ ] Integration guide complete (Phase 2: petalTongue integration)
- [ ] Examples working (Phase 2)
- [ ] Troubleshooting guide (Phase 2)

---

**Status**: Phase 1 Complete  
**Next**: Phase 2 — modesetting, page flip, petalTongue integration
