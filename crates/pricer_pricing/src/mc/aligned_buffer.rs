//! Aligned memory buffers for SIMD-optimised Monte Carlo simulation.
//!
//! This module provides [`AlignedPathBuffer`], a memory buffer that can be
//! aligned to specified boundaries (e.g., 64 bytes for AVX-512 cache lines).
//!
//! # Feature Flags
//!
//! - `simd-aligned`: Uses `aligned-vec` crate for guaranteed alignment
//! - Without flag: Falls back to standard `Vec<T>` (alignment not guaranteed)
//!
//! # Performance
//!
//! Proper alignment enables:
//! - Efficient SIMD operations (AVX-512 requires 64-byte alignment)
//! - Reduced cache line splits
//! - Better hardware prefetching

use num_traits::Float;
use std::marker::PhantomData;

/// Default alignment for AVX-512 cache lines.
pub const DEFAULT_ALIGNMENT: usize = 64;

/// Aligned memory buffer for path data.
///
/// Provides a contiguous buffer of floating-point values that can be
/// aligned to specified memory boundaries for SIMD efficiency.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (typically `f32` or `f64`)
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::mc::AlignedPathBuffer;
///
/// // Create buffer with default alignment (64 bytes)
/// let mut buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(1000);
///
/// // Fill buffer
/// for i in 0..1000 {
///     buffer.as_mut_slice()[i] = i as f64;
/// }
///
/// // Check alignment
/// assert!(buffer.alignment() >= 8); // At least f64 alignment
/// ```
#[cfg(feature = "simd-aligned")]
pub struct AlignedPathBuffer<T: Float> {
    inner: aligned_vec::AVec<T, aligned_vec::ConstAlign<DEFAULT_ALIGNMENT>>,
}

#[cfg(not(feature = "simd-aligned"))]
pub struct AlignedPathBuffer<T: Float> {
    inner: Vec<T>,
    _marker: PhantomData<T>,
}

impl<T: Float> AlignedPathBuffer<T> {
    /// Creates a new buffer with the specified capacity.
    ///
    /// The buffer is initialised with zeros.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Number of elements to allocate
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::AlignedPathBuffer;
    ///
    /// let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(1000);
    /// assert_eq!(buffer.len(), 1000);
    /// ```
    #[cfg(feature = "simd-aligned")]
    pub fn new(capacity: usize) -> Self {
        use aligned_vec::AVec;
        let mut inner = AVec::new(DEFAULT_ALIGNMENT);
        inner.resize(capacity, T::zero());
        Self { inner }
    }

