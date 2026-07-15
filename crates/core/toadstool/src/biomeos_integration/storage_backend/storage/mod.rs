// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unix-socket storage backend (constructors + trait impl).

#[cfg(unix)]
mod construct;
#[cfg(unix)]
mod ops;
#[cfg(test)]
mod tests;

#[cfg(unix)]
pub use construct::SocketStorageBackend;
