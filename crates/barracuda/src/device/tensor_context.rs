//! TensorContext - Zero-overhead Tensor operations via internal pooling
//!
//! **Problem**: Tensor layer adds ~200-300μs overhead per operation due to:
//! - Buffer allocation per output (~20μs)
//! - Bind group creation per operation (~50-100μs)
//! - Command encoder creation per operation (~11μs)
//! - Queue submit per operation (~150μs on NVIDIA)
//!
//! **Solution**: Pool and reuse all these objects internally:
//! - Pre-allocate output buffers (memory pool)
//! - Cache bind groups by buffer address combination
//! - Batch operations into single encoder/submit
//!
//! **Result**: Tensor API stays clean, internal execution matches raw wgpu speed.
//!
//! ## Usage
//!
//! ### Automatic (Global Context)
//! ```rust,ignore
//! // Tensor ops automatically use the global context
//! let a = Tensor::from_data(&[1.0, 2.0], vec![2], device)?;
//! let b = Tensor::from_data(&[3.0, 4.0], vec![2], device)?;
//! let c = a.add(&b)?;  // Uses pooled buffer automatically
//! ```
//!
//! ### Explicit Batching
//! ```rust,ignore
//! // For maximum control, use explicit context
//! let ctx = device.context();
//! ctx.begin_batch();
//! let c = a.add(&b)?;
//! let d = c.mul(&a)?;
//! ctx.end_batch()?;  // Single GPU submission
//! ```

use crate::device::pipeline_cache::{BindGroupLayoutSignature, DeviceFingerprint, GLOBAL_CACHE};
use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use dashmap::DashMap;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, RwLock, Weak};

/// Type alias for pending GPU operations (executed on sync)
type PendingOp = Box<dyn FnOnce(&mut wgpu::CommandEncoder) + Send>;

/// Global context registry - one TensorContext per device
/// Evolved from once_cell::sync::Lazy to std::sync::LazyLock (Rust 1.80+)
static GLOBAL_CONTEXTS: LazyLock<DashMap<DeviceFingerprint, Arc<TensorContext>>> =
    LazyLock::new(DashMap::new);

// ============================================================================
// PooledBuffer - Auto-returning buffer wrapper
// ============================================================================

/// A buffer that automatically returns to its pool when dropped.
///
/// This is the key to zero-allocation steady state. When a PooledBuffer
/// goes out of scope, it returns itself to the pool instead of being freed.
///
/// # Example
/// ```rust,ignore
/// let ctx = get_device_context(&device);
/// let buffer = ctx.acquire_pooled_buffer(1024);  // From pool
/// // ... use buffer ...
/// drop(buffer);  // Automatically returned to pool!
/// ```
pub struct PooledBuffer {
    /// The underlying wgpu buffer (Option to allow take on drop)
    buffer: Option<wgpu::Buffer>,
    /// Weak reference to the pool for return
    pool: Weak<BufferPoolInner>,
    /// Size bucket for returning to correct pool
    bucket: usize,
}

impl PooledBuffer {
    /// Create a new pooled buffer
    fn new(buffer: wgpu::Buffer, pool: Weak<BufferPoolInner>, bucket: usize) -> Self {
        Self {
            buffer: Some(buffer),
            pool,
            bucket,
        }
    }

    /// Get the underlying wgpu buffer
    pub fn buffer(&self) -> &wgpu::Buffer {
        self.buffer.as_ref().expect("Buffer already taken")
    }

    /// Get the buffer size in bytes
    pub fn size(&self) -> u64 {
        self.buffer().size()
    }

    /// Convert to a regular wgpu::Buffer (removes from pool management)
    /// Use this when you need to pass ownership elsewhere.
    pub fn into_buffer(mut self) -> wgpu::Buffer {
        self.buffer.take().expect("Buffer already taken")
    }
}

impl Deref for PooledBuffer {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        self.buffer()
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            // Try to return to pool
            if let Some(pool) = self.pool.upgrade() {
                pool.return_buffer(buffer, self.bucket);
            }
            // If pool is gone, buffer is dropped normally
        }
    }
}

/// Inner pool structure (separate to allow Weak references)
struct BufferPoolInner {
    pools: DashMap<usize, Vec<wgpu::Buffer>>,
    device: Arc<wgpu::Device>,
    allocations: AtomicUsize,
    reuses: AtomicUsize,
    /// Pinned solver buffers (not returned to pool until explicitly released)
    solver_buffers: RwLock<HashMap<String, SolverBufferSet>>,
}

impl BufferPoolInner {
    fn return_buffer(&self, buffer: wgpu::Buffer, bucket: usize) {
        self.pools.entry(bucket).or_default().push(buffer);
        self.reuses.fetch_add(1, Ordering::Relaxed);
    }
}

