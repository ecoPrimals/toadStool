// SPDX-License-Identifier: AGPL-3.0-or-later
//! BTSP (biomeOS Transport Security Protocol) implementation.
//!
//! Local analogue of TLS 1.3 for intra-machine primal IPC.
//! Production connections authenticate via BTSP first; plaintext is
//! negotiated after secure nucleation, not default.
//!
//! Reference: `wateringHole/BTSP_PROTOCOL_STANDARD.md` v1.0.0
//!
//! ## Phases
//!
//! 1. **Socket naming** (done) — `compute.sock` / `compute-{fid}.sock`
//! 2. **Handshake** — X25519 + HKDF-SHA256 + HMAC challenge-response
//! 3. **Cipher negotiation** — ChaCha20-Poly1305, HMAC-plain, or NULL
//! 4. **Framing** — length-prefixed (4-byte BE u32), max 16 MiB

pub mod client;
pub mod framing;
pub mod server;
pub mod types;

pub use client::{BtspClient, BtspSession};
pub use framing::{BtspFrameReader, BtspFrameWriter};
pub use server::BtspServer;
pub use types::{
    BtspCipher, ChallengeResponse, ClientHello, HandshakeComplete, HandshakeError, ServerHello,
    SessionKeys,
};
