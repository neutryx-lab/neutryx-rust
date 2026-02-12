// SAFETY: This module requires unsafe code for custom memory allocation
// with guaranteed alignment. The unsafe operations are:
// 1. Custom allocation/deallocation via std::alloc
// 2. Implementing Send/Sync for the owned buffer type
// 3. Raw pointer operations for element access
//
// All unsafe blocks are documented with safety invariants.
#![allow(unsafe_code)]

//! 64-byte aligned memory buffer for SIMD-optimised pricing kernels.
//!
//! This module provides [`AlignedBuffer<T>`], a heap-allocated buffer with
//! guaranteed 64-byte alignment for AVX-512 compatibility and optimal cache
//! line utilisation.
//!
//! # Design Rationale
//!
//! Unlike `#[repr(align(64))]` which only affects stack alignment,
//! `AlignedBuffer` uses `std::alloc::Layout` to guarantee heap memory
//! alignment. This is critical for:
//!
//! - AVX-512 instructions (`vmovaps` requires aligned loads)
//! - Cache line efficiency (64-byte cache lines on modern CPUs)
//! - Enzyme AD compatibility (contiguous, aligned memory)
//!
//! # Safety
//!
//! The implementation uses `unsafe` for custom memory allocation but
//! maintains safety through RAII patterns and careful lifetime management.

use std::{
    alloc::{alloc_zeroed, dealloc, Layout},
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut},
    ptr::NonNull,
    slice,
};

/// Alignment in bytes for AVX-512 cache lines.
pub const ALIGNMENT: usize = 64;

/// A 64-byte aligned heap buffer for SIMD-optimised operations and AD.
pub struct AlignedBuffer<T> {
    ptr: NonNull<T>,
    len: usize,
    cap: usize,
    _marker: PhantomData<T>,
}

// SAFETY: AlignedBuffer owns its data exclusively, so it's safe to send
// across threads if T is Send.
unsafe impl<T: Send> Send for AlignedBuffer<T> {}

// SAFETY: AlignedBuffer only allows shared access through &[T], which is
// safe if T is Sync.
unsafe impl<T: Sync> Sync for AlignedBuffer<T> {}

impl<T: Clone + Default> AlignedBuffer<T> {
    /// Creates a new aligned buffer with `capacity` elements initialised to
    /// `T::default()`.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity == 0 {
            return Self {
                ptr: NonNull::dangling(),
                len: 0,
                cap: 0,
                _marker: PhantomData,
            };
        }

        let size = capacity
            .checked_mul(std::mem::size_of::<T>())
            .expect("Capacity overflow");

        // SAFETY: We ensure alignment is non-zero and a power of 2 (64),
        // and size is valid for the allocation.
        let layout = Layout::from_size_align(size, ALIGNMENT).expect("Invalid layout");

        // SAFETY: Layout is valid, and we check for null pointer.
        let ptr = unsafe { alloc_zeroed(layout).cast::<T>() };

        let ptr = NonNull::new(ptr).expect("Memory allocation failed");

        // Initialise all elements to default
        // SAFETY: Memory is allocated and zeroed, we initialise each element.
        for i in 0..capacity {
            unsafe {
                std::ptr::write(ptr.as_ptr().add(i), T::default());
            }
        }

        Self {
            ptr,
            len: capacity,
            cap: capacity,
            _marker: PhantomData,
        }
    }

    /// Creates an aligned buffer by copying a `Vec<T>` into aligned memory.
    #[must_use]
    pub fn from_vec(vec: Vec<T>) -> Self {
        if vec.is_empty() {
            return Self::with_capacity(0);
        }

        let buf = Self::with_capacity(vec.len());

        // Copy elements from vec to aligned buffer
        for (i, item) in vec.into_iter().enumerate() {
            // SAFETY: i is within bounds and memory is valid.
            unsafe {
                std::ptr::write(buf.ptr.as_ptr().add(i), item);
            }
        }

        buf
    }

    /// Creates an aligned buffer by copying a slice into aligned memory.
    #[must_use]
    pub fn from_slice(slice: &[T]) -> Self {
        if slice.is_empty() {
            return Self::with_capacity(0);
        }

        let buf = Self::with_capacity(slice.len());

        for (i, item) in slice.iter().enumerate() {
            // SAFETY: i is within bounds and memory is valid.
            unsafe {
                std::ptr::write(buf.ptr.as_ptr().add(i), item.clone());
            }
        }

        buf
    }
}

