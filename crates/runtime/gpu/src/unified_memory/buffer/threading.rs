// SPDX-License-Identifier: AGPL-3.0-only
//! [`Send`] and [`Sync`] implementations for [`super::UnifiedBuffer`].

use super::UnifiedBuffer;

// SAFETY: Send implementation is safe because:
// - All interior data structures are thread-safe:
//   - Arc<T> is Send when T: Send (all our Arc types are Send)
//   - RwLock<T> is Send when T: Send (SyncState is Send)
//   - DashMap is thread-safe and Send
//   - AtomicU64 is Send
// - Raw pointers (cpu_ptr, device_ptr) are only accessed through safe API methods
// - The safe API enforces proper synchronization:
//   - Mutable operations require &mut self (exclusive access)
//   - Immutable operations use &self with interior mutability (RwLock, DashMap)
// - Moving UnifiedBuffer between threads doesn't invalidate the underlying memory
//   (unified memory is allocated by backend and remains valid across threads)
// - No thread-local state that would be invalidated by moving
unsafe impl Send for UnifiedBuffer {}

// SAFETY: Sync implementation is safe because:
// - All interior data structures are thread-safe and Sync:
//   - Arc<T> is Sync when T: Sync + Send (all our Arc types meet this)
//   - RwLock<T> is Sync when T: Send (SyncState is Send)
//   - DashMap is thread-safe and Sync
//   - AtomicU64 is Sync
// - Concurrent access patterns are safe:
//   - Multiple &self references can coexist (read-only operations)
//   - Mutable operations require &mut self (exclusive access enforced by borrow checker)
//   - Interior mutability (sync_state, allocations, metrics) uses proper synchronization
// - Raw pointers (cpu_ptr, device_ptr) are only accessed through safe API:
//   - Read operations use &self and validate before access
//   - Write operations use &mut self (exclusive access)
//   - Slice creation is bounds-checked and validated
// - No data races: all shared mutable state is protected by RwLock or atomic operations
// - The underlying unified memory is safe for concurrent reads (backend guarantees this)
unsafe impl Sync for UnifiedBuffer {}
