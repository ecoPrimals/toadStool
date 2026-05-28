// SPDX-License-Identifier: AGPL-3.0-or-later
//! systemd FileDescriptorStore integration for VFIO warm keepalive.
//!
//! On SIGTERM, stores VFIO device fds in systemd's fd store via the
//! `sd_notify` protocol with `SCM_RIGHTS`. systemd holds duplicated fds
//! in PID 1 — they survive our process exit, preventing the kernel from
//! releasing the VFIO group and triggering a Secondary Bus Reset.
//!
//! On startup, retrieves stored fds from `$LISTEN_FDS` / `$LISTEN_FDNAMES`
//! and reconstructs `VfioAnchor`s so the GPU stays warm.

use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::sync::Arc;

use tracing::{info, warn};
use toadstool_ember::VfioAnchor;

const SD_LISTEN_FDS_START: i32 = 3;

/// Send a bare `sd_notify` message (no fds).
pub(crate) fn sd_notify(msg: &str) -> std::io::Result<()> {
    sd_notify_with_fds(msg, &[])
}

/// Send an `sd_notify` message with file descriptors via `SCM_RIGHTS`.
fn sd_notify_with_fds(msg: &str, fds: &[BorrowedFd<'_>]) -> std::io::Result<()> {
    let socket_path = std::env::var(toadstool_common::interned_strings::socket_env::NOTIFY_SOCKET)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "NOTIFY_SOCKET not set"))?;

    let addr = if let Some(abstract_name) = socket_path.strip_prefix('@') {
        rustix::net::SocketAddrUnix::new_abstract_name(abstract_name.as_bytes())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?
    } else {
        rustix::net::SocketAddrUnix::new(&socket_path)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?
    };

    let sock = rustix::net::socket(
        rustix::net::AddressFamily::UNIX,
        rustix::net::SocketType::DGRAM,
        None,
    )
    .map_err(std::io::Error::other)?;

    let iov = [rustix::io::IoSlice::new(msg.as_bytes())];

    if fds.is_empty() {
        rustix::net::sendmsg_addr(
            &sock,
            &addr,
            &iov,
            &mut rustix::net::SendAncillaryBuffer::default(),
            rustix::net::SendFlags::empty(),
        )
        .map_err(std::io::Error::other)?;
    } else {
        // Allocate space for up to 4 fds (device + backend + group + spare)
        let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(4))];
        let mut cmsg_buf = rustix::net::SendAncillaryBuffer::new(&mut space);
        cmsg_buf.push(rustix::net::SendAncillaryMessage::ScmRights(fds));

        rustix::net::sendmsg_addr(
            &sock,
            &addr,
            &iov,
            &mut cmsg_buf,
            rustix::net::SendFlags::empty(),
        )
        .map_err(std::io::Error::other)?;
    }

    Ok(())
}

fn bdf_to_fdname(bdf: &str) -> String {
    bdf.replace(':', "_")
}

fn fdname_to_bdf(name: &str) -> String {
    // Reverse: first `_` group stays, convert `_` back to `:`
    // 0000_02_00.0 → 0000:02:00.0
    let parts: Vec<&str> = name.splitn(4, '_').collect();
    if parts.len() >= 3 {
        format!("{}:{}:{}", parts[0], parts[1], parts[2..].join("_"))
    } else {
        name.replace('_', ":")
    }
}

/// Store a single fd in systemd's fd store with the given name.
fn store_fd(fd: BorrowedFd<'_>, fdname: &str) -> std::io::Result<()> {
    let msg = format!("FDSTORE=1\nFDNAME={fdname}\n");
    sd_notify_with_fds(&msg, &[fd])
}

