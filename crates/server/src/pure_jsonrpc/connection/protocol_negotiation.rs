// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ecoPrimals Contributors

//! G65 Protocol Negotiation (Phase 3 Cephalization).
//!
//! Enables automatic protocol selection between JSON-RPC and tarpc at
//! connection time on a **single socket**, replacing the C2 dual-socket
//! pattern (`.sock` + `.tarpc.sock`).
//!
//! ## Wire Protocol
//!
//! ```text
//! Client → Server: "PROTOCOLS: tarpc,jsonrpc\n"
//! Server → Client: "PROTOCOL: tarpc\n"
//! [Connection proceeds in selected protocol]
//! ```
//!
//! ## Backward Compatibility
//!
//! If the client does not send a `PROTOCOLS:` line within 100 ms, the server
//! assumes JSON-RPC. Existing clients work with **zero changes**.
//!
//! ## Reference
//!
//! Convergent evolution from squirrel (origin, 432 lines) and nestGate
//! (refined byte-by-byte line read). See
//! `wateringHole/specs/PROTOCOL_NEGOTIATION_SPEC.md`.

use super::ipc_protocol::IpcProtocol;
use tokio::io::{AsyncRead, AsyncReadExt};

/// G65 negotiation timeout — legacy clients that never send a negotiation
/// line are assumed to speak JSON-RPC after this window. The server reads
/// the first byte synchronously; this constant documents the spec value.
#[expect(
    dead_code,
    reason = "documents the G65 spec; used by future client-side API"
)]
pub(crate) const NEGOTIATION_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);

/// Maximum length of a `PROTOCOLS:` line before the server rejects it.
const MAX_NEGOTIATION_LINE_LEN: usize = 256;

/// Errors arising during G65 protocol negotiation.
#[derive(Debug, thiserror::Error)]
pub(crate) enum NegotiationError {
    #[error("Invalid protocol request: {0}")]
    InvalidRequest(String),
    #[error("No valid protocols in request")]
    NoValidProtocols,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "used by ProtocolResponse::from_wire (test/client-side)"
        )
    )]
    #[error("Invalid protocol response: {0}")]
    InvalidResponse(String),
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "used by ProtocolResponse::from_wire (test/client-side)"
        )
    )]
    #[error("Unknown protocol: {0}")]
    UnknownProtocol(String),
    #[error("G65 negotiation line exceeds {MAX_NEGOTIATION_LINE_LEN} bytes")]
    LineTooLong,
    #[error("Invalid UTF-8 in G65 line: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("I/O error during G65 negotiation: {0}")]
    Io(#[from] std::io::Error),
}

/// G65 protocol negotiation request from a client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProtocolRequest {
    /// Protocols supported by the client (in preference order).
    pub supported: Vec<IpcProtocol>,
}

impl ProtocolRequest {
    /// Parse from wire format (server-side: incoming request).
    ///
    /// # Errors
    ///
    /// Returns an error if the line has no `PROTOCOLS:` prefix or contains
    /// no valid protocol names.
    pub fn from_wire(line: &str) -> Result<Self, NegotiationError> {
        let line = line.trim();
        let protocols_str = line
            .strip_prefix("PROTOCOLS: ")
            .ok_or_else(|| NegotiationError::InvalidRequest(line.to_string()))?;

        let mut supported = Vec::new();
        for name in protocols_str.split(',') {
            if let Some(proto) = IpcProtocol::from_str(name.trim()) {
                supported.push(proto);
            }
        }

        if supported.is_empty() {
            return Err(NegotiationError::NoValidProtocols);
        }

        Ok(Self { supported })
    }
}

#[cfg(test)]
impl ProtocolRequest {
    #[must_use]
    pub const fn new(supported: Vec<IpcProtocol>) -> Self {
        Self { supported }
    }

    #[must_use]
    pub fn all_supported() -> Self {
        Self {
            supported: IpcProtocol::supported(),
        }
    }

    /// Serialize to wire format: `"PROTOCOLS: tarpc,jsonrpc\n"`.
    #[must_use]
    pub fn to_wire(&self) -> String {
        let names: Vec<&str> = self
            .supported
            .iter()
            .copied()
            .map(IpcProtocol::negotiation_name)
            .collect();
        format!("PROTOCOLS: {}\n", names.join(","))
    }
}

/// G65 protocol negotiation response from the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProtocolResponse {
    pub selected: IpcProtocol,
}

impl ProtocolResponse {
    #[must_use]
    pub const fn new(selected: IpcProtocol) -> Self {
        Self { selected }
    }

    /// Serialize to wire format: `"PROTOCOL: tarpc\n"`.
    #[must_use]
    pub fn to_wire(&self) -> String {
        format!("PROTOCOL: {}\n", self.selected.negotiation_name())
    }