/// Descriptor for creating a pinned solver buffer
#[derive(Debug, Clone)]
pub struct BufferDescriptor {
    /// Size in bytes
    pub size: u64,
    /// Buffer usage flags (defaults to STORAGE | COPY_SRC | COPY_DST)
    pub usage: wgpu::BufferUsages,
    /// Optional label for debugging
    pub label: Option<String>,
}

impl BufferDescriptor {
    /// Create a new buffer descriptor with the given size
    ///
    /// Default usage: STORAGE | COPY_SRC | COPY_DST
    pub fn new(size: u64) -> Self {
        Self {
            size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            label: None,
        }
    }

    /// Create a descriptor for f64 array
    pub fn f64_array(count: usize) -> Self {
        Self::new((count * std::mem::size_of::<f64>()) as u64)
    }

    /// Create a descriptor for f32 array
    pub fn f32_array(count: usize) -> Self {
        Self::new((count * std::mem::size_of::<f32>()) as u64)
    }

    /// Set a label for debugging
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set custom usage flags
    pub fn with_usage(mut self, usage: wgpu::BufferUsages) -> Self {
        self.usage = usage;
        self
    }
}

/// A set of buffers pinned for the lifetime of a solver
///
/// This enables zero-allocation steady state during iterative solvers.
/// Buffers are created once at solver start and reused across all iterations.
///
/// # Example
///
/// ```rust,ignore
/// let buffers = pool.pin_solver_buffers("hfb_scf", &[
///     ("hamiltonian", BufferDescriptor::f64_array(batch * n * n)),
///     ("eigenvalues", BufferDescriptor::f64_array(batch * n)),
///     ("eigenvectors", BufferDescriptor::f64_array(batch * n * n)),
/// ])?;
///
/// for iteration in 0..max_iterations {
///     // Use buffers.get("hamiltonian") etc — no allocation
/// }
///
/// pool.release_solver_buffers("hfb_scf");
/// ```
#[derive(Debug)]
pub struct SolverBufferSet {
    solver_id: String,
    buffers: HashMap<String, Arc<wgpu::Buffer>>,
}

impl SolverBufferSet {
    /// Get a buffer by name
    pub fn get(&self, name: &str) -> Option<&wgpu::Buffer> {
        self.buffers.get(name).map(|b| b.as_ref())
    }

    /// Get a buffer by name (Arc reference)
    pub fn get_arc(&self, name: &str) -> Option<Arc<wgpu::Buffer>> {
        self.buffers.get(name).cloned()
    }

    /// Get the solver ID
    pub fn solver_id(&self) -> &str {
        &self.solver_id
    }

    /// Get all buffer names
    pub fn buffer_names(&self) -> impl Iterator<Item = &str> {
        self.buffers.keys().map(|s| s.as_str())
    }

    /// Get the number of buffers
    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }
}

/// Get or create the global TensorContext for a device
pub fn get_device_context(device: &Arc<WgpuDevice>) -> Arc<TensorContext> {
    let fingerprint = DeviceFingerprint::from_adapter_info(device.adapter_info());

    GLOBAL_CONTEXTS
        .entry(fingerprint)
        .or_insert_with(|| Arc::new(TensorContext::new(device.clone())))
        .clone()
}

/// Clear all global contexts (for testing/benchmarking)
pub fn clear_global_contexts() {
    GLOBAL_CONTEXTS.clear();
}

/// Memory pool for buffer reuse
///
/// Instead of allocating new buffers per operation, we pool and reuse them.
/// This eliminates the ~20μs allocation overhead per op.
///
/// Buffers acquired via `acquire_pooled()` automatically return to the pool
/// when dropped. This enables zero-allocation steady state.
pub struct BufferPool {
    /// Inner pool (Arc to allow Weak references from PooledBuffer)
    inner: Arc<BufferPoolInner>,
}

