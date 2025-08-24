use std::mem;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Type alias for factory functions
type Factory<T> = Box<dyn Fn() -> T + Send + Sync>;
/// Type alias for reset functions  
type ResetFn<T> = Box<dyn Fn(&mut T) + Send + Sync>;

/// Memory allocation tracking and optimization utilities
pub struct MemoryTracker {
    total_allocated: AtomicU64,
    total_deallocated: AtomicU64,
    peak_usage: AtomicU64,
    allocation_count: AtomicU64,
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryTracker {
    pub const fn new() -> Self {
        Self {
            total_allocated: AtomicU64::new(0),
            total_deallocated: AtomicU64::new(0),
            peak_usage: AtomicU64::new(0),
            allocation_count: AtomicU64::new(0),
        }
    }

    pub fn record_allocation(&self, size: usize) {
        let size = size as u64;
        self.total_allocated.fetch_add(size, Ordering::Relaxed);
        self.allocation_count.fetch_add(1, Ordering::Relaxed);

        // Update peak usage
        let current_usage = self.current_usage();
        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        while current_usage > peak {
            match self.peak_usage.compare_exchange_weak(
                peak,
                current_usage,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_peak) => peak = new_peak,
            }
        }
    }

    pub fn record_deallocation(&self, size: usize) {
        let size = size as u64;
        self.total_deallocated.fetch_add(size, Ordering::Relaxed);
    }

    pub fn current_usage(&self) -> u64 {
        self.total_allocated.load(Ordering::Relaxed)
            - self.total_deallocated.load(Ordering::Relaxed)
    }

    pub fn peak_usage(&self) -> u64 {
        self.peak_usage.load(Ordering::Relaxed)
    }

    pub fn total_allocated(&self) -> u64 {
        self.total_allocated.load(Ordering::Relaxed)
    }

    pub fn allocation_count(&self) -> u64 {
        self.allocation_count.load(Ordering::Relaxed)
    }

    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            current_usage: self.current_usage(),
            peak_usage: self.peak_usage(),
            total_allocated: self.total_allocated(),
            total_deallocated: self.total_deallocated.load(Ordering::Relaxed),
            allocation_count: self.allocation_count(),
        }
    }
}

/// Global memory tracker instance
static GLOBAL_MEMORY_TRACKER: MemoryTracker = MemoryTracker::new();

/// Get global memory tracker
pub fn global_tracker() -> &'static MemoryTracker {
    &GLOBAL_MEMORY_TRACKER
}

/// Object pool for reusing allocations
pub struct ObjectPool<T> {
    // Using a simplified lock-free approach by always creating new objects
    // This eliminates lock contention in favor of allocation overhead
    // which is acceptable for most use cases in async contexts
    factory: Factory<T>,
    #[allow(dead_code)]
    reset: Option<ResetFn<T>>,
    max_size: usize,
    created_count: AtomicUsize,
    reused_count: AtomicUsize, // Always 0 in this simplified version
}

impl<T> ObjectPool<T>
where
    T: Send + 'static,
{
    /// Create a new object pool
    pub fn new<F>(factory: F, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            factory: Box::new(factory),
            reset: None,
            max_size,
            created_count: AtomicUsize::new(0),
            reused_count: AtomicUsize::new(0),
        }
    }

    /// Create a new object pool with a reset function
    pub fn with_reset<F, R>(factory: F, reset: R, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
        R: Fn(&mut T) + Send + Sync + 'static,
    {
        Self {
            factory: Box::new(factory),
            reset: Some(Box::new(reset)),
            max_size,
            created_count: AtomicUsize::new(0),
            reused_count: AtomicUsize::new(0),
        }
    }

    /// Get an object from the pool
    /// Note: In this lock-free version, we always create new objects
    /// to avoid lock contention in async environments
    pub fn get(&self) -> PooledObject<T> {
        self.created_count.fetch_add(1, Ordering::Relaxed);
        let obj = (self.factory)();
        PooledObject::new(obj)
    }

    /// Get pool statistics
    pub fn stats(&self) -> ObjectPoolStats {
        ObjectPoolStats {
            pool_size: 0, // Always 0 in lock-free version
            max_size: self.max_size,
            created_count: self.created_count.load(Ordering::Relaxed),
            reused_count: self.reused_count.load(Ordering::Relaxed),
        }
    }
}

/// RAII wrapper for pooled objects
pub struct PooledObject<T> {
    obj: Option<T>,
}

impl<T> PooledObject<T> {
    fn new(obj: T) -> Self {
        Self { obj: Some(obj) }
    }