    /// Parse from wire format (client-side: incoming response).
    ///
    /// # Errors
    ///
    /// Returns an error if the line has no `PROTOCOL:` prefix or the name
    /// is unrecognised.
    #[cfg(test)]
    pub fn from_wire(line: &str) -> Result<Self, NegotiationError> {
        let line = line.trim();
        let proto_name = line
            .strip_prefix("PROTOCOL: ")
            .ok_or_else(|| NegotiationError::InvalidResponse(line.to_string()))?;
        let selected = IpcProtocol::from_str(proto_name)
            .ok_or_else(|| NegotiationError::UnknownProtocol(proto_name.to_string()))?;
        Ok(Self { selected })
    }
}

/// Select the best protocol from the client's preference list.
///
/// Picks the first client-preferred protocol that the server also supports.
/// Falls back to `IpcProtocol::JsonRpc` if no intersection exists.
#[must_use]
pub fn select_protocol(
    client_supported: &[IpcProtocol],
    server_supported: &[IpcProtocol],
) -> IpcProtocol {
    for proto in client_supported {
        if server_supported.contains(proto) {
            return *proto;
        }
    }
    IpcProtocol::JsonRpc
}

/// Client-side G65 negotiation: send supported protocols, read server's selection.
///
/// # Errors
///
/// Returns an error if writing the request or reading the response fails.
#[cfg(test)]
pub(crate) async fn negotiate_client<T>(
    transport: &mut T,
    supported: Vec<IpcProtocol>,
) -> Result<IpcProtocol, NegotiationError>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    use tokio::io::{AsyncWriteExt, BufReader};

    let request = ProtocolRequest::new(supported);
    let wire = request.to_wire();

    tracing::debug!("G65 client sending: {:?}", request);
    transport.write_all(wire.as_bytes()).await?;
    transport.flush().await?;

    let mut reader = BufReader::new(transport);
    let mut response_line = String::new();
    tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut response_line).await?;

    let response = ProtocolResponse::from_wire(&response_line)?;

    tracing::info!("G65 client negotiated: {}", response.selected);
    Ok(response.selected)
}

