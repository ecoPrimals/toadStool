// SPDX-License-Identifier: AGPL-3.0-or-later
//! Crypto provider errors

use crate::{ConfigError, ToadStoolError};

/// Errors from crypto provider operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum CryptoError {
    /// No crypto provider registered (sentinel / bootstrap state)
    #[error("No crypto provider registered: {0}")]
    NoProviderRegistered(String),
}

impl From<CryptoError> for ToadStoolError {
    fn from(err: CryptoError) -> Self {
        match err {
            CryptoError::NoProviderRegistered(reason) => {
                ConfigError::ValidationError { reason }.into()
            }
        }
    }
}
