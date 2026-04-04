// SPDX-License-Identifier: AGPL-3.0-only
//! Protocol client implementation for service communication
//!
//! Domain modules:
//! - `discovery` — Service discovery and registration
//! - `health` — Health monitoring for registered services
//! - `routing` — Service and endpoint selection
//! - `handler` — Simple message handler implementation

mod discovery;
mod handler;
mod health;
mod protocol_client;
mod routing;

#[cfg(test)]
mod tests;

pub use handler::SimpleMessageHandler;
pub use protocol_client::ProtocolClient;