/// Read a single `\n`-terminated line byte-by-byte.
///
/// Unlike `BufReader::read_line`, this does not buffer past the newline,
/// so subsequent reads on the underlying stream see the protocol payload
/// immediately after the negotiation line.
///
/// # Errors
///
/// Returns an error on I/O failure or if the line exceeds
/// `MAX_NEGOTIATION_LINE_LEN` bytes.
pub(crate) async fn read_negotiation_line<T: AsyncRead + Unpin>(
    stream: &mut T,
) -> Result<String, NegotiationError> {
    let mut buf = Vec::with_capacity(64);
    let mut byte = [0u8; 1];

    loop {
        let n = stream.read(&mut byte).await?;
        if n == 0 {
            break;
        }
        buf.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
        if buf.len() > MAX_NEGOTIATION_LINE_LEN {
            return Err(NegotiationError::LineTooLong);
        }
    }

    Ok(String::from_utf8(buf)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncWriteExt, BufReader};

    // ── Wire format round-trips ──────────────────────────────────────

    #[test]
    fn request_wire_roundtrip_jsonrpc() {
        let req = ProtocolRequest::new(vec![IpcProtocol::JsonRpc]);
        assert_eq!(req.to_wire(), "PROTOCOLS: jsonrpc\n");
        let parsed = ProtocolRequest::from_wire(&req.to_wire()).unwrap();
        assert_eq!(parsed, req);
    }

    #[test]
    fn request_wire_roundtrip_both() {
        let req = ProtocolRequest::new(vec![IpcProtocol::Tarpc, IpcProtocol::JsonRpc]);
        assert_eq!(req.to_wire(), "PROTOCOLS: tarpc,jsonrpc\n");
        let parsed = ProtocolRequest::from_wire(&req.to_wire()).unwrap();
        assert_eq!(parsed, req);
    }

    #[test]
    fn response_wire_roundtrip() {
        for proto in IpcProtocol::supported() {
            let resp = ProtocolResponse::new(proto);
            let parsed = ProtocolResponse::from_wire(&resp.to_wire()).unwrap();
            assert_eq!(parsed, resp);
        }
    }

    // ── Parsing edge cases ───────────────────────────────────────────

    #[test]
    fn request_from_wire_invalid_prefix() {
        let err = ProtocolRequest::from_wire("NOTPROTOCOLS: jsonrpc\n").unwrap_err();
        assert!(err.to_string().contains("Invalid protocol request"));
    }

    #[test]
    fn request_from_wire_no_valid_protocols() {
        let err = ProtocolRequest::from_wire("PROTOCOLS: quic,grpc\n").unwrap_err();
        assert!(err.to_string().contains("No valid protocols"));
    }

    #[test]
    fn response_from_wire_invalid_prefix() {
        let err = ProtocolResponse::from_wire("STATUS: ok\n").unwrap_err();
        assert!(err.to_string().contains("Invalid protocol response"));
    }

    #[test]
    fn response_from_wire_unknown_protocol() {
        let err = ProtocolResponse::from_wire("PROTOCOL: quic\n").unwrap_err();
        assert!(err.to_string().contains("Unknown protocol"));
    }

    #[test]
    fn request_all_supported_includes_both() {
        let req = ProtocolRequest::all_supported();
        assert!(req.supported.contains(&IpcProtocol::JsonRpc));
        assert!(req.supported.contains(&IpcProtocol::Tarpc));
    }

    // ── select_protocol ──────────────────────────────────────────────

    #[test]
    fn select_protocol_client_preference_wins() {
        let client = vec![IpcProtocol::Tarpc, IpcProtocol::JsonRpc];
        let server = vec![IpcProtocol::Tarpc, IpcProtocol::JsonRpc];
        assert_eq!(select_protocol(&client, &server), IpcProtocol::Tarpc);
    }

    #[test]
    fn select_protocol_server_only_jsonrpc() {
        let client = vec![IpcProtocol::Tarpc, IpcProtocol::JsonRpc];
        let server = vec![IpcProtocol::JsonRpc];
        assert_eq!(select_protocol(&client, &server), IpcProtocol::JsonRpc);
    }

    #[test]
    fn select_protocol_no_common_falls_back() {
        let client = vec![IpcProtocol::Tarpc];
        let server = vec![IpcProtocol::JsonRpc];
        assert_eq!(select_protocol(&client, &server), IpcProtocol::JsonRpc);
    }

    // ── read_negotiation_line ────────────────────────────────────────

    #[tokio::test]
    async fn read_negotiation_line_reads_exactly_one_line() {
        let data = b"PROTOCOLS: tarpc,jsonrpc\nREMAINING DATA";
        let mut cursor = std::io::Cursor::new(&data[..]);
        let line = read_negotiation_line(&mut cursor).await.unwrap();
        assert_eq!(line, "PROTOCOLS: tarpc,jsonrpc\n");
        // "PROTOCOLS: tarpc,jsonrpc\n" = 25 bytes
        assert_eq!(cursor.position(), 25);
    }

    #[tokio::test]
    async fn read_negotiation_line_rejects_overlong() {
        let long = "P".repeat(MAX_NEGOTIATION_LINE_LEN + 10) + "\n";
        let mut cursor = std::io::Cursor::new(long.as_bytes());
        let err = read_negotiation_line(&mut cursor).await.unwrap_err();
        assert!(err.to_string().contains("exceeds"));
    }

    #[tokio::test]
    async fn read_negotiation_line_eof_before_newline() {
        let data = b"PROTOCOLS: tarpc";
        let mut cursor = std::io::Cursor::new(&data[..]);
        let line = read_negotiation_line(&mut cursor).await.unwrap();
        assert_eq!(line, "PROTOCOLS: tarpc");
    }

    // ── Full duplex negotiation ──────────────────────────────────────

    #[tokio::test]
    async fn negotiate_client_server_duplex_jsonrpc() {
        let (mut client, mut server) = tokio::io::duplex(4096);

        let server_supported = IpcProtocol::supported();
        let server_task = tokio::spawn(async move {
            let mut line = String::new();
            let mut reader = BufReader::new(&mut server);
            tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut line)
                .await
                .unwrap();
            let req = ProtocolRequest::from_wire(&line).unwrap();
            let selected = select_protocol(&req.supported, &server_supported);
            let resp = ProtocolResponse::new(selected);
            reader
                .get_mut()
                .write_all(resp.to_wire().as_bytes())
                .await
                .unwrap();
            reader.get_mut().flush().await.unwrap();
            selected
        });

        let selected = negotiate_client(&mut client, vec![IpcProtocol::JsonRpc])
            .await
            .unwrap();
        assert_eq!(selected, IpcProtocol::JsonRpc);

        let server_selected = server_task.await.unwrap();
        assert_eq!(server_selected, IpcProtocol::JsonRpc);
    }

    #[tokio::test]
    async fn negotiate_client_server_duplex_tarpc_preferred() {
        let (mut client, mut server) = tokio::io::duplex(4096);

        let server_supported = IpcProtocol::supported();
        let server_task = tokio::spawn(async move {
            let mut line = String::new();
            let mut reader = BufReader::new(&mut server);
            tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut line)
                .await
                .unwrap();
            let req = ProtocolRequest::from_wire(&line).unwrap();
            let selected = select_protocol(&req.supported, &server_supported);
            let resp = ProtocolResponse::new(selected);
            reader
                .get_mut()
                .write_all(resp.to_wire().as_bytes())
                .await
                .unwrap();
            reader.get_mut().flush().await.unwrap();
            selected
        });

        let selected =
            negotiate_client(&mut client, vec![IpcProtocol::Tarpc, IpcProtocol::JsonRpc])
                .await
                .unwrap();
        assert_eq!(selected, IpcProtocol::Tarpc);

        let server_selected = server_task.await.unwrap();
        assert_eq!(server_selected, IpcProtocol::Tarpc);
    }
}
