// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(target_os = "linux")]
use std::path::Path;

#[cfg(target_os = "linux")]
use toadstool_common::platform;

/// Linux device file opener — wraps `rustix::fs::open` with platform flags.
///
/// Implements [`platform::DeviceFile`] for opening `/dev/*` nodes with
/// appropriate `O_RDWR | O_CLOEXEC` (and optionally `O_NONBLOCK`).
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxDeviceFile;

#[cfg(target_os = "linux")]
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

/// Safe device I/O operations on borrowed file descriptors.
///
/// Provides `read`, `write`, and `poll` wrappers using `rustix::io` so that
/// consumer crates don't need to depend on rustix directly.
#[cfg(target_os = "linux")]
pub struct LinuxDeviceIo;

#[cfg(target_os = "linux")]
impl LinuxDeviceIo {
    /// Read from a borrowed file descriptor into `buf`.
    ///
    /// Returns the number of bytes read, or an I/O error.
    #[inline]
    pub fn read(fd: std::os::fd::BorrowedFd<'_>, buf: &mut [u8]) -> std::io::Result<usize> {
        rustix::io::read(fd, buf).map_err(std::io::Error::from)
    }

    /// Write `data` to a borrowed file descriptor.
    ///
    /// Returns the number of bytes written, or an I/O error.
    #[inline]
    pub fn write(fd: std::os::fd::BorrowedFd<'_>, data: &[u8]) -> std::io::Result<usize> {
        rustix::io::write(fd, data).map_err(std::io::Error::from)
    }

    /// Poll a file descriptor for readability with an optional timeout.
    ///
    /// Returns `Ok(true)` if data is available, `Ok(false)` on timeout.
    /// Pass `None` for infinite blocking.
    pub fn poll_read(
        fd: std::os::fd::BorrowedFd<'_>,
        timeout_ms: Option<i32>,
    ) -> std::io::Result<bool> {
        let ts = timeout_ms.map(|ms| rustix::time::Timespec {
            tv_sec: i64::from(ms / 1000),
            tv_nsec: i64::from(ms % 1000) * 1_000_000,
        });
        let mut pfds = [rustix::event::PollFd::new(
            &fd,
            rustix::event::PollFlags::IN,
        )];
        let n = rustix::event::poll(&mut pfds, ts.as_ref()).map_err(std::io::Error::from)?;
        Ok(n > 0)
    }

    /// Positional read from a file descriptor (does not change file offset).
    #[inline]
    pub fn pread(
        fd: std::os::fd::BorrowedFd<'_>,
        buf: &mut [u8],
        offset: u64,
    ) -> std::io::Result<usize> {
        rustix::io::pread(fd, buf, offset).map_err(std::io::Error::from)
    }

    /// Positional write to a file descriptor (does not change file offset).
    #[inline]
    pub fn pwrite(
        fd: std::os::fd::BorrowedFd<'_>,
        data: &[u8],
        offset: u64,
    ) -> std::io::Result<usize> {
        rustix::io::pwrite(fd, data, offset).map_err(std::io::Error::from)
    }
}

/// Create a character device node at `path`.
///
/// Wraps `mknodat(CWD, ...)` with `CharacterDevice` type.
#[cfg(target_os = "linux")]
pub fn mknod_char(path: &Path, mode: u32, major: u32, minor: u32) -> std::io::Result<()> {
    rustix::fs::mknodat(
        rustix::fs::CWD,
        path,
        rustix::fs::FileType::CharacterDevice,
        rustix::fs::Mode::from_raw_mode(mode as rustix::fs::RawMode),
        rustix::fs::makedev(major, minor),
    )
    .map_err(std::io::Error::from)
}

/// Open a file with specified flags (wraps `rustix::fs::open`).
#[cfg(target_os = "linux")]
pub fn open_path(path: &Path, rdwr: bool, sync: bool) -> std::io::Result<std::os::fd::OwnedFd> {
    let mut flags = rustix::fs::OFlags::CLOEXEC;
    if rdwr {
        flags |= rustix::fs::OFlags::RDWR;
    } else {
        flags |= rustix::fs::OFlags::RDONLY;
    }
    if sync {
        flags |= rustix::fs::OFlags::SYNC;
    }
    rustix::fs::open(path, flags, rustix::fs::Mode::empty()).map_err(std::io::Error::from)
}

/// Open a file write-only (for sysfs control files that have no read fop).
#[cfg(target_os = "linux")]
pub fn open_path_wronly(path: &Path) -> std::io::Result<std::os::fd::OwnedFd> {
    rustix::fs::open(
        path,
        rustix::fs::OFlags::WRONLY | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(std::io::Error::from)
}

/// Seek to the end of a file descriptor.
#[cfg(target_os = "linux")]
pub fn seek_end(fd: std::os::fd::BorrowedFd<'_>) -> std::io::Result<u64> {
    rustix::fs::seek(fd, rustix::fs::SeekFrom::End(0)).map_err(std::io::Error::from)
}