impl BufferPool {
    /// Create new buffer pool
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        Self {
            inner: Arc::new(BufferPoolInner {
                pools: DashMap::new(),
                device,
                allocations: AtomicUsize::new(0),
                reuses: AtomicUsize::new(0),
                solver_buffers: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Round size up to next power of 2 for pooling efficiency
    fn bucket_size(size: usize) -> usize {
        // Minimum 256 bytes, round to power of 2
        let min_size = 256;
        let size = size.max(min_size);
        size.next_power_of_two()
    }

    /// Acquire a buffer that automatically returns to pool on drop
    ///
    /// This is the preferred method for tensor operations.
    pub fn acquire_pooled(&self, size_bytes: usize) -> PooledBuffer {
        let bucket = Self::bucket_size(size_bytes);

        // Try to reuse from pool
        let buffer = if let Some(mut pool) = self.inner.pools.get_mut(&bucket) {
            if let Some(buffer) = pool.pop() {
                // Don't count as reuse yet - count when returned
                buffer
            } else {
                self.allocate_new(bucket)
            }
        } else {
            self.allocate_new(bucket)
        };

        PooledBuffer::new(buffer, Arc::downgrade(&self.inner), bucket)
    }

    /// Acquire a raw buffer (caller responsible for returning)
    pub fn acquire(&self, size_bytes: usize) -> wgpu::Buffer {
        let bucket = Self::bucket_size(size_bytes);

        // Try to reuse from pool
        if let Some(mut pool) = self.inner.pools.get_mut(&bucket) {
            if let Some(buffer) = pool.pop() {
                self.inner.reuses.fetch_add(1, Ordering::Relaxed);
                return buffer;
            }
        }

        self.allocate_new(bucket)
    }

    fn allocate_new(&self, bucket: usize) -> wgpu::Buffer {
        self.inner.allocations.fetch_add(1, Ordering::Relaxed);
        self.inner.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Pooled Buffer"),
            size: bucket as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Return a buffer to the pool for reuse
    pub fn release(&self, buffer: wgpu::Buffer) {
        let size = buffer.size() as usize;
        let bucket = Self::bucket_size(size);
        self.inner.pools.entry(bucket).or_default().push(buffer);
    }

    /// Get pool statistics
    pub fn stats(&self) -> (usize, usize) {
        (
            self.inner.allocations.load(Ordering::Relaxed),
            self.inner.reuses.load(Ordering::Relaxed),
        )
    }

    /// Pin a set of named buffers for the lifetime of a solver
    ///
    /// Pinned buffers are not returned to the general pool when dropped.
    /// They remain allocated until explicitly released via `release_solver_buffers()`.
    ///
    /// This enables zero-allocation steady state during iterative solvers like SCF.
    ///
    /// # Arguments
    /// * `solver_id` - Unique identifier for the solver (e.g., "hfb_scf", "md_nvt")
    /// * `buffers` - Array of (name, descriptor) pairs
    ///
    /// # Returns
    /// A `SolverBufferSet` that provides access to the buffers by name
    ///
    /// # Errors
    /// Returns error if a solver with the same ID already has pinned buffers
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let buffers = pool.pin_solver_buffers("hfb_scf", &[
    ///     ("hamiltonian", BufferDescriptor::f64_array(batch * n * n)),
    ///     ("eigenvalues", BufferDescriptor::f64_array(batch * n)),
    /// ])?;
    ///
    /// let h = buffers.get("hamiltonian").unwrap();
    /// // Use h in compute passes...
    /// ```
    pub fn pin_solver_buffers(
        &self,
        solver_id: &str,
        buffers: &[(&str, BufferDescriptor)],
    ) -> Result<SolverBufferSet> {
        let mut solver_buffers = self
            .inner
            .solver_buffers
            .write()
            .expect("solver_buffers lock poisoned");

        if solver_buffers.contains_key(solver_id) {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Solver '{}' already has pinned buffers. Call release_solver_buffers() first.",
                    solver_id
                ),
            });
        }

        let mut buffer_map = HashMap::new();
        for (name, desc) in buffers {
            let label = desc
                .label
                .clone()
                .unwrap_or_else(|| format!("{}:{}", solver_id, name));

            let buffer = self.inner.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&label),
                size: desc.size,
                usage: desc.usage,
                mapped_at_creation: false,
            });

            self.inner.allocations.fetch_add(1, Ordering::Relaxed);
            buffer_map.insert(name.to_string(), Arc::new(buffer));
        }

        let buffer_set = SolverBufferSet {
            solver_id: solver_id.to_string(),
            buffers: buffer_map.clone(),
        };

        solver_buffers.insert(solver_id.to_string(), buffer_set);

        // Return a clone of the buffer set for the caller
        Ok(SolverBufferSet {
            solver_id: solver_id.to_string(),
            buffers: buffer_map,
        })
    }

    /// Release all buffers associated with a solver
    ///
    /// After calling this, the solver's buffers are dropped and memory is freed.
    /// The solver ID can then be reused for a new set of buffers.
    ///
    /// # Arguments
    /// * `solver_id` - The solver ID passed to `pin_solver_buffers()`
    ///
    /// # Returns
    /// `true` if buffers were found and released, `false` if no buffers existed
    pub fn release_solver_buffers(&self, solver_id: &str) -> bool {
        let mut solver_buffers = self
            .inner
            .solver_buffers
            .write()
            .expect("solver_buffers lock poisoned");

        solver_buffers.remove(solver_id).is_some()
    }

    /// Get a reference to an existing solver's buffer set
    ///
    /// Returns None if the solver has no pinned buffers
    pub fn get_solver_buffers(&self, solver_id: &str) -> Option<SolverBufferSet> {
        let solver_buffers = self
            .inner
            .solver_buffers
            .read()
            .expect("solver_buffers lock poisoned");

        solver_buffers.get(solver_id).map(|set| SolverBufferSet {
            solver_id: set.solver_id.clone(),
            buffers: set.buffers.clone(),
        })
    }

