// SPDX-License-Identifier: AGPL-3.0-or-later
//! Load and normalize the BTSP family seed for BearDog JSON-line handshake.

use std::path::PathBuf;

use thiserror::Error;

/// Failure loading the family seed for BTSP.
#[derive(Debug, Error)]
pub enum BtspFamilySeedError {
    /// Seed file could not be read.
    #[error("BTSP family seed file read failed: {0}")]
    Io(#[from] std::io::Error),

    /// No seed found in environment or standard paths.
    #[error(
        "BTSP family seed not found (set FAMILY_SEED / BEARDOG_FAMILY_SEED or install .family.seed)"
    )]
    NotFound,
}

fn read_seed_file_as_string(path: &std::path::Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
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
/// 1. `FAMILY_SEED` env var
/// 2. `BEARDOG_FAMILY_SEED` env var
/// 3. File: `$BIOMEOS_SOCKET_DIR/.family.seed` or `~/.config/biomeos/.family.seed`
///
/// Returns the seed as a **raw string** — passed directly to BearDog's
/// `btsp.session.create` without hex-decoding or base64 re-encoding.
/// BearDog owns the encoding interpretation.
pub fn load_family_seed_for_btsp() -> Result<String, BtspFamilySeedError> {
    if let Ok(v) = std::env::var("FAMILY_SEED") {
        let trimmed = v.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }
    if let Ok(v) = std::env::var("BEARDOG_FAMILY_SEED") {
        let trimmed = v.trim().to_string();
        if !trimmed.is_empty() {
            tracing::warn!("BEARDOG_FAMILY_SEED is deprecated — use FAMILY_SEED");
            return Ok(trimmed);
        }
    }

    if let Some(path) = socket_dir_family_seed_path() {
        if let Some(s) = read_seed_file_as_string(&path) {
            return Ok(s);
        }
    }

    let cfg = config_family_seed_path();
    if let Some(s) = read_seed_file_as_string(&cfg) {
        return Ok(s);
    }

    Err(BtspFamilySeedError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_from_file_returns_raw_content() {
        let mut f = tempfile::NamedTempFile::new().expect("tmp");
        let seed = "plain-ascii-seed-bytes!!!!!!";
        writeln!(f, "{seed}").unwrap();
        let raw = read_seed_file_as_string(f.path()).expect("read");
        assert_eq!(raw, seed);
    }

    #[test]
    fn load_family_seed_returns_raw_env_value() {
        temp_env::with_vars(
            [
                ("FAMILY_SEED", Some("my-raw-seed-string")),
                ("BEARDOG_FAMILY_SEED", Some("fallback")),
            ],
            || {
                let seed = load_family_seed_for_btsp().expect("seed");
                assert_eq!(seed, "my-raw-seed-string");
            },
        );
    }

    #[test]
    fn load_family_seed_falls_back_to_beardog_env() {
        temp_env::with_vars(
            [
                ("FAMILY_SEED", None::<&str>),
                ("BEARDOG_FAMILY_SEED", Some("beardog-seed")),
            ],
            || {
                let seed = load_family_seed_for_btsp().expect("seed");
                assert_eq!(seed, "beardog-seed");
            },
        );
    }
}
