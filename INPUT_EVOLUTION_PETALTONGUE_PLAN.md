# 🌱 Input Evolution for petalTongue Symbiosis

**Date**: Friday, January 31, 2026 (Evening)  
**Vision**: toadstool grows in the metal, petalTongue grows on top - symbiotic!  
**Mission**: Evolve input system from Phase 1 → Phase 2 (Production Ready for petalTongue)

---

## 🎯 Architectural Vision

### The Symbiotic Relationship:

```
┌─────────────────────────────────────────┐
│         petalTongue (UI Layer)          │
│  ┌───────────────────────────────────┐  │
│  │   Touch    │  Mouse  │  Keyboard  │  │
│  │   Gestures │  Wheel  │  Gamepad   │  │
│  │   Haptics* │  Stylus │  ...       │  │
│  └───────────────────────────────────┘  │
│              ▲                           │
│              │ IPC (JSON-RPC)            │
│              │ ALL input events          │
└──────────────┼───────────────────────────┘
               │
┌──────────────▼───────────────────────────┐
│      toadstool (Hardware Layer)          │
│  ┌───────────────────────────────────┐  │
│  │   /dev/input/* discovery          │  │
│  │   evdev event streams (async)     │  │
│  │   Multi-touch parsing             │  │
│  │   Haptic output* (future)         │  │
│  │   Device hotplug                  │  │
│  └───────────────────────────────────┘  │
│         "Grows within the metal"         │
└──────────────────────────────────────────┘
```

**Key Principle**: toadstool provides **ALL** hardware abstraction, petalTongue consumes **ALL** events!

---

## 📊 Current Status Assessment

### ✅ Phase 1 Complete (Architecture):

1. **Event Types** - ✅ Complete
   - Keyboard (KeyPress/KeyRelease)
   - Mouse (Move/Button/Wheel)
   - Touch (Started/Moved/Ended/Cancelled)
   - Window events (Focus/Resize/Close)

2. **Device Discovery** - ✅ Working
   - Scans `/dev/input/event*`
   - Runtime self-knowledge
   - Permission handling

3. **Focus Management** - ✅ Complete
   - Focus routing to windows
   - Focus change events
   - Multi-window support

4. **IPC Protocol** - ✅ Defined
   - JSON-RPC 2.0
   - Event subscription
   - Poll API

### ⏳ Phase 2 Needed (Actual Streaming):

1. **Device Opening** - TODO
   - Open evdev::Device handles
   - Detect device types (keyboard/mouse/touch)
   - Query capabilities

2. **Event Parsing** - TODO
   - Parse evdev → InputEvent
   - Modifier key tracking
   - Multi-touch tracking

3. **Async Streams** - TODO
   - Spawn tokio tasks per device
   - Stream events to channel
   - Route to focused window

4. **Multi-Touch** - TODO
   - Track multiple touch points
   - Touch ID management
   - Gesture recognition (future)

5. **Haptic Output** - FUTURE
   - Force feedback API
   - Rumble/vibration
   - Architecture ready

---

## 🚀 Evolution Plan: Phase 2 Execution

### Priority 1: Real Device Opening & Type Detection

**Goal**: Open actual evdev devices and detect types

**Files to Modify**:
- `crates/runtime/display/src/input/device.rs`

**Changes**:
1. Add `evdev::Device` field to `Device` struct
2. Implement real `open()` with evdev crate
3. Implement `detect_type()` heuristics
4. Implement `capabilities()` detection

**Deep Debt Compliance**:
- ✅ Pure Rust (evdev crate)
- ✅ Self-knowledge (runtime detection)
- ✅ No hardcoding (capability-based)
- ✅ Zero unsafe (evdev handles it)

### Priority 2: Event Stream Parsing

**Goal**: Parse evdev → InputEvent

**Files to Create/Modify**:
- `crates/runtime/display/src/input/parser.rs` (NEW)
- `crates/runtime/display/src/input/events.rs`

**Changes**:
1. Create `EventParser` struct
2. Implement evdev → InputEvent mapping
3. Track modifier key state
4. Handle relative vs absolute axes

**Deep Debt Compliance**:
- ✅ Complete implementation (no placeholders)
- ✅ Type-safe parsing
- ✅ Error handling

### Priority 3: Async Event Tasks

**Goal**: Spawn async tasks for each device

**Files to Modify**:
- `crates/runtime/display/src/input/mod.rs`
- `crates/runtime/display/src/input/device.rs`

**Changes**:
1. Spawn tokio task per device in `discover()`
2. Stream events to mpsc channel
3. Route events to focused window
4. Handle device disconnection

**Deep Debt Compliance**:
- ✅ Modern async (tokio)
- ✅ Concurrent (parallel device reading)
- ✅ Graceful shutdown

### Priority 4: Multi-Touch Support

**Goal**: Track multiple simultaneous touches

**Files to Create/Modify**:
- `crates/runtime/display/src/input/touch.rs` (NEW)

**Changes**:
1. Create `TouchTracker` struct
2. Track active touch points by slot
3. Generate touch events (Started/Moved/Ended)
4. Map touch coords to window space

**Deep Debt Compliance**:
- ✅ Complete state tracking
- ✅ No assumptions about touch count
- ✅ Agnostic design

### Priority 5 (FUTURE): Haptic Output

**Goal**: Architecture for haptic/force feedback

**Files to Create**:
- `crates/runtime/display/src/input/haptic.rs` (NEW)

**Changes**:
1. Define `HapticEffect` types
2. Implement force feedback upload
3. API for petalTongue to trigger effects

**Deep Debt Compliance**:
- ✅ Agnostic effect types
- ✅ Capability-based (not all devices support)

---

## 🎨 petalTongue Integration Story

### Use Case: Touch-Based UI