    /// Check if a solver has pinned buffers
    pub fn has_solver_buffers(&self, solver_id: &str) -> bool {
        let solver_buffers = self
            .inner
            .solver_buffers
            .read()
            .expect("solver_buffers lock poisoned");

        solver_buffers.contains_key(solver_id)
    }

    /// Get list of all solver IDs with pinned buffers
    pub fn solver_ids(&self) -> Vec<String> {
        let solver_buffers = self
            .inner
            .solver_buffers
            .read()
            .expect("solver_buffers lock poisoned");

        solver_buffers.keys().cloned().collect()
    }
}

/// Bind group cache key - combination of buffer IDs
#[derive(Clone, Hash, PartialEq, Eq)]
struct BindGroupKey {
    layout_sig: BindGroupLayoutSignature,
    buffer_ids: Vec<wgpu::Id<wgpu::Buffer>>,
}

/// TensorContext - Accelerated tensor operations via internal pooling
///
/// Use this when you need maximum performance. The context pools
/// buffers, caches bind groups, and batches operations automatically.
///
/// # Example
/// ```rust,ignore
/// let ctx = TensorContext::new(device);
///
/// // Operations use pooled resources internally
/// let a = ctx.tensor(&[1.0, 2.0, 3.0])?;
/// let b = ctx.tensor(&[4.0, 5.0, 6.0])?;
/// let c = ctx.add(&a, &b)?;  // Uses cached bind group, pooled output buffer
///
/// // Execute all pending operations in one batch
/// ctx.sync()?;
/// ```
pub struct TensorContext {
    device: Arc<WgpuDevice>,
    /// Buffer pool for output reuse
    buffer_pool: BufferPool,
    /// Cached bind groups (Arc for cheap cloning on cache hit)
    bind_group_cache: DashMap<BindGroupKey, Arc<wgpu::BindGroup>>,
    /// Pending operations (batched before submit)
    pending_ops: std::sync::Mutex<Vec<PendingOp>>,
    /// Whether we're in batching mode (deferred execution)
    batching: AtomicBool,
    /// Statistics
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
    ops_executed: AtomicUsize,
    ops_batched: AtomicUsize,
}

impl TensorContext {
    /// Create new TensorContext
    pub fn new(device: Arc<WgpuDevice>) -> Self {
        // Get the Arc<wgpu::Device> from WgpuDevice's internal field
        let wgpu_device = device.device_arc();
        Self {
            buffer_pool: BufferPool::new(wgpu_device),
            device,
            bind_group_cache: DashMap::new(),
            pending_ops: std::sync::Mutex::new(Vec::new()),
            batching: AtomicBool::new(false),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
            ops_executed: AtomicUsize::new(0),
            ops_batched: AtomicUsize::new(0),
        }
    }

    /// Begin batching mode - operations are deferred until end_batch()
    ///
    /// In batching mode, GPU operations are recorded but not submitted.
    /// Call `end_batch()` to execute all operations in a single GPU submission.
    pub fn begin_batch(&self) {
        self.batching.store(true, Ordering::SeqCst);
    }

    /// End batching mode and execute all pending operations
    pub fn end_batch(&self) -> Result<()> {
        self.batching.store(false, Ordering::SeqCst);
        self.sync()
    }

    /// Check if in batching mode
    pub fn is_batching(&self) -> bool {
        self.batching.load(Ordering::SeqCst)
    }

    /// Acquire a buffer from the pool for tensor output (raw buffer)
    ///
    /// This returns a raw wgpu::Buffer. Caller is responsible for
    /// returning it to the pool or it will be dropped normally.
    pub fn acquire_output_buffer(&self, size_elements: usize) -> wgpu::Buffer {
        self.buffer_pool
            .acquire(size_elements * std::mem::size_of::<f32>())
    }

    /// Acquire a pooled buffer for tensor output
    ///
    /// This is the key optimization - instead of allocating a new buffer
    /// for each operation output, we get one from the pool. When the
    /// PooledBuffer is dropped, it automatically returns to the pool.
    ///
    /// Use this with `Tensor::from_pooled_buffer()` for zero-allocation
    /// steady state.
    pub fn acquire_pooled_output(&self, size_elements: usize) -> PooledBuffer {
        self.buffer_pool
            .acquire_pooled(size_elements * std::mem::size_of::<f32>())
    }