    #[cfg(not(feature = "simd-aligned"))]
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: vec![T::zero(); capacity],
            _marker: PhantomData,
        }
    }

    /// Creates a buffer with custom alignment.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Number of elements to allocate
    /// * `alignment` - Alignment in bytes (ignored without `simd-aligned` feature)
    ///
    /// # Panics
    ///
    /// Panics if `alignment` is not a power of 2 (with `simd-aligned` feature).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::AlignedPathBuffer;
    ///
    /// let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::with_alignment(1000, 64);
    /// ```
    #[cfg(feature = "simd-aligned")]
    pub fn with_alignment(capacity: usize, alignment: usize) -> Self {
        use aligned_vec::AVec;
        assert!(
            alignment > 0 && alignment.is_power_of_two(),
            "alignment must be a power of 2, got {}",
            alignment
        );
        let mut inner = AVec::new(alignment);
        inner.resize(capacity, T::zero());
        Self { inner }
    }

    #[cfg(not(feature = "simd-aligned"))]
    pub fn with_alignment(capacity: usize, _alignment: usize) -> Self {
        Self::new(capacity)
    }

    /// Returns an immutable slice of the buffer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::AlignedPathBuffer;
    ///
    /// let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(10);
    /// let slice = buffer.as_slice();
    /// assert_eq!(slice.len(), 10);
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.inner
    }

    /// Returns a mutable slice of the buffer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::AlignedPathBuffer;
    ///
    /// let mut buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(10);
    /// buffer.as_mut_slice()[0] = 42.0;
    /// assert_eq!(buffer.as_slice()[0], 42.0);
    /// ```
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.inner
    }

    /// Returns the number of elements in the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the alignment of the buffer in bytes.
    ///
    /// # Note
    ///
    /// Without the `simd-aligned` feature, this returns the natural
    /// alignment of type `T` (8 bytes for `f64`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::AlignedPathBuffer;
    ///
    /// let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(1000);
    /// println!("Alignment: {} bytes", buffer.alignment());
    /// ```
    #[cfg(feature = "simd-aligned")]
    #[inline]
    pub fn alignment(&self) -> usize {
        DEFAULT_ALIGNMENT
    }

    #[cfg(not(feature = "simd-aligned"))]
    #[inline]
    pub fn alignment(&self) -> usize {
        std::mem::align_of::<T>()
    }

    /// Checks if the buffer data is aligned to the specified boundary.
    ///
    /// # Arguments
    ///
    /// * `alignment` - Alignment in bytes to check
    ///
    /// # Returns
    ///
    /// `true` if the buffer data pointer is aligned to `alignment` bytes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::AlignedPathBuffer;
    ///
    /// let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(1000);
    /// assert!(buffer.is_aligned_to(8)); // Always true for f64
    /// ```
    #[inline]
    pub fn is_aligned_to(&self, alignment: usize) -> bool {
        let ptr = self.inner.as_ptr() as usize;
        ptr % alignment == 0
    }

    /// Returns the memory usage of the buffer in bytes.
    #[inline]
    pub fn memory_usage(&self) -> usize {
        self.inner.len() * std::mem::size_of::<T>()
    }

    /// Resizes the buffer to the specified length.
    ///
    /// If the new length is greater than the current length, the buffer
    /// is extended with zeros.
    ///
    /// # Arguments
    ///
    /// * `new_len` - New number of elements
    pub fn resize(&mut self, new_len: usize) {
        self.inner.resize(new_len, T::zero());
    }

    /// Clears the buffer, setting all elements to zero.
    ///
    /// Does not deallocate memory.
    pub fn clear(&mut self) {
        for elem in self.inner.iter_mut() {
            *elem = T::zero();
        }
    }

    /// Returns the raw pointer to the buffer data.
    ///
    /// # Safety
    ///
    /// The returned pointer is valid for `len()` elements.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.inner.as_ptr()
    }

    /// Returns a mutable raw pointer to the buffer data.
    ///
    /// # Safety
    ///
    /// The returned pointer is valid for `len()` elements.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.inner.as_mut_ptr()
    }
}

// Implement Clone manually since we need Float bound
impl<T: Float + Clone> Clone for AlignedPathBuffer<T> {
    fn clone(&self) -> Self {
        #[cfg(feature = "simd-aligned")]
        {
            let mut inner = aligned_vec::AVec::new(DEFAULT_ALIGNMENT);
            inner.extend(self.inner.iter().cloned());
            Self { inner }
        }
        #[cfg(not(feature = "simd-aligned"))]
        {
            Self {
                inner: self.inner.clone(),
                _marker: PhantomData,
            }
        }
    }
}

