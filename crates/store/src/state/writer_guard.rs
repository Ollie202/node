use std::cell::UnsafeCell;

/// A single-writer / multi-reader wrapper that provides lock-free access to shared state.
///
/// This type enables a pattern where one dedicated writer task mutates data while many reader
/// tasks concurrently access it, without any locks.
///
/// # Safety Contract
///
/// 1. **Single writer**: Only one task (the writer, serialized by a channel) may call
///    [`as_mut()`](Self::as_mut). This invariant is enforced architecturally, not by the type
///    system.
/// 2. **Concurrent read safety**: The wrapped type (currently the `RocksDB`-backed nullifier tree)
///    provides its own MVCC / snapshot isolation, making concurrent reads during writes safe at the
///    storage layer.
/// 3. **Append-only data**: The wrapped data structures are append-only (keyed by block number), so
///    readers observing an older state simply query at that older block number, which is safe.
///
/// Concurrent read safety relies on the guarantees of the underlying storage engine (e.g.
/// `RocksDB` MVCC).
pub struct WriterGuard<T> {
    inner: UnsafeCell<T>,
}

// SAFETY: The single-writer invariant is enforced by the channel-based writer task architecture.
// Readers only call `as_ref()` which returns `&T`. Concurrent read safety during writes is
// guaranteed by the underlying storage engine (RocksDB MVCC / snapshot isolation), not by
// acquire/release barriers. The data structures are append-only, so readers see a consistent
// view at their query's block number.
unsafe impl<T: Send + Sync> Send for WriterGuard<T> {}
unsafe impl<T: Send + Sync> Sync for WriterGuard<T> {}

impl<T> WriterGuard<T> {
    /// Creates a new `WriterGuard` wrapping the given value.
    pub fn new(value: T) -> Self {
        Self { inner: UnsafeCell::new(value) }
    }

    /// Returns a shared reference to the wrapped value.
    ///
    /// Safe for any reader thread. Concurrent read safety is provided by the underlying storage
    /// engine (e.g. `RocksDB` MVCC / snapshot isolation). The data structures are append-only,
    /// so readers see a consistent view at their query's block number.
    pub(super) fn as_ref(&self) -> &T {
        // SAFETY: Single-writer is enforced by the channel-based writer task. Concurrent reads
        // are safe because the underlying storage engine (RocksDB) provides MVCC / snapshot
        // isolation, and the data is append-only.
        unsafe { &*self.inner.get() }
    }

    /// Returns an exclusive mutable reference to the wrapped value.
    ///
    /// # Safety
    ///
    /// Must only be called from the single writer task. The caller must ensure:
    /// - No other calls to `as_mut()` are concurrent (enforced by channel serialization).
    #[expect(clippy::mut_from_ref)]
    pub unsafe fn as_mut(&self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }
}