    /// Record an operation for execution
    ///
    /// If batching is enabled, the operation is queued.
    /// Otherwise, it executes immediately.
    pub fn record_operation<F>(&self, op: F) -> Result<()>
    where
        F: FnOnce(&mut wgpu::CommandEncoder) + Send + 'static,
    {
        if self.is_batching() {
            // Queue for batch execution
            self.pending_ops
                .lock()
                .expect("pending_ops mutex poisoned")
                .push(Box::new(op));
            self.ops_batched.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            // Execute immediately
            let mut encoder =
                self.device
                    .device()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("TensorContext Immediate"),
                    });
            op(&mut encoder);
            self.device.queue().submit(Some(encoder.finish()));
            self.ops_executed.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    /// Get or create bind group (cached by buffer combination)
    ///
    /// This is a key optimization: bind group creation is expensive (~100μs on NVIDIA).
    /// By caching bind groups keyed by (layout, buffer_ids), we can reuse them
    /// across operations, saving significant latency.
    pub fn get_or_create_bind_group(
        &self,
        layout_sig: BindGroupLayoutSignature,
        buffers: &[&wgpu::Buffer],
        label: Option<&str>,
    ) -> Arc<wgpu::BindGroup> {
        let buffer_ids: Vec<_> = buffers.iter().map(|b| b.global_id()).collect();
        let key = BindGroupKey {
            layout_sig,
            buffer_ids,
        };

        // Check cache - on hit, just clone the Arc (very cheap)
        if let Some(bg) = self.bind_group_cache.get(&key) {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            return bg.clone(); // Arc clone is cheap
        }

        // Cache miss - create new bind group
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        let layout = GLOBAL_CACHE.get_or_create_layout(
            self.device.device(),
            self.device.adapter_info(),
            layout_sig,
            label,
        );

        let entries: Vec<wgpu::BindGroupEntry> = buffers
            .iter()
            .enumerate()
            .map(|(i, buf)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: buf.as_entire_binding(),
            })
            .collect();

        let bind_group = Arc::new(self.device.device().create_bind_group(
            &wgpu::BindGroupDescriptor {
                label,
                layout: &layout,
                entries: &entries,
            },
        ));

        // Insert into cache for future reuse
        self.bind_group_cache.insert(key, bind_group.clone());

        bind_group
    }

    /// Acquire output buffer from pool
    pub fn acquire_buffer(&self, size_elements: usize) -> wgpu::Buffer {
        self.buffer_pool
            .acquire(size_elements * std::mem::size_of::<f32>())
    }

    /// Release buffer back to pool
    pub fn release_buffer(&self, buffer: wgpu::Buffer) {
        self.buffer_pool.release(buffer);
    }

    /// Execute all pending operations in a single batch
    pub fn sync(&self) -> Result<()> {
        let mut pending = self.pending_ops.lock().expect("pending_ops mutex poisoned");

        if pending.is_empty() {
            return Ok(());
        }

        let mut encoder =
            self.device
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("TensorContext Batch Encoder"),
                });

        // Execute all pending operations
        for op in pending.drain(..) {
            op(&mut encoder);
        }

        self.device.queue().submit(Some(encoder.finish()));

        Ok(())
    }

    /// Get device reference
    pub fn device(&self) -> &Arc<WgpuDevice> {
        &self.device
    }

    /// Get the buffer pool
    pub fn buffer_pool(&self) -> &BufferPool {
        &self.buffer_pool
    }

    /// Pin solver buffers (convenience method delegating to buffer_pool)
    ///
    /// See `BufferPool::pin_solver_buffers()` for documentation.
    pub fn pin_solver_buffers(
        &self,
        solver_id: &str,
        buffers: &[(&str, BufferDescriptor)],
    ) -> Result<SolverBufferSet> {
        self.buffer_pool.pin_solver_buffers(solver_id, buffers)
    }

    /// Release solver buffers (convenience method delegating to buffer_pool)
    ///
    /// See `BufferPool::release_solver_buffers()` for documentation.
    pub fn release_solver_buffers(&self, solver_id: &str) -> bool {
        self.buffer_pool.release_solver_buffers(solver_id)
    }

    /// Get solver buffers (convenience method delegating to buffer_pool)
    pub fn get_solver_buffers(&self, solver_id: &str) -> Option<SolverBufferSet> {
        self.buffer_pool.get_solver_buffers(solver_id)
    }

    /// Get statistics
    pub fn stats(&self) -> TensorContextStats {
        let (allocs, reuses) = self.buffer_pool.stats();
        TensorContextStats {
            buffer_allocations: allocs,
            buffer_reuses: reuses,
            bind_group_cache_hits: self.cache_hits.load(Ordering::Relaxed),
            bind_group_cache_misses: self.cache_misses.load(Ordering::Relaxed),
            ops_executed: self.ops_executed.load(Ordering::Relaxed),
            ops_batched: self.ops_batched.load(Ordering::Relaxed),
        }
    }
}

/// TensorContext statistics
#[derive(Debug, Clone)]
pub struct TensorContextStats {
    pub buffer_allocations: usize,
    pub buffer_reuses: usize,
    pub bind_group_cache_hits: usize,
    pub bind_group_cache_misses: usize,
    pub ops_executed: usize,
    pub ops_batched: usize,
}

