// SPDX-License-Identifier: AGPL-3.0-or-later
//! Simple message handler implementation for protocol messages

use crate::types::{MessageHandler, ProtocolError, ProtocolMessage, ProtocolResult};

/// Simple message handler implementation
pub struct SimpleMessageHandler<F>
where
    F: Fn(ProtocolMessage) -> Result<Option<ProtocolMessage>, ProtocolError> + Send + Sync,
{
    handler_fn: F,
}

impl<F> SimpleMessageHandler<F>
where
    F: Fn(ProtocolMessage) -> Result<Option<ProtocolMessage>, ProtocolError> + Send + Sync,
{
    pub fn new(handler_fn: F) -> Self {
        Self { handler_fn }
    }
}

impl<F> MessageHandler for SimpleMessageHandler<F>
where
    F: Fn(ProtocolMessage) -> Result<Option<ProtocolMessage>, ProtocolError> + Send + Sync,
{
    fn handle_message(&self, message: ProtocolMessage) -> ProtocolResult<Option<ProtocolMessage>> {
        (self.handler_fn)(message)
    }
}