/// Store all `VfioAnchor`s in systemd's fd store.
///
/// Each anchor contributes 2-3 named fds:
/// - `vfio-dev-{bdf}` — the VFIO device fd
/// - `vfio-iommufd-{bdf}-{ioas_id}` — iommufd backend fd
/// - `vfio-container-{bdf}` — legacy container fd
/// - `vfio-group-{bdf}` — legacy group fd
pub(crate) fn store_anchors(anchors: &HashMap<String, VfioAnchor>) -> usize {
    let mut stored = 0usize;

    for (bdf, anchor) in anchors {
        let safe_bdf = bdf_to_fdname(bdf);

        // Store device fd
        let dev_name = format!("vfio-dev-{safe_bdf}");
        match store_fd(anchor.device_fd(), &dev_name) {
            Ok(()) => {
                info!(bdf, fdname = %dev_name, "stored device fd in systemd");
                stored += 1;
            }
            Err(e) => {
                warn!(bdf, fdname = %dev_name, err = %e, "failed to store device fd");
                continue;
            }
        }

        // Store backend fd
        let backend_name = if let Some(ioas_id) = anchor.ioas_id() {
            format!("vfio-iommufd-{safe_bdf}-{ioas_id}")
        } else {
            format!("vfio-container-{safe_bdf}")
        };

        match store_fd(anchor.backend_fd(), &backend_name) {
            Ok(()) => {
                info!(bdf, fdname = %backend_name, "stored backend fd in systemd");
                stored += 1;
            }
            Err(e) => {
                warn!(bdf, fdname = %backend_name, err = %e, "failed to store backend fd");
            }
        }

        // For legacy backend, also store the group fd
        if let Some(group_fd) = anchor.group_fd() {
            let group_name = format!("vfio-group-{safe_bdf}");
            match store_fd(group_fd, &group_name) {
                Ok(()) => {
                    info!(bdf, fdname = %group_name, "stored group fd in systemd");
                    stored += 1;
                }
                Err(e) => {
                    warn!(bdf, fdname = %group_name, err = %e, "failed to store group fd");
                }
            }
        }
    }

    stored
}

/// Retrieve stored fds from systemd and reconstruct `VfioAnchor`s.
///
/// Called on startup to recover anchors that were stored during the
/// previous daemon's SIGTERM handler.
pub(crate) fn retrieve_anchors() -> HashMap<String, VfioAnchor> {
    use toadstool_common::interned_strings::socket_env;

    let listen_fds: usize = std::env::var(socket_env::LISTEN_FDS)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let listen_pid: u32 = std::env::var(socket_env::LISTEN_PID)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if listen_fds == 0 || listen_pid != std::process::id() {
        return HashMap::new();
    }

    let names: Vec<String> = std::env::var(socket_env::LISTEN_FDNAMES)
        .ok()
        .map(|s| s.split(':').map(String::from).collect())
        .unwrap_or_default();

    info!(
        listen_fds,
        listen_pid,
        names = ?names,
        "systemd passed stored fds"
    );

    let mut named_fds: Vec<(String, OwnedFd)> = Vec::new();
    for i in 0..listen_fds {
        let fd_num = SD_LISTEN_FDS_START + i as i32;
        let name = names.get(i).cloned().unwrap_or_default();
        // SAFETY: systemd guarantees these fd numbers are valid, owned by us,
        // and that LISTEN_PID matches our PID (checked above).
        let fd = unsafe { OwnedFd::from_raw_fd(fd_num) };
        info!(fd = fd.as_raw_fd(), fdname = %name, "retrieved stored fd");
        named_fds.push((name, fd));
    }

    // SAFETY: single-threaded at this point in startup (before tokio runtime
    // spawns worker threads). Clearing these prevents child processes from
    // accidentally consuming the stored fds.
    unsafe {
        std::env::remove_var("LISTEN_FDS");
        std::env::remove_var("LISTEN_PID");
        std::env::remove_var("LISTEN_FDNAMES");
    }

    // Group fds by BDF and reconstruct anchors
    reconstruct_anchors(named_fds)
}