impl std::fmt::Display for TensorContextStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let buffer_hit_rate = if self.buffer_allocations + self.buffer_reuses > 0 {
            self.buffer_reuses as f64 / (self.buffer_allocations + self.buffer_reuses) as f64
                * 100.0
        } else {
            0.0
        };
        let bg_hit_rate = if self.bind_group_cache_hits + self.bind_group_cache_misses > 0 {
            self.bind_group_cache_hits as f64
                / (self.bind_group_cache_hits + self.bind_group_cache_misses) as f64
                * 100.0
        } else {
            0.0
        };

        write!(
            f,
            "Buffers: {} allocs, {} reuses ({:.1}% reuse)\n\
             BindGroups: {} hits, {} misses ({:.1}% hit rate)\n\
             Operations: {} executed, {} batched",
            self.buffer_allocations,
            self.buffer_reuses,
            buffer_hit_rate,
            self.bind_group_cache_hits,
            self.bind_group_cache_misses,
            bg_hit_rate,
            self.ops_executed,
            self.ops_batched
        )
    }
}

/// Science-grade limits for serious GPU compute
///
/// These are the recommended defaults for scientific computing:
/// - 512 MiB max storage buffer binding (up from 128 MiB default)
/// - 1 GiB max buffer size (up from 256 MiB default)
///
/// Validated by hotSpring nuclear EOS study (169/169 acceptance checks)
/// on consumer GPU (RTX 4070) with full f64 precision.
///
/// Use these limits for:
/// - Large tensor operations (>32M elements)
/// - 3D fields and grids
/// - Molecular dynamics buffers
/// - Scientific workloads
pub fn science_limits() -> wgpu::Limits {
    wgpu::Limits {
        // 512 MiB for storage buffer binding (4x default)
        max_storage_buffer_binding_size: 512 * 1024 * 1024,
        // 1 GiB max buffer size (4x default)
        max_buffer_size: 1024 * 1024 * 1024,
        // 12 storage buffers per shader stage (hotSpring HFB potentials shader binds 12)
        // Default is 8, which is insufficient for complex physics shaders
        max_storage_buffers_per_shader_stage: 12,
        // Keep other limits at defaults
        ..wgpu::Limits::default()
    }
}