```rust
// In petalTongue:
use toadstool_display_client::DisplayClient;

let client = DisplayClient::connect().await?;

// Create window
let window = client.create_window(800, 600).await?;

// Subscribe to ALL input
let mut events = client.subscribe_input(window).await?;

// Handle events
while let Some(event) = events.recv().await {
    match event {
        InputEvent::Touch { id, phase, x, y, .. } => {
            match phase {
                TouchPhase::Started => ui.handle_touch_start(id, x, y),
                TouchPhase::Moved => ui.handle_touch_move(id, x, y),
                TouchPhase::Ended => ui.handle_touch_end(id),
                _ => {}
            }
        }
        InputEvent::MouseMove { x, y, .. } => {
            ui.handle_cursor_move(x, y);
        }
        InputEvent::KeyPress { key, modifiers, .. } => {
            ui.handle_key(key, modifiers);
        }
        _ => {}
    }
}
```

### Use Case: Multi-Touch Gestures

```rust
// In petalTongue's gesture recognizer:
let mut gesture_detector = GestureDetector::new();

for event in touch_events {
    if let Some(gesture) = gesture_detector.process(event) {
        match gesture {
            Gesture::Pinch { scale } => zoom_view(scale),
            Gesture::Rotate { angle } => rotate_view(angle),
            Gesture::Swipe { direction } => change_page(direction),
            _ => {}
        }
    }
}
```

### Use Case: Haptic Feedback (Future)

```rust
// In petalTongue:
// User touches button
if touch_started {
    client.haptic_feedback(HapticEffect::Click).await?;
}

// User drags slider
if dragging {
    client.haptic_feedback(HapticEffect::ContinuousBuzz { intensity: 0.3 }).await?;
}
```

---

## 🧪 Testing Strategy

### Unit Tests:

1. **Device Detection**:
   ```rust
   #[test]
   fn test_detect_keyboard() {
       // Mock evdev device with key capabilities
       // Assert detected as DeviceType::Keyboard
   }
   ```

2. **Event Parsing**:
   ```rust
   #[test]
   fn test_parse_key_press() {
       // evdev key event → InputEvent::KeyPress
   }
   
   #[test]
   fn test_parse_touch_event() {
       // evdev ABS_MT_* → InputEvent::Touch
   }
   ```

3. **Touch Tracking**:
   ```rust
   #[test]
   fn test_multi_touch_tracking() {
       // Track 3 simultaneous touches
       // Verify correct ID assignment
   }
   ```

### Integration Tests:

1. **Full Event Flow**:
   ```rust
   #[tokio::test]
   async fn test_device_to_window() {
       // Device → Parser → InputManager → Window
       // Verify event routing
   }
   ```

2. **Multi-Device**:
   ```rust
   #[tokio::test]
   async fn test_keyboard_and_mouse() {
       // Both devices sending events simultaneously
       // Verify no crosstalk, proper routing
   }
   ```

### Chaos Tests:

1. **Device Hotplug**:
   ```rust
   #[test]
   fn chaos_device_disconnect_during_event() {
       // Unplug device mid-stream
       // Verify graceful handling
   }
   ```

2. **Rapid Touch**:
   ```rust
   #[test]
   fn chaos_rapid_multitouch() {
       // 10 fingers, rapid tapping
       // Verify no touch ID corruption
   }
   ```

---

## 📏 Success Metrics

### Phase 2 Complete When:

1. ✅ **Real Device Opening**: Open actual evdev devices
2. ✅ **Type Detection**: Correctly identify keyboards/mice/touch
3. ✅ **Event Parsing**: evdev → InputEvent working
4. ✅ **Async Streams**: Events flowing via tokio channels
5. ✅ **Multi-Touch**: Track 10+ simultaneous touches
6. ✅ **Test Coverage**: 30+ tests (unit + integration + chaos)
7. ✅ **Zero Placeholders**: All TODOs resolved or moved to Phase 3
8. ✅ **Deep Debt A+**: All principles maintained

### petalTongue Ready When:

1. ✅ IPC client can subscribe to events
2. ✅ ALL event types flow correctly
3. ✅ Touch events include proper IDs
4. ✅ Focus routing works reliably
5. ✅ Performance: <1ms event latency
6. ✅ Documentation: petalTongue integration guide

---

## 🌊 Execution Order

### Session 1 (Now): Device Opening & Type Detection
- Modify `device.rs`
- Add real evdev opening
- Implement type detection
- Add capability queries
- **~2 hours**

### Session 2: Event Parsing
- Create `parser.rs`
- Implement evdev → InputEvent
- Track modifier keys
- Handle mouse/keyboard events
- **~2 hours**

### Session 3: Async Streams
- Modify `mod.rs`
- Spawn device tasks
- Stream to channels
- Route to windows
- **~2 hours**

### Session 4: Multi-Touch
- Create `touch.rs`
- Implement TouchTracker
- Parse MT events
- Generate touch events
- **~2 hours**

### Session 5: Testing & Integration
- Unit tests for all
- Integration tests
- Chaos tests
- petalTongue test client
- **~2 hours**

**Total Estimate**: 10 hours for complete Phase 2

---

## 🎯 Ready to Execute?

**Current State**: Phase 1 architecture solid  
**Next Step**: Start with Priority 1 (Real Device Opening)  
**Goal**: petalTongue can consume ALL hardware input  
**Timeline**: Complete Phase 2 in upcoming sessions

**Question for you**: 

Should we:
1. **Execute Priority 1 now** (Device opening & detection - 2 hours)?
2. **Execute all of Phase 2** (Full marathon - 10 hours)?
3. **Something else** (your direction)?

---

*"toadstool grows in the metal, petalTongue blooms on top!"* 🌱🎨✨
