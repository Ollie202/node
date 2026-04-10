use std::cell::UnsafeCell;

/// A single-writer wrapper that provides interior mutability for the writer task.
///
/// This type enables a pattern where one dedicated writer task mutates data stored in a shared
/// `Arc<State>`, without any locks. Readers do NOT access the wrapped data — they use
/// snapshot-backed copies in `InMemoryState` via `ArcSwap` instead.
///
/// # Safety Contract
///
/// **Single writer**: Only one task (the writer, serialized by a channel) may call
/// [`as_mut()`](Self::as_mut). This invariant is enforced architecturally, not by the type
/// system. No reader access is provided.
pub struct WriterGuard<T> {
    inner: UnsafeCell<T>,
}

// SAFETY: The single-writer invariant is enforced by the channel-based writer task architecture.
// No reader access is provided — readers use snapshot-backed copies in InMemoryState instead.
unsafe impl<T: Send + Sync> Send for WriterGuard<T> {}
unsafe impl<T: Send + Sync> Sync for WriterGuard<T> {}

impl<T> WriterGuard<T> {
    /// Creates a new `WriterGuard` wrapping the given value.
    pub fn new(value: T) -> Self {
        Self { inner: UnsafeCell::new(value) }
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
