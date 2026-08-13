// SPDX-License-Identifier: AGPL-3.0-or-later

/// Re-export rustix ioctl infrastructure for consumer crates.
///
/// Consumer crates implement these traits on their device-specific adapter
/// types without importing rustix directly.
#[cfg(target_os = "linux")]
pub mod ioctl_infra {
    pub use rustix::io::Errno;
    pub use rustix::io::Result as IoResult;
    pub use rustix::ioctl::{Getter, Ioctl, IoctlOutput, Opcode, Setter, Updater, ioctl, opcode};
}

/// Receive data and file descriptors via SCM_RIGHTS from a Unix socket.
///
/// Returns `(bytes_read, received_fds)`. Up to `max_fds` file descriptors
/// will be extracted from the ancillary control message.
#[cfg(target_os = "linux")]
pub fn recv_with_fds(
    sock: impl std::os::fd::AsFd,
    buf: &mut [u8],
    max_fds: usize,
) -> std::io::Result<(usize, Vec<std::os::fd::OwnedFd>)> {
    use std::mem::MaybeUninit;
    let mut iov = [rustix::io::IoSliceMut::new(buf)];

    let space_size = max_fds * (std::mem::size_of::<std::os::fd::RawFd>() + 16) + 32;
    let mut recv_space: Vec<MaybeUninit<u8>> = vec![MaybeUninit::uninit(); space_size];
    let mut control = rustix::net::RecvAncillaryBuffer::new(&mut recv_space);

    let msg = rustix::net::recvmsg(
        sock,
        &mut iov,
        &mut control,
        rustix::net::RecvFlags::empty(),
    )
    .map_err(std::io::Error::from)?;

    let mut fds = Vec::new();
    for ancillary in control.drain() {
        if let rustix::net::RecvAncillaryMessage::ScmRights(iter) = ancillary {
            fds.extend(iter);
        }
    }

    Ok((msg.bytes, fds))
}

/// Create a Unix DGRAM socket.
#[cfg(target_os = "linux")]
pub fn unix_dgram_socket() -> std::io::Result<std::os::fd::OwnedFd> {
    rustix::net::socket(
        rustix::net::AddressFamily::UNIX,
        rustix::net::SocketType::DGRAM,
        None,
    )
    .map_err(std::io::Error::from)
}

/// Unix socket address (filesystem or abstract).
pub enum UnixAddr {
    /// Filesystem-based socket path.
    Path(std::path::PathBuf),
    /// Linux abstract namespace socket name.
    Abstract(Vec<u8>),
}

/// Send a message with optional file descriptors (SCM_RIGHTS) over a Unix socket.
#[cfg(target_os = "linux")]
pub fn sendmsg_with_fds(
    sock: impl std::os::fd::AsFd,
    addr: &UnixAddr,
    data: &[u8],
    fds: &[std::os::fd::BorrowedFd<'_>],
) -> std::io::Result<()> {
    use std::mem::MaybeUninit;

    let unix_addr = match addr {
        UnixAddr::Path(p) => rustix::net::SocketAddrUnix::new(p).map_err(std::io::Error::from)?,
        UnixAddr::Abstract(name) => {
            rustix::net::SocketAddrUnix::new_abstract_name(name).map_err(std::io::Error::from)?
        }
    };

    let iov = [rustix::io::IoSlice::new(data)];

    if fds.is_empty() {
        rustix::net::sendmsg_addr(
            sock,
            &unix_addr,
            &iov,
            &mut rustix::net::SendAncillaryBuffer::default(),
            rustix::net::SendFlags::empty(),
        )
        .map_err(std::io::Error::from)?;
    } else {
        let space_size = fds.len() * (std::mem::size_of::<std::os::fd::RawFd>() + 16) + 32;
        let mut space: Vec<MaybeUninit<u8>> = vec![MaybeUninit::uninit(); space_size];
        let mut cmsg_buf = rustix::net::SendAncillaryBuffer::new(&mut space);
        cmsg_buf.push(rustix::net::SendAncillaryMessage::ScmRights(fds));
        rustix::net::sendmsg_addr(
            sock,
            &unix_addr,
            &iov,
            &mut cmsg_buf,
            rustix::net::SendFlags::empty(),
        )
        .map_err(std::io::Error::from)?;
    }
    Ok(())
}