impl<T> AlignedBuffer<T> {
    /// Returns the number of elements in the buffer.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize { self.len }

    /// Returns `true` if the buffer contains no elements.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool { self.len == 0 }

    /// Returns the capacity of the buffer in elements.
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize { self.cap }

    /// Returns `true` if the buffer data is 64-byte aligned.
    #[inline]
    #[must_use]
    #[allow(unknown_lints, clippy::manual_is_multiple_of)] // is_multiple_of is unstable
    pub fn is_aligned(&self) -> bool {
        if self.cap == 0 {
            return true;
        }
        (self.ptr.as_ptr() as usize) % ALIGNMENT == 0
    }

    /// Returns the memory alignment in bytes.
    #[inline]
    #[must_use]
    pub fn alignment(&self) -> usize { ALIGNMENT }

    /// Returns a raw pointer to the buffer data, valid for `len()` elements.
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *const T { self.ptr.as_ptr() }

    /// Returns a mutable raw pointer to the buffer data, valid for `len()`
    /// elements.
    #[inline]
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut T { self.ptr.as_ptr() }

    /// Returns the buffer as a slice.
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: ptr is valid for len elements.
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    /// Returns the buffer as a mutable slice.
    #[inline]
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        // SAFETY: ptr is valid for len elements and we have exclusive access.
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    /// Returns an iterator over the buffer elements.
    #[inline]
    pub fn iter(&self) -> slice::Iter<'_, T> { self.as_slice().iter() }

    /// Returns a mutable iterator over the buffer elements.
    #[inline]
    pub fn iter_mut(&mut self) -> slice::IterMut<'_, T> { self.as_mut_slice().iter_mut() }

    /// Returns the total memory usage in bytes.
    #[inline]
    #[must_use]
    pub fn memory_usage(&self) -> usize { self.cap * std::mem::size_of::<T>() }
}

impl<'a, T> IntoIterator for &'a AlignedBuffer<T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter { self.iter() }
}

impl<'a, T> IntoIterator for &'a mut AlignedBuffer<T> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter { self.iter_mut() }
}

impl<T> Drop for AlignedBuffer<T> {
    fn drop(&mut self) {
        if self.cap == 0 {
            return;
        }

        let size = self.cap * std::mem::size_of::<T>();
        let layout = Layout::from_size_align(size, ALIGNMENT).expect("Invalid layout");

        // Drop all elements first
        // SAFETY: All elements within len are valid.
        unsafe {
            for i in 0..self.len {
                std::ptr::drop_in_place(self.ptr.as_ptr().add(i));
            }
        }

        // SAFETY: ptr was allocated with this layout.
        unsafe {
            dealloc(self.ptr.as_ptr().cast::<u8>(), layout);
        }
    }
}

impl<T: Clone + Default> Clone for AlignedBuffer<T> {
    fn clone(&self) -> Self {
        if self.cap == 0 {
            return Self::with_capacity(0);
        }

        let mut buf = Self::with_capacity(self.cap);
        buf.len = self.len;

        // SAFETY: Both buffers have valid memory for len elements.
        for i in 0..self.len {
            unsafe {
                let src = self.ptr.as_ptr().add(i);
                let dst = buf.ptr.as_ptr().add(i);
                std::ptr::write(dst, (*src).clone());
            }
        }

        buf
    }
}

impl<T: fmt::Debug> fmt::Debug for AlignedBuffer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T> Deref for AlignedBuffer<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] { self.as_slice() }
}

impl<T> DerefMut for AlignedBuffer<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] { self.as_mut_slice() }
}

impl<T> Index<usize> for AlignedBuffer<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output { &self.as_slice()[index] }
}

impl<T> IndexMut<usize> for AlignedBuffer<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output { &mut self.as_mut_slice()[index] }
}

impl<T: PartialEq> PartialEq for AlignedBuffer<T> {
    fn eq(&self, other: &Self) -> bool { self.as_slice() == other.as_slice() }
}

