// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code, reason = "mmap/mlock require unsafe — containment zone")]

//! [`toadstool_common::platform`] trait implementations for Linux.
//!
//! Provides concrete `MemoryMapper` and `PinnedMemory` implementations using
//! `rustix` syscalls. These are the "L3 backend" for the G68 platform traits.

use std::path::Path;

use toadstool_common::platform;

use crate::safe_mmap::{MmapError, SafeMmapRegion};

/// Linux memory mapper — creates [`SafeMmapRegion`] handles from file paths.
///
/// Implements [`platform::MemoryMapper`] using `rustix::mm::mmap`.
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxMemoryMapper;

impl platform::MemoryMapper for LinuxMemoryMapper {
    type Mapping = SafeMmapRegion;
    type Error = MmapError;

    fn map_file(
        &self,
        path: &Path,
        _offset: u64,
        _length: usize,
        writable: bool,
    ) -> Result<Self::Mapping, Self::Error> {
        if writable {
            SafeMmapRegion::map_shared_rw(path)
        } else {
            SafeMmapRegion::map_shared_ro(path)
        }
    }

    fn map_anonymous(&self, length: usize) -> Result<Self::Mapping, Self::Error> {
        use std::ptr;

        if length == 0 {
            return Err(MmapError::ZeroSize {
                path: "<anonymous>".to_string(),
            });
        }

        // SAFETY: null hint, valid length, anonymous mapping (no fd).
        let raw = unsafe {
            rustix::mm::mmap_anonymous(
                ptr::null_mut(),
                length,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::PRIVATE,
            )
        }
        .map_err(|e| MmapError::MmapFailed {
            path: "<anonymous>".to_string(),
            source: e.into(),
        })?;

        let ptr = std::ptr::NonNull::new(raw.cast()).ok_or_else(|| MmapError::NullPointer {
            path: "<anonymous>".to_string(),
        })?;

        Ok(SafeMmapRegion::from_anonymous(ptr, length))
    }
}

/// Linux memory pinner — mlock/munlock for DMA-safe page pinning.
///
/// Implements [`platform::PinnedMemory`] using `rustix::mm::mlock`/`munlock`.
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxPinnedMemory;

impl platform::PinnedMemory for LinuxPinnedMemory {
    type Error = std::io::Error;

    fn pin(&self, region: &[u8]) -> Result<(), Self::Error> {
        // SAFETY: region is a valid slice, so ptr + len describe a valid memory range.
        // mlock requires *mut but only reads the pages (no modification).
        unsafe {
            rustix::mm::mlock(
                region.as_ptr().cast_mut().cast::<std::ffi::c_void>(),
                region.len(),
            )
        }
        .map_err(std::io::Error::from)
    }

    fn unpin(&self, region: &[u8]) -> Result<(), Self::Error> {
        // SAFETY: region is a valid slice from a previously locked mapping.
        unsafe {
            rustix::mm::munlock(
                region.as_ptr().cast_mut().cast::<std::ffi::c_void>(),
                region.len(),
            )
        }
        .map_err(std::io::Error::from)
    }
}

/// Linux event notification channel — eventfd + poll.
///
/// Implements [`platform::EventNotifier`] using `rustix::event::eventfd`/`poll`.
/// Used by VFIO IRQ wiring and NVPmu interrupt handling.
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxEventNotifier;

/// Opaque event handle wrapping a Linux eventfd.
pub struct LinuxEvent {
    fd: rustix::fd::OwnedFd,
}

impl std::fmt::Debug for LinuxEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::os::fd::AsFd;
        f.debug_struct("LinuxEvent")
            .field("fd", &self.fd.as_fd())
            .finish()
    }
}

impl LinuxEvent {
    /// Access the underlying fd for integration with external epoll/VFIO wiring.
    pub fn as_fd(&self) -> rustix::fd::BorrowedFd<'_> {
        use std::os::fd::AsFd;
        self.fd.as_fd()
    }

    /// Consume and return the owned fd (for passing to VFIO SET_IRQS).
    pub fn into_fd(self) -> rustix::fd::OwnedFd {
        self.fd
    }
}

// SAFETY: OwnedFd is Send; eventfd has no thread-local state.
unsafe impl Send for LinuxEvent {}
// SAFETY: eventfd read/write are atomic; concurrent poll is safe.
unsafe impl Sync for LinuxEvent {}

impl platform::EventNotifier for LinuxEventNotifier {
    type Error = std::io::Error;
    type Event = LinuxEvent;

    fn create(&self) -> Result<Self::Event, Self::Error> {
        let fd = rustix::event::eventfd(0, rustix::event::EventfdFlags::NONBLOCK)
            .map_err(std::io::Error::from)?;
        Ok(LinuxEvent { fd })
    }

    fn signal(&self, event: &Self::Event) -> Result<(), Self::Error> {
        let buf = 1u64.to_ne_bytes();
        rustix::io::write(&event.fd, &buf).map_err(std::io::Error::from)?;
        Ok(())
    }

    fn wait(
        &self,
        event: &Self::Event,
        timeout: std::time::Duration,
    ) -> Result<Option<u64>, Self::Error> {
        use std::os::fd::AsFd;

        let ts = rustix::time::Timespec {
            tv_sec: timeout.as_secs() as i64,
            tv_nsec: i64::from(timeout.subsec_nanos()),
        };
        let borrowed = event.fd.as_fd();
        let mut pfds = [rustix::event::PollFd::new(
            &borrowed,
            rustix::event::PollFlags::IN,
        )];

        match rustix::event::poll(&mut pfds, Some(&ts)) {
            Ok(n) if n > 0 => {
                let mut buf = [0u8; 8];
                match rustix::io::read(&event.fd, &mut buf) {
                    Ok(8) => Ok(Some(u64::from_ne_bytes(buf))),
                    _ => Ok(Some(1)),
                }
            }
            Ok(_) => Ok(None),
            Err(e) => Err(std::io::Error::from(e)),
        }
    }
}

/// Linux device file opener — wraps `rustix::fs::open` with platform flags.
///
/// Implements [`platform::DeviceFile`] for opening `/dev/*` nodes with
/// appropriate `O_RDWR | O_CLOEXEC` (and optionally `O_NONBLOCK`).
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxDeviceFile;

impl platform::DeviceFile for LinuxDeviceFile {
    type Error = std::io::Error;
    type Handle = rustix::fd::OwnedFd;

    fn open(
        &self,
        path: &Path,
        writable: bool,
        non_blocking: bool,
    ) -> Result<Self::Handle, Self::Error> {
        let mut flags = rustix::fs::OFlags::CLOEXEC;
        if writable {
            flags |= rustix::fs::OFlags::RDWR;
        } else {
            flags |= rustix::fs::OFlags::RDONLY;
        }
        if non_blocking {
            flags |= rustix::fs::OFlags::NONBLOCK;
        }

        rustix::fs::open(path, flags, rustix::fs::Mode::empty()).map_err(std::io::Error::from)
    }

    fn read(&self, handle: &Self::Handle, buf: &mut [u8]) -> Result<usize, Self::Error> {
        rustix::io::read(handle, buf).map_err(std::io::Error::from)
    }

    fn write(&self, handle: &Self::Handle, buf: &[u8]) -> Result<usize, Self::Error> {
        rustix::io::write(handle, buf).map_err(std::io::Error::from)
    }
}