// Safety: AlignedPathBuffer is Send + Sync if T is
unsafe impl<T: Float + Send> Send for AlignedPathBuffer<T> {}
unsafe impl<T: Float + Sync> Sync for AlignedPathBuffer<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_buffer_new() {
        let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(1000);
        assert_eq!(buffer.len(), 1000);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_aligned_buffer_empty() {
        let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(0);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_aligned_buffer_with_alignment() {
        let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::with_alignment(1000, 64);
        assert_eq!(buffer.len(), 1000);
    }

    #[test]
    fn test_aligned_buffer_as_slice() {
        let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(10);
        let slice = buffer.as_slice();
        assert_eq!(slice.len(), 10);
        // All elements should be zero
        for &val in slice {
            assert_eq!(val, 0.0);
        }
    }

    #[test]
    fn test_aligned_buffer_as_mut_slice() {
        let mut buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(10);
        buffer.as_mut_slice()[0] = 42.0;
        buffer.as_mut_slice()[9] = 99.0;
        assert_eq!(buffer.as_slice()[0], 42.0);
        assert_eq!(buffer.as_slice()[9], 99.0);
    }

    #[test]
    fn test_aligned_buffer_alignment() {
        let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(1000);
        // Alignment should be at least f64 alignment
        assert!(buffer.alignment() >= 8);
    }

    #[test]
    fn test_aligned_buffer_is_aligned_to() {
        let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(1000);
        // Should always be aligned to f64 (8 bytes)
        assert!(buffer.is_aligned_to(8));
    }

    #[test]
    fn test_aligned_buffer_memory_usage() {
        let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(1000);
        let expected = 1000 * std::mem::size_of::<f64>();
        assert_eq!(buffer.memory_usage(), expected);
    }

    #[test]
    fn test_aligned_buffer_resize() {
        let mut buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(100);
        assert_eq!(buffer.len(), 100);

        buffer.resize(200);
        assert_eq!(buffer.len(), 200);

        buffer.resize(50);
        assert_eq!(buffer.len(), 50);
    }

    #[test]
    fn test_aligned_buffer_clear() {
        let mut buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(10);
        buffer.as_mut_slice()[0] = 42.0;
        buffer.as_mut_slice()[5] = 99.0;

        buffer.clear();

        for &val in buffer.as_slice() {
            assert_eq!(val, 0.0);
        }
    }

    #[test]
    fn test_aligned_buffer_clone() {
        let mut buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(10);
        buffer.as_mut_slice()[0] = 42.0;

        let cloned = buffer.clone();
        assert_eq!(cloned.len(), 10);
        assert_eq!(cloned.as_slice()[0], 42.0);
    }

    #[test]
    fn test_aligned_buffer_f32() {
        let buffer: AlignedPathBuffer<f32> = AlignedPathBuffer::new(1000);
        assert_eq!(buffer.len(), 1000);
        // f32 alignment is 4 bytes
        assert!(buffer.alignment() >= 4);
    }

    #[test]
    fn test_aligned_buffer_large_capacity() {
        // Test with 1 million elements
        let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(1_000_000);
        assert_eq!(buffer.len(), 1_000_000);
        assert_eq!(buffer.memory_usage(), 1_000_000 * 8);
    }

    #[test]
    fn test_aligned_buffer_ptr() {
        let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(100);
        let ptr = buffer.as_ptr();
        assert!(!ptr.is_null());
        // Verify pointer alignment to f64
        assert_eq!(ptr as usize % std::mem::align_of::<f64>(), 0);
    }

    #[test]
    fn test_aligned_buffer_mut_ptr() {
        let mut buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(100);
        let ptr = buffer.as_mut_ptr();
        assert!(!ptr.is_null());
        // Write through pointer
        unsafe {
            *ptr = 42.0;
        }
        assert_eq!(buffer.as_slice()[0], 42.0);
    }

    #[cfg(feature = "simd-aligned")]
    #[test]
    fn test_aligned_buffer_simd_alignment() {
        let buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::new(1000);
        // With simd-aligned feature, should be 64-byte aligned
        assert_eq!(buffer.alignment(), 64);
        assert!(buffer.is_aligned_to(64));
    }

    #[cfg(feature = "simd-aligned")]
    #[test]
    #[should_panic(expected = "alignment must be a power of 2")]
    fn test_aligned_buffer_invalid_alignment() {
        let _buffer: AlignedPathBuffer<f64> = AlignedPathBuffer::with_alignment(1000, 7);
    }
}