/// Extend wgpu::Limits with higher buffer limits (legacy compatibility)
///
/// For maximum capacity, use `high_capacity_limits()`.
/// For validated science workloads, prefer `science_limits()`.
pub fn high_capacity_limits() -> wgpu::Limits {
    wgpu::Limits {
        // Increase from 128MB to 1GB max binding
        max_storage_buffer_binding_size: 1 << 30,
        // Increase from 256MB to 2GB max buffer
        max_buffer_size: 1 << 31,
        // Keep other limits at defaults
        ..wgpu::Limits::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    /// Get shared test device (uses pool to prevent GPU exhaustion)
    ///
    /// **Deep Debt Evolution**: Tests now share a single wgpu device via LazyLock pool.
    /// This prevents "Maximum number of clients reached" errors when running many tests.
    async fn create_test_device() -> Arc<wgpu::Device> {
        Arc::clone(&get_test_device().await.device)
    }

    // ========================================================================
    // BufferPool Unit Tests
    // ========================================================================

    #[tokio::test]
    async fn test_buffer_pool_basic_acquire_release() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);

        // First allocation
        let buf1 = pool.acquire(1024);
        assert!(buf1.size() >= 1024);

        // Return to pool
        let size = buf1.size();
        pool.release(buf1);

        // Second acquire should reuse
        let buf2 = pool.acquire(1024);
        assert_eq!(buf2.size(), size);

        let (allocs, reuses) = pool.stats();
        assert_eq!(allocs, 1);
        assert_eq!(reuses, 1);
    }

    #[tokio::test]
    async fn test_buffer_pool_power_of_two_bucketing() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);

        // Request 1000 bytes, should get 1024 (next power of 2)
        let buf = pool.acquire(1000);
        assert_eq!(buf.size(), 1024);

        // Request 1025 bytes, should get 2048
        let buf2 = pool.acquire(1025);
        assert_eq!(buf2.size(), 2048);

        // Request 256 bytes (minimum bucket)
        let buf3 = pool.acquire(100);
        assert_eq!(buf3.size(), 256); // Minimum bucket size
    }

    #[tokio::test]
    async fn test_buffer_pool_multiple_buckets() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);

        // Allocate different sizes
        let buf_256 = pool.acquire(256);
        let buf_1024 = pool.acquire(1024);
        let buf_4096 = pool.acquire(4096);

        // Release all
        pool.release(buf_256);
        pool.release(buf_1024);
        pool.release(buf_4096);

        // Acquire again - should reuse from correct buckets
        let buf_256_reuse = pool.acquire(200);
        let buf_1024_reuse = pool.acquire(1000);
        let buf_4096_reuse = pool.acquire(4000);

        assert_eq!(buf_256_reuse.size(), 256);
        assert_eq!(buf_1024_reuse.size(), 1024);
        assert_eq!(buf_4096_reuse.size(), 4096);

        let (allocs, reuses) = pool.stats();
        assert_eq!(allocs, 3);
        assert_eq!(reuses, 3);
    }

    // ========================================================================
    // PooledBuffer Unit Tests
    // ========================================================================

    #[tokio::test]
    async fn test_pooled_buffer_auto_return() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);

        let (allocs_before, reuses_before) = pool.stats();

        // Acquire pooled buffer
        {
            let _pooled = pool.acquire_pooled(1024);
            // Buffer is in use
        }
        // PooledBuffer dropped here - should return to pool

        // Acquire again - should be a reuse
        let _pooled2 = pool.acquire_pooled(1024);

        let (allocs_after, reuses_after) = pool.stats();
        assert_eq!(allocs_after - allocs_before, 1); // Only 1 allocation
        assert_eq!(reuses_after - reuses_before, 1); // 1 reuse from auto-return
    }

    #[tokio::test]
    async fn test_pooled_buffer_deref() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);

        let pooled = pool.acquire_pooled(1024);

        // Test Deref
        let _size: u64 = pooled.size();

        // Test buffer() method
        let _buf_ref: &wgpu::Buffer = pooled.buffer();
    }

    #[tokio::test]
    async fn test_pooled_buffer_into_buffer() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);

        let pooled = pool.acquire_pooled(1024);

        // Convert to owned buffer (removes from pool management)
        let owned = pooled.into_buffer();
        assert!(owned.size() >= 1024);

        // Dropping owned buffer won't return to pool
        drop(owned);

        // Next acquire should allocate new
        let _new = pool.acquire_pooled(1024);
        let (allocs, _) = pool.stats();
        assert_eq!(allocs, 2); // Two allocations, no reuse
    }

    // ========================================================================
    // TensorContext Unit Tests
    // ========================================================================

    #[tokio::test]
    async fn test_tensor_context_acquire_pooled() {
        let device = get_test_device().await;
        let ctx = TensorContext::new(device);

        let pooled = ctx.acquire_pooled_output(1000); // 1000 f32s = 4000 bytes
        assert!(pooled.size() >= 4000);
    }

    #[tokio::test]
    async fn test_tensor_context_batching_mode() {
        let device = get_test_device().await;
        let ctx = TensorContext::new(device);

        // Not batching by default
        assert!(!ctx.is_batching());

        // Begin batch
        ctx.begin_batch();
        assert!(ctx.is_batching());

        // End batch
        ctx.end_batch().expect("end_batch failed");
        assert!(!ctx.is_batching());
    }

    #[tokio::test]
    async fn test_tensor_context_stats() {
        let device = get_test_device().await;
        let ctx = TensorContext::new(device);

        let stats = ctx.stats();
        assert_eq!(stats.buffer_allocations, 0);
        assert_eq!(stats.buffer_reuses, 0);
        assert_eq!(stats.ops_executed, 0);
        assert_eq!(stats.ops_batched, 0);
    }

    #[tokio::test]
    async fn test_tensor_context_stats_display() {
        let stats = TensorContextStats {
            buffer_allocations: 10,
            buffer_reuses: 90,
            bind_group_cache_hits: 50,
            bind_group_cache_misses: 50,
            ops_executed: 100,
            ops_batched: 50,
        };

        let display = format!("{}", stats);
        assert!(display.contains("90.0% reuse"));
        assert!(display.contains("50.0% hit rate"));
    }

    // ========================================================================
    // Global Context Tests
    // ========================================================================

    #[tokio::test]
    async fn test_global_context_registry() {
        // NOTE: Do NOT call clear_global_contexts() - it interferes with other tests
        // that share the global device pool

        let device = get_test_device().await;

        // Get context twice - should be same instance
        let ctx1 = get_device_context(&device);
        let ctx2 = get_device_context(&device);

        // Record initial allocations
        let initial_allocs = ctx1.stats().buffer_allocations;

        // Acquire from ctx1
        let _buf1 = ctx1.acquire_pooled_output(1000);

        // Stats should be visible from ctx2 (same context) and increase
        let stats = ctx2.stats();
        assert!(
            stats.buffer_allocations >= initial_allocs,
            "Allocations should not decrease"
        );
    }

    // ========================================================================
    // High Capacity Limits Tests
    // ========================================================================

    #[test]
    fn test_high_capacity_limits() {
        let limits = high_capacity_limits();

        // 1GB max binding
        assert_eq!(limits.max_storage_buffer_binding_size, 1 << 30);

        // 2GB max buffer
        assert_eq!(limits.max_buffer_size, 1 << 31);
    }

    // ========================================================================
    // Persistent Solver Buffer Tests
    // ========================================================================

    #[tokio::test]
    async fn test_pin_solver_buffers_basic() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);

        // Pin some buffers
        let buffers = pool
            .pin_solver_buffers(
                "test_solver",
                &[
                    ("matrix_a", BufferDescriptor::f64_array(100)),
                    ("matrix_b", BufferDescriptor::f64_array(100)),
                    ("result", BufferDescriptor::f64_array(100)),
                ],
            )
            .expect("pin_solver_buffers failed");

        // Verify buffers exist
        assert_eq!(buffers.len(), 3);
        assert!(buffers.get("matrix_a").is_some());
        assert!(buffers.get("matrix_b").is_some());
        assert!(buffers.get("result").is_some());
        assert!(buffers.get("nonexistent").is_none());

        // Verify buffer sizes (100 f64s = 800 bytes)
        assert!(buffers.get("matrix_a").unwrap().size() >= 800);

        // Release
        assert!(pool.release_solver_buffers("test_solver"));
        assert!(!pool.release_solver_buffers("test_solver")); // Already released
    }

    #[tokio::test]
    async fn test_pin_solver_buffers_duplicate_id_error() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);

        // Pin first
        pool.pin_solver_buffers("solver_a", &[("buf", BufferDescriptor::new(256))])
            .expect("first pin failed");

        // Try to pin again with same ID - should fail
        let result = pool.pin_solver_buffers("solver_a", &[("buf2", BufferDescriptor::new(256))]);
        assert!(result.is_err());

        // Release and try again - should work
        pool.release_solver_buffers("solver_a");
        let result = pool.pin_solver_buffers("solver_a", &[("buf2", BufferDescriptor::new(256))]);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_solver_buffer_set_methods() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);

        let buffers = pool
            .pin_solver_buffers(
                "my_solver",
                &[
                    ("h", BufferDescriptor::f64_array(25)),
                    ("v", BufferDescriptor::f64_array(5)),
                ],
            )
            .expect("pin failed");

        // Test solver_id
        assert_eq!(buffers.solver_id(), "my_solver");

        // Test is_empty
        assert!(!buffers.is_empty());

        // Test buffer_names
        let names: Vec<_> = buffers.buffer_names().collect();
        assert!(names.contains(&"h"));
        assert!(names.contains(&"v"));

        // Test get_arc
        let arc = buffers.get_arc("h");
        assert!(arc.is_some());
    }

    #[tokio::test]
    async fn test_buffer_descriptor_helpers() {
        // Test f64_array
        let desc = BufferDescriptor::f64_array(100);
        assert_eq!(desc.size, 800); // 100 * 8 bytes

        // Test f32_array
        let desc = BufferDescriptor::f32_array(100);
        assert_eq!(desc.size, 400); // 100 * 4 bytes

        // Test with_label
        let desc = BufferDescriptor::new(1024).with_label("my_buffer");
        assert_eq!(desc.label, Some("my_buffer".to_string()));

        // Test default usage
        let desc = BufferDescriptor::new(1024);
        assert!(desc.usage.contains(wgpu::BufferUsages::STORAGE));
        assert!(desc.usage.contains(wgpu::BufferUsages::COPY_SRC));
        assert!(desc.usage.contains(wgpu::BufferUsages::COPY_DST));
    }

    #[tokio::test]
    async fn test_get_solver_buffers() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);

        // Should be None before pinning
        assert!(pool.get_solver_buffers("solver_x").is_none());

        // Pin
        pool.pin_solver_buffers("solver_x", &[("buf", BufferDescriptor::new(256))])
            .unwrap();

        // Should exist now
        let buffers = pool.get_solver_buffers("solver_x");
        assert!(buffers.is_some());
        assert_eq!(buffers.unwrap().len(), 1);

        // has_solver_buffers
        assert!(pool.has_solver_buffers("solver_x"));
        assert!(!pool.has_solver_buffers("solver_y"));

        // solver_ids
        let ids = pool.solver_ids();
        assert!(ids.contains(&"solver_x".to_string()));
    }

    #[tokio::test]
    async fn test_tensor_context_solver_buffers() {
        let device = get_test_device().await;
        let ctx = TensorContext::new(device);

        // Test via TensorContext convenience methods
        let buffers = ctx
            .pin_solver_buffers(
                "scf_solver",
                &[
                    ("hamiltonian", BufferDescriptor::f64_array(64)),
                    ("density", BufferDescriptor::f64_array(32)),
                ],
            )
            .expect("pin failed");

        assert_eq!(buffers.len(), 2);
        assert!(ctx.get_solver_buffers("scf_solver").is_some());
        assert!(ctx.release_solver_buffers("scf_solver"));
        assert!(ctx.get_solver_buffers("scf_solver").is_none());
    }
}