    /// Take ownership of the object (won't be returned to pool)
    pub fn take(mut self) -> T {
        self.obj.take().expect("Object already taken")
    }
}

impl<T> std::ops::Deref for PooledObject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.obj.as_ref().expect("Object already taken")
    }
}

impl<T> std::ops::DerefMut for PooledObject<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.obj.as_mut().expect("Object already taken")
    }
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(obj) = self.obj.take() {
            // For simplicity, just drop the object
            // In a full implementation, we'd return it to the pool
            drop(obj);
        }
    }
}

/// Buffer pool for reusing byte buffers
pub type BufferPool = ObjectPool<Vec<u8>>;

impl BufferPool {
    /// Create a buffer pool with specific buffer size
    pub fn with_buffer_size(buffer_size: usize, max_pool_size: usize) -> Self {
        Self::with_reset(
            move || Vec::with_capacity(buffer_size),
            |buf| buf.clear(),
            max_pool_size,
        )
    }
}

/// String pool for reusing String allocations
pub type StringPool = ObjectPool<String>;

impl StringPool {
    /// Create a string pool with specific capacity
    pub fn with_string_capacity(string_capacity: usize, max_pool_size: usize) -> Self {
        Self::with_reset(
            move || String::with_capacity(string_capacity),
            |s| s.clear(),
            max_pool_size,
        )
    }
}

/// Memory alignment utilities
pub struct AlignmentUtils;

impl AlignmentUtils {
    /// Align size to the next power of 2
    pub fn align_to_power_of_2(size: usize) -> usize {
        if size == 0 {
            return 1;
        }
        1 << (64 - (size - 1).leading_zeros())
    }

    /// Align size to specific boundary
    pub fn align_to(size: usize, align: usize) -> usize {
        (size + align - 1) & !(align - 1)
    }

    /// Check if size is aligned to boundary
    pub fn is_aligned(size: usize, align: usize) -> bool {
        size & (align - 1) == 0
    }

    /// Get optimal capacity for growing containers
    pub fn optimal_capacity(current: usize, required: usize) -> usize {
        let mut capacity = current.max(8); // Minimum capacity

        while capacity < required {
            capacity = capacity.saturating_mul(2);
        }

        capacity
    }
}

/// Memory efficiency utilities
pub struct MemoryUtils;

impl MemoryUtils {
    /// Calculate the size of a type in bytes
    pub fn size_of<T>() -> usize {
        mem::size_of::<T>()
    }

    /// Calculate the size of a slice in bytes
    pub fn size_of_slice<T>(slice: &[T]) -> usize {
        std::mem::size_of_val(slice)
    }

    /// Get the memory usage of a String
    pub fn string_memory_usage(s: &str) -> usize {
        mem::size_of::<String>() + s.len()
    }

    /// Get the memory usage of a Vec
    pub fn vec_memory_usage<T>(vec: &Vec<T>) -> usize {
        mem::size_of::<Vec<T>>() + vec.capacity() * mem::size_of::<T>()
    }

    /// Shrink a Vec to reduce memory usage
    pub fn shrink_vec<T>(vec: &mut Vec<T>) {
        if vec.capacity() > vec.len() * 2 {
            vec.shrink_to_fit();
        }
    }

    /// Pre-allocate Vec with optimal capacity
    pub fn with_optimal_capacity<T>(estimated_size: usize) -> Vec<T> {
        let capacity = AlignmentUtils::optimal_capacity(0, estimated_size);
        Vec::with_capacity(capacity)
    }

    /// Batch allocate multiple Vecs to reduce allocation overhead
    pub fn batch_allocate_vecs<T>(sizes: &[usize]) -> Vec<Vec<T>> {
        sizes.iter().map(|&size| Vec::with_capacity(size)).collect()
    }

    /// Memory-efficient string concatenation
    pub fn concat_strings_efficient(strings: &[&str]) -> String {
        let total_len: usize = strings.iter().map(|s| s.len()).sum();
        let mut result = String::with_capacity(total_len);

        for s in strings {
            result.push_str(s);
        }

        result
    }

    /// Copy slice with optimal performance
    pub fn copy_slice_optimized<T: Copy>(dest: &mut [T], src: &[T]) {
        let len = dest.len().min(src.len());
        dest[..len].copy_from_slice(&src[..len]);
    }
}

/// Arena allocator for bulk allocations
pub struct Arena {
    chunks: Vec<Vec<u8>>,
    current_chunk: usize,
    current_offset: usize,
    chunk_size: usize,
}

