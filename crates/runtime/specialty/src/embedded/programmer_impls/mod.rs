// SPDX-License-Identifier: AGPL-3.0-or-later
//! Trait implementations for embedded programmers
//!
//! ISP / ICSP protocol sequencing and chip validation are implemented in pure Rust; any operation
//! that would require talking to a USB/serial/parallel adapter returns
//! [`EmbeddedProgrammerError::TransportNotConfigured`] after validation succeeds.
//!
//! See DEBT.md `D-EMBEDDED-PROGRAMMER` for hardware transport tracking.

use std::collections::HashMap;

use crate::SpecialtyRuntimeError;
use toadstool::ToadStoolError;

use super::errors::EmbeddedProgrammerError;

fn programmer_err(e: EmbeddedProgrammerError) -> ToadStoolError {
    SpecialtyRuntimeError::from(e).into()
}

fn get_str<'a>(
    params: &'a HashMap<String, String>,
    key: &str,
) -> Result<&'a str, EmbeddedProgrammerError> {
    params.get(key).map(String::as_str).ok_or_else(|| {
        EmbeddedProgrammerError::ConfigurationInvalid {
            detail: format!("missing connection parameter `{key}`"),
        }
    })
}

fn parse_u64(params: &HashMap<String, String>, key: &str) -> Result<u64, EmbeddedProgrammerError> {
    let s = get_str(params, key)?;
    s.parse::<u64>()
        .map_err(|_| EmbeddedProgrammerError::ConfigurationInvalid {
            detail: format!("`{key}` must be a decimal integer (got {s:?})"),
        })
}

fn parse_u32(params: &HashMap<String, String>, key: &str) -> Result<u32, EmbeddedProgrammerError> {
    let s = get_str(params, key)?;
    s.parse::<u32>()
        .map_err(|_| EmbeddedProgrammerError::ConfigurationInvalid {
            detail: format!("`{key}` must be a decimal integer (got {s:?})"),
        })
}

fn parse_signature_hex(s: &str) -> Result<u32, EmbeddedProgrammerError> {
    let t = s.trim();
    let t = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .unwrap_or(t);
    u32::from_str_radix(t, 16)
        .map(|v| v & 0xFF_FFFF)
        .map_err(|_| EmbeddedProgrammerError::ConfigurationInvalid {
            detail: format!("invalid hex signature {s:?}"),
        })
}

mod eprom;
mod generic;
mod init;

#[cfg(test)]
mod tests;
