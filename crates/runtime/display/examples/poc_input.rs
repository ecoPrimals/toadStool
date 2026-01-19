//! Input Proof of Concept
//!
//! Demonstrates enumerating input devices and reading events.
//!
//! **Pure Rust!** No libevdev dependency!
//!
//! ## Usage
//!
//! ```bash
//! # Run with proper permissions (input device access)
//! sudo cargo run --example poc_input
//! ```
//!
//! ## Expected Output
//!
//! - Lists all input devices
//! - Reads keyboard events
//! - Reads mouse events
//! - Prints events to console
//! - Runs for 10 seconds

#[allow(unused_imports)]
use anyhow::{Context, Result};
use std::time::Duration;

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    tracing::info!("🍄 Toadstool Display Backend - Input PoC");
    tracing::info!("Goal: 100% Pure Rust input handling!");

    // Phase 0: Proof of Concept
    // TODO: Implement input device operations

    tracing::info!("📋 Phase 0 Tasks:");
    tracing::info!("  1. Enumerate /dev/input/event* devices");
    tracing::info!("  2. Open keyboard device");
    tracing::info!("  3. Open mouse device");
    tracing::info!("  4. Read events (async)");
    tracing::info!("  5. Parse and print events");
    tracing::info!("  6. Handle hotplug (future)");

    tracing::warn!("⚠️  TODO: Implement input operations");
    tracing::warn!("    See: evdev crate documentation");
    tracing::warn!("    See: specs/DISPLAY_BACKEND_SPEC.md");

    // Placeholder for now
    tracing::info!("💤 Sleeping for 2 seconds (placeholder)...");
    std::thread::sleep(Duration::from_secs(2));

    tracing::info!("✅ PoC structure ready!");
    tracing::info!("🚀 Next: Implement device enumeration");

    Ok(())
}

// TODO: Phase 0 Implementation
//
// Step 1: Enumerate devices
// ```rust
// let devices = evdev::enumerate()?;
// for (path, device) in devices {
//     println!("Found: {} at {}", device.name()?, path.display());
// }
// ```
//
// Step 2: Open specific device
// ```rust
// let mut keyboard = evdev::Device::open("/dev/input/event3")?;
// ```
//
// Step 3: Read events (async with tokio)
// ```rust
// let mut events = keyboard.into_event_stream()?;
// while let Some(event) = events.next_event().await? {
//     match event.kind() {
//         evdev::InputEventKind::Key(key) => {
//             println!("Key: {:?} = {:?}", key, event.value());
//         }
//         evdev::InputEventKind::RelAxis(axis) => {
//             println!("Mouse: {:?} = {}", axis, event.value());
//         }
//         _ => {}
//     }
// }
// ```
//
// Step 4: Handle multiple devices concurrently
// ```rust
// tokio::select! {
//     event = keyboard_stream.next_event() => { /* ... */ }
//     event = mouse_stream.next_event() => { /* ... */ }
// }
// ```
