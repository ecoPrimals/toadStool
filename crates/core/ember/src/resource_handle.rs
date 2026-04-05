// SPDX-License-Identifier: AGPL-3.0-or-later

//! Abstract handle to an exclusive hardware resource.
//!
//! Every hardware class implements [`ResourceHandle`] for its exclusive
//! access pattern:
//!
//! - **GPU (VFIO)**: container fd + group fd + device fd
//! - **USB**: claimed interface handle
//! - **HSM / TEE**: cryptographic session
//! - **DRM**: primary node fd
//! - **NPU**: MMIO-mapped region reference
//! - **Bluetooth**: HCI socket
//!
//! ember does not know what kind of handle it holds — only that it can
//! check liveness and release it.

use std::fmt;

/// Abstract exclusive handle to a hardware resource.
///
/// Implementors represent the minimum state needed to keep a hardware
/// resource exclusively claimed. When dropped, the resource should be
/// released back to the kernel.
pub trait ResourceHandle: Send + Sync + fmt::Debug {
    /// The error type for handle operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Human-readable name for this handle type (e.g. "vfio", "usb-claim", "drm-primary").
    fn handle_type(&self) -> &str;

    /// Whether the underlying resource is still alive and accessible.
    fn is_alive(&self) -> bool;

    /// Release the exclusive hold on the resource.
    ///
    /// After this call, [`is_alive`](ResourceHandle::is_alive) should return `false`.
    /// Idempotent — calling release on an already-released handle is a no-op.
    ///
    /// # Errors
    ///
    /// Returns an error if the kernel refuses the release (e.g. device in
    /// D3cold, pending DMA, etc.).
    fn release(&mut self) -> Result<(), Self::Error>;

    /// Attempt to reacquire the resource after a release.
    ///
    /// Not all handle types support reacquisition — returns `false` if
    /// the handle type requires a fresh open instead.
    ///
    /// # Errors
    ///
    /// Returns an error if reacquisition fails (e.g. device claimed by
    /// another process).
    fn reacquire(&mut self) -> Result<bool, Self::Error>;
}
