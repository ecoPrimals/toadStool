// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ember FD-sharing client for diagnostic binaries.
//!
//! Connects to coral-ember's Unix socket, requests VFIO FDs via
//! `SCM_RIGHTS`, and builds a [`VfioDevice`] + [`MappedBar`] for
//! direct BAR0 access. The ember keeps the original FDs alive, so
//! dropping the session closes only the dup'd copies — no VFIO
//! group reset fires.

use std::borrow::Cow;
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::net::UnixStream;

use crate::error::{DriverError, DriverResult};
use crate::vfio::device::MappedBar;
use crate::vfio::{ReceivedVfioFds, VfioDevice};

const MAX_RESPONSE: usize = 4096;

/// Resolves the default ember socket path.
///
/// Priority: `TOADSTOOL_EMBER_SOCKET` →
/// `{XDG_RUNTIME_DIR}/{namespace}/toadstool-ember-{family}.sock`.
fn default_ember_socket_path_without_env_override() -> String {
    use std::path::PathBuf;
    use toadstool_common::interned_strings::socket_env;

    let base = std::env::var(socket_env::XDG_RUNTIME_DIR)
        .map_or_else(|_| std::env::temp_dir(), PathBuf::from);
    let ns =
        std::env::var(socket_env::BIOMEOS_ECOSYSTEM_NAMESPACE).unwrap_or_else(|_| "biomeos".into());
    let family = std::env::var(socket_env::BIOMEOS_FAMILY_ID).unwrap_or_else(|_| "default".into());
    base.join(ns)
        .join(format!("toadstool-ember-{family}.sock"))
        .display()
        .to_string()
}

/// Default ember socket path.
///
/// Priority: `TOADSTOOL_EMBER_SOCKET` → default path.
pub(super) fn default_socket() -> String {
    use toadstool_common::interned_strings::socket_env;

    if let Some(v) = std::env::var(socket_env::TOADSTOOL_EMBER_SOCKET)
        .ok()
        .filter(|s| !s.is_empty())
    {
        return v;
    }
    default_ember_socket_path_without_env_override()
}

/// A VFIO session obtained from coral-ember via FD sharing.
///
/// Provides direct BAR0 access through the ember-held VFIO device.
/// Dropping this struct closes only the dup'd FD copies — ember's
/// originals keep the VFIO group alive (no bus reset).
pub struct EmberSession {
    /// The VFIO device built from ember's dup'd FDs.
    pub device: VfioDevice,
    /// BAR0 mmap for direct MMIO read/write.
    pub bar0: MappedBar,
}

impl std::fmt::Debug for EmberSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmberSession")
            .field("bdf", &self.device.bdf())
            .finish_non_exhaustive()
    }
}

impl EmberSession {
    /// Connect to ember and obtain BAR0 access for `bdf`.
    ///
    /// # Errors
    ///
    /// Returns `DriverError` if ember is unreachable, the BDF is not
    /// held by ember, or BAR0 mapping fails.
    pub fn connect(bdf: &str) -> DriverResult<Self> {
        let socket_path = default_socket();
        let stream = UnixStream::connect(&socket_path).map_err(|e| {
            DriverError::DeviceNotFound(Cow::Owned(format!("ember socket {socket_path}: {e}")))
        })?;
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .map_err(|e| DriverError::DeviceNotFound(Cow::Owned(e.to_string())))?;

        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "ember.vfio_fds",
            "params": { "bdf": bdf },
            "id": 1,
        });
        let payload = format!("{req}\n");
        std::io::Write::write_all(&mut &stream, payload.as_bytes())
            .map_err(|e| DriverError::DeviceNotFound(Cow::Owned(format!("ember send: {e}"))))?;

        let mut buf = [0u8; MAX_RESPONSE];
        let (n, fds) = recv_with_fds(&stream, &mut buf)?;

        let resp: serde_json::Value = serde_json::from_slice(&buf[..n]).map_err(|e| {
            DriverError::DeviceNotFound(Cow::Owned(format!("ember response parse: {e}")))
        })?;

        if let Some(err) = resp.get("error") {
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown ember error");
            return Err(DriverError::DeviceNotFound(Cow::Owned(format!(
                "ember error: {msg}"
            ))));
        }

        let result = resp
            .get("result")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let backend = result
            .get("backend")
            .and_then(|b| b.as_str())
            .unwrap_or("legacy");

        let received = match backend {
            "iommufd" => {
                if fds.len() < 2 {
                    return Err(DriverError::DeviceNotFound(Cow::Owned(format!(
                        "ember: expected 2 fds for iommufd, got {}",
                        fds.len()
                    ))));
                }
                let ioas_id = result.get("ioas_id").and_then(|v| v.as_u64()).ok_or(
                    DriverError::DeviceNotFound(Cow::Borrowed(
                        "ember: iommufd response missing ioas_id",
                    )),
                )? as u32;
                let mut it = fds.into_iter();
                ReceivedVfioFds::Iommufd {
                    iommufd: it.next().ok_or(DriverError::DeviceNotFound(Cow::Borrowed(
                        "ember: iommufd fd missing from SCM_RIGHTS",
                    )))?,
                    device: it.next().ok_or(DriverError::DeviceNotFound(Cow::Borrowed(
                        "ember: device fd missing from SCM_RIGHTS",
                    )))?,
                    ioas_id,
                }
            }
            _ => {
                if fds.len() < 3 {
                    return Err(DriverError::DeviceNotFound(Cow::Owned(format!(
                        "ember: expected 3 fds for legacy, got {}",
                        fds.len()
                    ))));
                }
                let mut it = fds.into_iter();
                ReceivedVfioFds::Legacy {
                    container: it.next().ok_or(DriverError::DeviceNotFound(Cow::Borrowed(
                        "ember: container fd missing from SCM_RIGHTS",
                    )))?,
                    group: it.next().ok_or(DriverError::DeviceNotFound(Cow::Borrowed(
                        "ember: group fd missing from SCM_RIGHTS",
                    )))?,
                    device: it.next().ok_or(DriverError::DeviceNotFound(Cow::Borrowed(
                        "ember: device fd missing from SCM_RIGHTS",
                    )))?,
                }
            }
        };

        let device = VfioDevice::from_received(bdf, received)?;
        let bar0 = device.map_bar(0)?;

        Ok(Self { device, bar0 })
    }
}

fn recv_with_fds(sock: impl AsFd, buf: &mut [u8]) -> DriverResult<(usize, Vec<OwnedFd>)> {
    const MAX_SCM_FDS: usize = 3;

    toadstool_hw_safe::recv_with_fds(sock, buf, MAX_SCM_FDS)
        .map_err(|e| DriverError::DeviceNotFound(Cow::Owned(format!("ember recvmsg: {e}"))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_fails_gracefully_when_no_ember() {
        // SAFETY: single-threaded test; no other thread reads this env var concurrently.
        unsafe {
            std::env::set_var("TOADSTOOL_EMBER_SOCKET", "/tmp/nonexistent-ember-test.sock");
        }
        let result = EmberSession::connect("0000:99:00.0");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("ember") || err.contains("socket") || err.contains("No such file"),
            "unexpected error: {err}"
        );
    }
}
