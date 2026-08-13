// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code, reason = "mmap/mlock require unsafe — containment zone")]

#[cfg(target_os = "linux")]
use toadstool_common::platform;

/// Linux event notification channel — eventfd + poll.
///
/// Implements [`platform::EventNotifier`] using `rustix::event::eventfd`/`poll`.
/// Used by VFIO IRQ wiring and NVPmu interrupt handling.
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxEventNotifier;

/// Opaque event handle wrapping a Linux eventfd.
#[cfg(target_os = "linux")]
pub struct LinuxEvent {
    fd: rustix::fd::OwnedFd,
}

#[cfg(target_os = "linux")]
impl std::fmt::Debug for LinuxEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::os::fd::AsFd;
        f.debug_struct("LinuxEvent")
            .field("fd", &self.fd.as_fd())
            .finish()
    }
}

#[cfg(target_os = "linux")]
impl LinuxEvent {
    /// Access the underlying fd for integration with external epoll/VFIO wiring.
    pub fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        use std::os::fd::AsFd;
        self.fd.as_fd()
    }

    /// Consume and return the owned fd (for passing to VFIO SET_IRQS).
    pub fn into_fd(self) -> std::os::fd::OwnedFd {
        self.fd
    }

    /// Access the underlying fd as a raw fd for ioctl integration.
    pub fn as_raw_fd(&self) -> std::os::fd::RawFd {
        use std::os::fd::AsRawFd;
        self.fd.as_raw_fd()
    }
}

// SAFETY: OwnedFd is Send; eventfd has no thread-local state.
#[cfg(target_os = "linux")]
unsafe impl Send for LinuxEvent {}
// SAFETY: eventfd read/write are atomic; concurrent poll is safe.
#[cfg(target_os = "linux")]
unsafe impl Sync for LinuxEvent {}

#[cfg(target_os = "linux")]
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