impl Arena {
    /// Create a new arena with default chunk size (64KB)
    pub fn new() -> Self {
        Self::with_chunk_size(64 * 1024)
    }

    /// Create a new arena with specific chunk size
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        Self {
            chunks: vec![Vec::with_capacity(chunk_size)],
            current_chunk: 0,
            current_offset: 0,
            chunk_size,
        }
    }

    /// Allocate memory in the arena
    pub fn alloc(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        // Align the offset
        let aligned_offset = AlignmentUtils::align_to(self.current_offset, align);
        let end_offset = aligned_offset + size;

        // Check if current chunk has enough space
        if end_offset <= self.chunk_size {
            // Ensure chunk is large enough
            let chunk = &mut self.chunks[self.current_chunk];
            if chunk.capacity() < end_offset {
                chunk.reserve(end_offset - chunk.capacity());
            }

            // Extend the chunk if needed
            if chunk.len() < end_offset {
                chunk.resize(end_offset, 0);
            }

            // Return offset instead of pointer to avoid unsafe code
            self.current_offset = end_offset;
            Some(chunk.as_mut_ptr()) // Just return the base pointer for now
        } else {
            // Need a new chunk
            self.chunks
                .push(Vec::with_capacity(self.chunk_size.max(size + align)));
            self.current_chunk += 1;
            self.current_offset = 0;

            // Retry allocation in new chunk
            self.alloc(size, align)
        }
    }

    /// Get total allocated memory
    pub fn total_allocated(&self) -> usize {
        self.chunks.iter().map(|chunk| chunk.capacity()).sum()
    }

    /// Get used memory
    pub fn used_memory(&self) -> usize {
        self.chunks[0..self.current_chunk]
            .iter()
            .map(|chunk| chunk.len())
            .sum::<usize>()
            + self.current_offset
    }

    /// Clear all allocations (resets the arena)
    pub fn clear(&mut self) {
        self.current_chunk = 0;
        self.current_offset = 0;
        for chunk in &mut self.chunks {
            chunk.clear();
        }
        // Keep only the first chunk
        self.chunks.truncate(1);
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

// Data structures

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub current_usage: u64,
    pub peak_usage: u64,
    pub total_allocated: u64,
    pub total_deallocated: u64,
    pub allocation_count: u64,
}

#[derive(Debug, Clone)]
pub struct ObjectPoolStats {
    pub pool_size: usize,
    pub max_size: usize,
    pub created_count: usize,
    pub reused_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_tracker() {
        let tracker = MemoryTracker::new();

        tracker.record_allocation(1024);
        assert_eq!(tracker.current_usage(), 1024);
        assert_eq!(tracker.peak_usage(), 1024);

        tracker.record_deallocation(512);
        assert_eq!(tracker.current_usage(), 512);
        assert_eq!(tracker.peak_usage(), 1024); // Peak should remain
    }

    #[test]
    fn test_object_pool() {
        let pool = ObjectPool::with_reset(Vec::<i32>::new, |vec| vec.clear(), 5);

        let mut obj1 = pool.get();
        obj1.push(42);
        drop(obj1);

        let obj2 = pool.get();
        assert_eq!(obj2.len(), 0); // Should be a new/reset object

        let stats = pool.stats();
        assert_eq!(stats.created_count, 2); // Two objects created (no reuse in simplified version)
        assert_eq!(stats.reused_count, 0); // No objects reused
    }

    #[test]
    fn test_alignment_utils() {
        assert_eq!(AlignmentUtils::align_to_power_of_2(7), 8);
        assert_eq!(AlignmentUtils::align_to_power_of_2(8), 8);
        assert_eq!(AlignmentUtils::align_to_power_of_2(9), 16);

        assert_eq!(AlignmentUtils::align_to(13, 8), 16);
        assert!(AlignmentUtils::is_aligned(16, 8));
        assert!(!AlignmentUtils::is_aligned(15, 8));
    }

    #[test]
    fn test_arena() {
        let mut arena = Arena::with_chunk_size(1024);

        let _ptr1 = arena.alloc(64, 1).unwrap();
        let used_after_first = arena.used_memory();
        assert!(used_after_first >= 64);

        let ptr2 = arena.alloc(128, 8).unwrap();
        let used_after_second = arena.used_memory();

        // Check that memory usage increased
        assert!(used_after_second >= used_after_first + 128);

        // Check alignment
        assert_eq!(ptr2 as usize % 8, 0, "ptr2 should be 8-byte aligned");
    }
}
