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

// SAFETY: All fields other than raw addresses are `Send`/`Sync` (`Arc`, `RwLock`, `AtomicU64`,
// `HashMap` metadata). `cpu_ptr`/`device_ptr` refer to backend-owned unified memory valid for
// the buffer’s lifetime; they are only produced/consumed through this module’s safe methods.
// Moving or sharing `UnifiedBuffer` across threads does not introduce data races beyond what
// the backend and `sync_state` already synchronize.
unsafe impl Send for UnifiedBuffer {}

// SAFETY: Concurrent `&UnifiedBuffer` access uses `RwLock`/`Arc` for shared state; exclusive
// mutation uses `&mut self`. Raw pointers follow the same backend lifetime and API contracts
// as `Send`.
unsafe impl Sync for UnifiedBuffer {}
