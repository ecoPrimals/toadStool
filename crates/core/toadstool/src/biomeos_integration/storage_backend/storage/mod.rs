// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unix-socket storage backend (constructors + trait impl).

mod construct;
mod ops;
#[cfg(test)]
mod tests;

pub use construct::SocketStorageBackend;