impl<T: Eq> Eq for AlignedBuffer<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_buffer_with_capacity() {
        let buffer: AlignedBuffer<f64> = AlignedBuffer::with_capacity(1000);
        assert_eq!(buffer.len(), 1000);
        assert_eq!(buffer.capacity(), 1000);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_aligned_buffer_empty() {
        let buffer: AlignedBuffer<f64> = AlignedBuffer::with_capacity(0);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_aligned_buffer_64_byte_alignment() {
        let buffer: AlignedBuffer<f64> = AlignedBuffer::with_capacity(1000);
        assert!(buffer.is_aligned());
        assert_eq!(buffer.alignment(), 64);

        // Verify pointer alignment directly
        let ptr_addr = buffer.as_ptr() as usize;
        assert_eq!(ptr_addr % 64, 0, "Buffer must be 64-byte aligned");
    }

    #[test]
    fn test_aligned_buffer_i32_alignment() {
        let buffer: AlignedBuffer<i32> = AlignedBuffer::with_capacity(1000);
        assert!(buffer.is_aligned());

        let ptr_addr = buffer.as_ptr() as usize;
        assert_eq!(ptr_addr % 64, 0, "i32 buffer must be 64-byte aligned");
    }

    #[test]
    fn test_aligned_buffer_from_vec() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let buffer: AlignedBuffer<f64> = AlignedBuffer::from_vec(data);

        assert_eq!(buffer.len(), 5);
        assert!(buffer.is_aligned());
        assert_eq!(buffer[0], 1.0);
        assert_eq!(buffer[4], 5.0);
    }

    #[test]
    fn test_aligned_buffer_from_slice() {
        let data = [1.0, 2.0, 3.0];
        let buffer: AlignedBuffer<f64> = AlignedBuffer::from_slice(&data);

        assert_eq!(buffer.len(), 3);
        assert!(buffer.is_aligned());
        assert_eq!(buffer[0], 1.0);
        assert_eq!(buffer[2], 3.0);
    }

    #[test]
    fn test_aligned_buffer_default_values() {
        let buffer: AlignedBuffer<f64> = AlignedBuffer::with_capacity(10);

        // All elements should be initialised to 0.0 (f64::default())
        for i in 0..10 {
            assert_eq!(buffer[i], 0.0);
        }
    }

    #[test]
    fn test_aligned_buffer_deref_slice() {
        let buffer: AlignedBuffer<f64> = AlignedBuffer::from_vec(vec![1.0, 2.0, 3.0]);
        let slice: &[f64] = &buffer;

        assert_eq!(slice.len(), 3);
        assert_eq!(slice[0], 1.0);
    }

    #[test]
    fn test_aligned_buffer_deref_mut() {
        let mut buffer: AlignedBuffer<f64> = AlignedBuffer::with_capacity(10);
        buffer[0] = 42.0;
        buffer[9] = 99.0;

        assert_eq!(buffer[0], 42.0);
        assert_eq!(buffer[9], 99.0);
    }

    #[test]
    fn test_aligned_buffer_clone() {
        let mut original: AlignedBuffer<f64> = AlignedBuffer::with_capacity(5);
        original[0] = 1.0;
        original[4] = 5.0;

        let cloned = original.clone();

        assert_eq!(cloned.len(), 5);
        assert!(cloned.is_aligned());
        assert_eq!(cloned[0], 1.0);
        assert_eq!(cloned[4], 5.0);
    }

    #[test]
    fn test_aligned_buffer_debug() {
        let buffer: AlignedBuffer<f64> = AlignedBuffer::from_vec(vec![1.0, 2.0]);
        let debug_str = format!("{:?}", buffer);
        assert!(debug_str.contains("1.0"));
        assert!(debug_str.contains("2.0"));
    }

    #[test]
    fn test_aligned_buffer_iter() {
        let buffer: AlignedBuffer<f64> = AlignedBuffer::from_vec(vec![1.0, 2.0, 3.0]);
        let sum: f64 = buffer.iter().sum();
        assert_eq!(sum, 6.0);
    }

    #[test]
    fn test_aligned_buffer_iter_mut() {
        let mut buffer: AlignedBuffer<f64> = AlignedBuffer::with_capacity(3);
        for (i, elem) in buffer.iter_mut().enumerate() {
            *elem = (i + 1) as f64;
        }

        assert_eq!(buffer[0], 1.0);
        assert_eq!(buffer[1], 2.0);
        assert_eq!(buffer[2], 3.0);
    }

