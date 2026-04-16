// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)]
// `unsafe impl Send/Sync` — `NonNull`/raw GPU pointers are not `Send`/`Sync` by default (rust-lang/rust#48214)
//! [`Send`] and [`Sync`] for [`super::UnifiedBuffer`].
//!
//! Rust does not auto-derive [`Send`]/[`Sync`] for [`std::ptr::NonNull`] or raw pointers to
//! arbitrary memory (issue #48214). [`UnifiedBuffer`] stores CPU and device addresses plus
//! `Arc`/`RwLock` state; the manual impls assert the same invariants the GPU backends rely on:
//! unified memory remains valid for the buffer lifetime, and coordinated access goes through
//! the type’s API and the [`UnifiedMemoryBackend`](crate::unified_memory::backend::UnifiedMemoryBackend) protocol.

use super::UnifiedBuffer;

// SAFETY (`Send`):
// - `UnifiedBuffer` may be moved to another thread if all its fields are safe to move together.
// - Non-raw fields (`Arc`, `RwLock`, atomics, maps, etc.) are `Send` by construction.
// - `cpu_ptr` / `device_ptr` are backend-issued addresses for one unified allocation; they must
//   denote memory that remains valid for this buffer until drop/free regardless of which thread
//   owns the `UnifiedBuffer`. Constructors and backend code must only store pointers allowed by
//   the GPU/CPU API for cross-thread moves of the owning handle.
// - Moving the struct does not duplicate Rust-level ownership of the allocation; the buffer’s
//   RAII (`allocation`, drop path) remains responsible for freeing exactly once.
// - Aliasing: other threads or GPUs may hold related handles; cross-device access rules are
//   enforced by backend sync methods and buffer API, not by `Send` alone.
unsafe impl Send for UnifiedBuffer {}

// SAFETY (`Sync`):
// - Shared `&UnifiedBuffer` access must not allow unsynchronized data races on any field.
// - Interior mutability (`RwLock`, atomics) serializes concurrent access to shared metadata;
//   raw pointers are only dereferenced through methods that enforce validation and locking
//   discipline (e.g. CPU slice helpers after `validate_cpu_ptr`).
// - Callers must still respect GPU/CPU coherence: `Sync` does not make concurrent GPU writes
//   safe for CPU readers without explicit synchronization (`sync_device_to_cpu`, etc.).
// - Same pointer validity and backend-lifetime guarantees as `Send`: `&UnifiedBuffer` may be
//   shared across threads only if reading address fields and using the public API is allowed
//   concurrently under the backend contract.
unsafe impl Sync for UnifiedBuffer {}
