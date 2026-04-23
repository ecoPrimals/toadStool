// SPDX-License-Identifier: AGPL-3.0-or-later
//! Load and normalize the BTSP family seed for BearDog JSON-line handshake.

use std::path::PathBuf;

use base64::Engine;
use thiserror::Error;

/// Failure loading or decoding the family seed for BTSP.
#[derive(Debug, Error)]
pub enum BtspFamilySeedError {
    /// Seed file could not be read.
    #[error("BTSP family seed file read failed: {0}")]
    Io(#[from] std::io::Error),

    /// Hex decoding failed.
    #[error("BTSP family seed hex decode failed: {0}")]
    InvalidHex(String),

    /// Base64 decoding failed.
    #[error("BTSP family seed base64 decode failed: {0}")]
    Base64Decode(#[from] base64::DecodeError),

    /// No seed found in environment or standard paths.
    #[error(
        "BTSP family seed not found (set FAMILY_SEED / BEARDOG_FAMILY_SEED or install .family.seed)"
    )]
    NotFound,
}

fn hex_nibble(b: u8) -> Result<u8, BtspFamilySeedError> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(BtspFamilySeedError::InvalidHex(format!(
            "invalid hex digit: {b:#02x}"
        ))),
    }
}

fn decode_hex_even(s: &str) -> Result<Vec<u8>, BtspFamilySeedError> {
    if s.len() % 2 != 0 {
        return Err(BtspFamilySeedError::InvalidHex(
            "hex length must be even".into(),
        ));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for chunk in s.as_bytes().chunks(2) {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn decode_seed_text(text: &str) -> Result<Vec<u8>, BtspFamilySeedError> {
    let t = text.trim();
    if t.is_empty() {
        return Err(BtspFamilySeedError::NotFound);
    }

    let hex_trim = t.strip_prefix("0x").unwrap_or(t);
    if !hex_trim.is_empty()
        && hex_trim.chars().all(|c| c.is_ascii_hexdigit())
        && hex_trim.len() % 2 == 0
    {
        return decode_hex_even(hex_trim);
    }

    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(t) {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }

    if let Ok(bytes) = base64::engine::general_purpose::STANDARD_NO_PAD.decode(t) {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }

    Ok(t.as_bytes().to_vec())
}

fn read_seed_file(path: &std::path::Path) -> Option<Vec<u8>> {
    let raw = std::fs::read(path).ok()?;
    let text = std::str::from_utf8(&raw).ok()?;
    decode_seed_text(text).ok()
}

fn socket_dir_family_seed_path() -> Option<PathBuf> {
    let dir = std::env::var("BIOMEOS_SOCKET_DIR").ok()?;
    if dir.is_empty() {
        return None;
    }
    Some(PathBuf::from(dir).join(".family.seed"))
}

fn config_family_seed_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config/biomeos/.family.seed");
    }
    PathBuf::from(".config/biomeos/.family.seed")
}

/// Load family seed for BTSP handshake.
///
/// Resolution order:
/// 1. `FAMILY_SEED` env var (may be base64 or hex)
/// 2. `BEARDOG_FAMILY_SEED` env var
/// 3. File: `$BIOMEOS_SOCKET_DIR/.family.seed` or `~/.config/biomeos/.family.seed`
///
/// Returns the seed as a **standard** base64-encoded string (BearDog expects base64).
pub fn load_family_seed_for_btsp() -> Result<String, BtspFamilySeedError> {
    if let Ok(v) = std::env::var("FAMILY_SEED") {
        let bytes = decode_seed_text(&v)?;
        if bytes.is_empty() {
            return Err(BtspFamilySeedError::NotFound);
        }
        return Ok(base64::engine::general_purpose::STANDARD.encode(bytes));
    }

    if let Ok(v) = std::env::var("BEARDOG_FAMILY_SEED") {
        let bytes = decode_seed_text(&v)?;
        if bytes.is_empty() {
            return Err(BtspFamilySeedError::NotFound);
        }
        return Ok(base64::engine::general_purpose::STANDARD.encode(bytes));
    }

    if let Some(path) = socket_dir_family_seed_path()
        && let Some(bytes) = read_seed_file(&path)
        && !bytes.is_empty()
    {
        return Ok(base64::engine::general_purpose::STANDARD.encode(bytes));
    }

    let cfg = config_family_seed_path();
    if let Some(bytes) = read_seed_file(&cfg)
        && !bytes.is_empty()
    {
        return Ok(base64::engine::general_purpose::STANDARD.encode(bytes));
    }

    Err(BtspFamilySeedError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn decode_round_trip_base64() {
        let raw = b"exactly-thirty-two-byte-seed!!!!";
        let b64 = base64::engine::general_purpose::STANDARD.encode(raw);
        let decoded = decode_seed_text(&b64).expect("decode");
        assert_eq!(decoded, raw);
    }

    #[test]
    fn decode_hex_seed() {
        let hex = "deadbeef";
        let decoded = decode_seed_text(hex).expect("hex");
        assert_eq!(decoded, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn load_from_file_tmp() {
        let mut f = tempfile::NamedTempFile::new().expect("tmp");
        let seed = "plain-ascii-seed-bytes!!!!!!";
        writeln!(f, "{seed}").unwrap();
        let bytes = read_seed_file(f.path()).expect("read");
        assert_eq!(bytes, seed.as_bytes());
    }

    #[test]
    fn load_family_seed_prefers_env_family_seed() {
        temp_env::with_vars(
            [
                ("FAMILY_SEED", Some("deadbeef")),
                ("BEARDOG_FAMILY_SEED", Some("010203")),
            ],
            || {
                let b64 = load_family_seed_for_btsp().expect("seed");
                assert_eq!(
                    b64,
                    base64::engine::general_purpose::STANDARD.encode([0xde, 0xad, 0xbe, 0xef])
                );
            },
        );
    }
}
