// SPDX-License-Identifier: AGPL-3.0-or-later
//! BTSP (biomeOS Transport Security Protocol) implementation.
//!
//! Local analogue of TLS 1.3 for intra-machine primal IPC.
//! Production connections authenticate via BTSP first; plaintext is
//! negotiated after secure nucleation, not default.
//!
//! Reference: `ecoPrimals/infra/wateringHole/BTSP_PROTOCOL_STANDARD.md` v1.0.0
//!
//! ## Phases
//!
//! 1. **Socket naming** (done) — `compute.sock` / `compute-{fid}.sock`
//! 2. **Handshake** — X25519 + HKDF-SHA256 + HMAC challenge-response
//! 3. **Cipher negotiation** — ChaCha20-Poly1305, HMAC-plain, or NULL
//! 4. **Framing** — length-prefixed (4-byte BE u32), max 16 MiB

pub mod client;
pub mod family_seed;
pub mod framing;
pub mod json_line;
pub mod negotiate;
pub mod phase3;
pub mod relay;
pub mod server;
pub mod types;

pub use client::{BtspClient, BtspSession};
pub use family_seed::{BtspFamilySeedError, load_family_seed_for_btsp};
pub use framing::{BtspFrameReader, BtspFrameWriter, PrependByte};
pub use json_line::{
    BtspJsonLineError, BtspSessionInfo, line_looks_like_btsp_client_hello,
    read_full_line_after_first_byte, read_line_suffix, resolve_security_socket_path,
};
pub use negotiate::{NegotiateOutcome, try_handle_negotiate};
pub use phase3::{NegotiateParams, NegotiateResponse, Phase3Error, Phase3SessionKeys};
pub use relay::relay_json_line_handshake;
pub use server::BtspServer;
pub use types::{
    BtspCipher, ChallengeResponse, ClientHello, HandshakeComplete, HandshakeError, ServerHello,
    SessionKeys,
};