/// Group stored fds by BDF and build VfioAnchors.
fn reconstruct_anchors(named_fds: Vec<(String, OwnedFd)>) -> HashMap<String, VfioAnchor> {
    // Parse names and group by BDF
    struct FdSet {
        device_fd: Option<OwnedFd>,
        iommufd: Option<(OwnedFd, u32)>,  // fd + ioas_id
        container: Option<OwnedFd>,
        group: Option<OwnedFd>,
    }

    let mut by_bdf: HashMap<String, FdSet> = HashMap::new();

    for (name, fd) in named_fds {
        if let Some(rest) = name.strip_prefix("vfio-dev-") {
            let bdf = fdname_to_bdf(rest);
            by_bdf.entry(bdf).or_insert_with(|| FdSet {
                device_fd: None, iommufd: None, container: None, group: None,
            }).device_fd = Some(fd);
        } else if let Some(rest) = name.strip_prefix("vfio-iommufd-") {
            // Format: vfio-iommufd-{bdf}-{ioas_id}
            // Find the last `-` to split bdf from ioas_id
            if let Some(last_dash) = rest.rfind('-') {
                let bdf_part = &rest[..last_dash];
                let ioas_str = &rest[last_dash + 1..];
                let bdf = fdname_to_bdf(bdf_part);
                let ioas_id: u32 = ioas_str.parse().unwrap_or(0);
                by_bdf.entry(bdf).or_insert_with(|| FdSet {
                    device_fd: None, iommufd: None, container: None, group: None,
                }).iommufd = Some((fd, ioas_id));
            }
        } else if let Some(rest) = name.strip_prefix("vfio-container-") {
            let bdf = fdname_to_bdf(rest);
            by_bdf.entry(bdf).or_insert_with(|| FdSet {
                device_fd: None, iommufd: None, container: None, group: None,
            }).container = Some(fd);
        } else if let Some(rest) = name.strip_prefix("vfio-group-") {
            let bdf = fdname_to_bdf(rest);
            by_bdf.entry(bdf).or_insert_with(|| FdSet {
                device_fd: None, iommufd: None, container: None, group: None,
            }).group = Some(fd);
        } else {
            warn!(fdname = %name, "unrecognized stored fd name — skipping");
        }
    }

    let mut anchors = HashMap::new();

    for (bdf, fds) in by_bdf {
        let Some(device_fd) = fds.device_fd else {
            warn!(bdf, "no device fd found in stored fds — cannot reconstruct anchor");
            continue;
        };

        let anchor = if let Some((iommufd, ioas_id)) = fds.iommufd {
            VfioAnchor::from_iommufd(bdf.clone(), device_fd, Arc::new(iommufd), ioas_id)
        } else if let Some(container) = fds.container {
            if let Some(group) = fds.group {
                VfioAnchor::from_legacy(bdf.clone(), device_fd, Arc::new(container), group)
            } else {
                warn!(bdf, "legacy backend missing group fd — cannot reconstruct anchor");
                continue;
            }
        } else {
            warn!(bdf, "no backend fd found — cannot reconstruct anchor");
            continue;
        };

        info!(bdf, "reconstructed VfioAnchor from systemd fd store");
        anchors.insert(bdf, anchor);
    }

    anchors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bdf_round_trip() {
        let bdf = "0000:02:00.0";
        let encoded = bdf_to_fdname(bdf);
        assert_eq!(encoded, "0000_02_00.0");
        let decoded = fdname_to_bdf(&encoded);
        assert_eq!(decoded, bdf);
    }

    #[test]
    fn bdf_round_trip_slot49() {
        let bdf = "0000:49:00.0";
        let encoded = bdf_to_fdname(bdf);
        assert_eq!(encoded, "0000_49_00.0");
        let decoded = fdname_to_bdf(&encoded);
        assert_eq!(decoded, bdf);
    }

    #[test]
    fn sd_notify_missing_socket() {
        // SAFETY: test runs single-threaded (serial test runner)
        unsafe { std::env::remove_var("NOTIFY_SOCKET") };
        let err = sd_notify("READY=1\n").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn retrieve_no_stored_fds() {
        // SAFETY: test runs single-threaded (serial test runner)
        unsafe {
            std::env::remove_var("LISTEN_FDS");
            std::env::remove_var("LISTEN_PID");
        }
        let anchors = retrieve_anchors();
        assert!(anchors.is_empty());
    }
}