    #[test]
    fn test_aligned_buffer_memory_usage() {
        let buffer: AlignedBuffer<f64> = AlignedBuffer::with_capacity(1000);
        let expected = 1000 * std::mem::size_of::<f64>();
        assert_eq!(buffer.memory_usage(), expected);
    }

    #[test]
    fn test_aligned_buffer_large_capacity() {
        // Test with 1 million elements to verify scaling
        let buffer: AlignedBuffer<f64> = AlignedBuffer::with_capacity(1_000_000);
        assert_eq!(buffer.len(), 1_000_000);
        assert!(buffer.is_aligned());
    }

    #[test]
    fn test_aligned_buffer_equality() {
        let a: AlignedBuffer<f64> = AlignedBuffer::from_vec(vec![1.0, 2.0, 3.0]);
        let b: AlignedBuffer<f64> = AlignedBuffer::from_vec(vec![1.0, 2.0, 3.0]);
        let c: AlignedBuffer<f64> = AlignedBuffer::from_vec(vec![1.0, 2.0, 4.0]);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_aligned_buffer_as_slice() {
        let buffer: AlignedBuffer<f64> = AlignedBuffer::from_vec(vec![1.0, 2.0, 3.0]);
        let slice = buffer.as_slice();

        assert_eq!(slice.len(), 3);
        assert_eq!(slice[0], 1.0);
    }

    #[test]
    fn test_aligned_buffer_as_mut_slice() {
        let mut buffer: AlignedBuffer<f64> = AlignedBuffer::with_capacity(3);
        let slice = buffer.as_mut_slice();
        slice[0] = 1.0;
        slice[1] = 2.0;
        slice[2] = 3.0;

        assert_eq!(buffer[0], 1.0);
        assert_eq!(buffer[1], 2.0);
        assert_eq!(buffer[2], 3.0);
    }

    #[test]
    fn test_aligned_buffer_ptr() {
        let buffer: AlignedBuffer<f64> = AlignedBuffer::with_capacity(100);
        let ptr = buffer.as_ptr();

        // Pointer should be 64-byte aligned
        assert_eq!(ptr as usize % 64, 0);
    }

    #[test]
    fn test_aligned_buffer_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<AlignedBuffer<f64>>();
        assert_sync::<AlignedBuffer<f64>>();
    }

    #[test]
    fn test_aligned_buffer_lower_6_bits_zero() {
        // 64-byte alignment requires lower 6 bits of address to be zero
        // (64 = 2^6, so address & 0x3F should equal 0)
        let buffer: AlignedBuffer<f64> = AlignedBuffer::with_capacity(1000);
        let ptr_addr = buffer.as_ptr() as usize;

        // Explicit bit-level check per spec requirement 11.1
        assert_eq!(
            ptr_addr & 0x3F,
            0,
            "Lower 6 bits must be 0 for 64-byte alignment. Got addr: {:#x}",
            ptr_addr
        );
    }

    #[test]
    fn test_aligned_buffer_multiple_buffers_alignment() {
        // Verify multiple independent buffers are all aligned
        let buffers: Vec<AlignedBuffer<f64>> =
            (0..10).map(|_| AlignedBuffer::with_capacity(100)).collect();

        for (i, buf) in buffers.iter().enumerate() {
            let ptr_addr = buf.as_ptr() as usize;
            assert_eq!(
                ptr_addr & 0x3F,
                0,
                "Buffer {} must be 64-byte aligned. Got addr: {:#x}",
                i,
                ptr_addr
            );
        }
    }

    #[test]
    fn test_aligned_buffer_different_sizes_alignment() {
        // Verify alignment is maintained regardless of buffer size
        for size in [1, 7, 16, 64, 100, 1000, 10000] {
            let buffer: AlignedBuffer<f64> = AlignedBuffer::with_capacity(size);
            let ptr_addr = buffer.as_ptr() as usize;
            assert_eq!(
                ptr_addr & 0x3F,
                0,
                "Buffer of size {} must be 64-byte aligned. Got addr: {:#x}",
                size,
                ptr_addr
            );
        }
    }
}
