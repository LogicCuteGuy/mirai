//! Enhanced memory management system for the unified mirai server
//!
//! This module combines the object pooling from minecraft-server-core with mirai's
//! existing recycling system to provide comprehensive memory management optimizations.

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Enhanced object pool that combines features from both systems
pub struct EnhancedObjectPool<T> {
    pool: Mutex<VecDeque<T>>,
    factory: Box<dyn Fn() -> T + Send + Sync>,
    max_size: usize,
    created_count: AtomicUsize,
    reused_count: AtomicUsize,
    name: String,
}

impl<T> EnhancedObjectPool<T> {
    /// Create a new enhanced object pool with monitoring
    pub fn new<F>(name: String, factory: F, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            pool: Mutex::new(VecDeque::with_capacity(max_size)),
            factory: Box::new(factory),
            max_size,
            created_count: AtomicUsize::new(0),
            reused_count: AtomicUsize::new(0),
            name,
        }
    }

    /// Get an object from the pool, creating a new one if the pool is empty
    pub fn get(self: &Arc<Self>) -> EnhancedPooledObject<T> {
        let mut pool = self.pool.lock();

        if let Some(obj) = pool.pop_front() {
            self.reused_count.fetch_add(1, Ordering::Relaxed);
            debug!("Reused object from pool '{}'", self.name);
            EnhancedPooledObject::new(obj, Arc::downgrade(self))
        } else {
            drop(pool); // Release lock before creating new object
            let obj = (self.factory)();
            self.created_count.fetch_add(1, Ordering::Relaxed);
            debug!("Created new object for pool '{}'", self.name);
            EnhancedPooledObject::new(obj, Arc::downgrade(self))
        }
    }

    /// Return an object to the pool
    fn return_object(&self, obj: T) {
        let mut pool = self.pool.lock();
        if pool.len() < self.max_size {
            pool.push_back(obj);
            debug!("Returned object to pool '{}'", self.name);
        } else {
            debug!("Pool '{}' full, dropping object", self.name);
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> EnhancedPoolStats {
        let pool = self.pool.lock();
        EnhancedPoolStats {
            name: self.name.clone(),
            pool_size: pool.len(),
            max_size: self.max_size,
            created_count: self.created_count.load(Ordering::Relaxed),
            reused_count: self.reused_count.load(Ordering::Relaxed),
        }
    }

    /// Clear the pool
    pub fn clear(&self) {
        let mut pool = self.pool.lock();
        pool.clear();
        info!("Cleared enhanced object pool '{}'", self.name);
    }

    /// Get pool name
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Enhanced statistics for object pools
#[derive(Debug, Clone)]
pub struct EnhancedPoolStats {
    pub name: String,
    pub pool_size: usize,
    pub max_size: usize,
    pub created_count: usize,
    pub reused_count: usize,
}

impl EnhancedPoolStats {
    /// Calculate the reuse ratio (0.0 to 1.0)
    pub fn reuse_ratio(&self) -> f64 {
        let total = self.created_count + self.reused_count;
        if total == 0 {
            0.0
        } else {
            self.reused_count as f64 / total as f64
        }
    }

    /// Calculate pool utilization (0.0 to 1.0)
    pub fn utilization(&self) -> f64 {
        if self.max_size == 0 {
            0.0
        } else {
            self.pool_size as f64 / self.max_size as f64
        }
    }
}

/// A pooled object that automatically returns to the pool when dropped
pub struct EnhancedPooledObject<T> {
    obj: Option<T>,
    pool: std::sync::Weak<EnhancedObjectPool<T>>,
}

impl<T> EnhancedPooledObject<T> {
    fn new(obj: T, pool: std::sync::Weak<EnhancedObjectPool<T>>) -> Self {
        Self {
            obj: Some(obj),
            pool,
        }
    }

    /// Get a reference to the pooled object
    pub fn get(&self) -> &T {
        self.obj.as_ref().expect("EnhancedPooledObject accessed after drop")
    }

    /// Get a mutable reference to the pooled object
    pub fn get_mut(&mut self) -> &mut T {
        self.obj.as_mut().expect("EnhancedPooledObject accessed after drop")
    }
}

impl<T> Drop for EnhancedPooledObject<T> {
    fn drop(&mut self) {
        if let Some(obj) = self.obj.take() {
            if let Some(pool) = self.pool.upgrade() {
                pool.return_object(obj);
            }
        }
    }
}

impl<T> std::ops::Deref for EnhancedPooledObject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> std::ops::DerefMut for EnhancedPooledObject<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

/// Enhanced memory usage monitor with leak detection and profiling
#[derive(Debug)]
pub struct EnhancedMemoryMonitor {
    allocations: AtomicUsize,
    deallocations: AtomicUsize,
    peak_usage: AtomicUsize,
    current_usage: AtomicUsize,
    leak_threshold: usize,
    name: String,
}

impl EnhancedMemoryMonitor {
    /// Create a new enhanced memory monitor
    pub fn new(name: String, leak_threshold: usize) -> Self {
        Self {
            allocations: AtomicUsize::new(0),
            deallocations: AtomicUsize::new(0),
            peak_usage: AtomicUsize::new(0),
            current_usage: AtomicUsize::new(0),
            leak_threshold,
            name,
        }
    }

    /// Record an allocation
    pub fn record_allocation(&self, size: usize) {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        let current = self.current_usage.fetch_add(size, Ordering::Relaxed) + size;

        // Update peak usage if necessary
        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_usage.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_peak) => peak = new_peak,
            }
        }
    }

    /// Record a deallocation
    pub fn record_deallocation(&self, size: usize) {
        self.deallocations.fetch_add(1, Ordering::Relaxed);
        self.current_usage.fetch_sub(size, Ordering::Relaxed);
    }

    /// Get current memory statistics
    pub fn stats(&self) -> EnhancedMemoryStats {
        EnhancedMemoryStats {
            name: self.name.clone(),
            allocations: self.allocations.load(Ordering::Relaxed),
            deallocations: self.deallocations.load(Ordering::Relaxed),
            current_usage: self.current_usage.load(Ordering::Relaxed),
            peak_usage: self.peak_usage.load(Ordering::Relaxed),
            leak_threshold: self.leak_threshold,
        }
    }

    /// Check for potential memory leaks
    pub fn check_leaks(&self) -> bool {
        let stats = self.stats();
        let leaked_allocations = stats.leaked_allocations();

        if leaked_allocations > self.leak_threshold {
            warn!(
                "Potential memory leak detected in '{}': {} unmatched allocations, current usage: {}",
                self.name, leaked_allocations, format_bytes(stats.current_usage)
            );
            true
        } else {
            false
        }
    }

    /// Reset statistics
    pub fn reset(&self) {
        self.allocations.store(0, Ordering::Relaxed);
        self.deallocations.store(0, Ordering::Relaxed);
        self.current_usage.store(0, Ordering::Relaxed);
        self.peak_usage.store(0, Ordering::Relaxed);
        info!("Reset memory monitor '{}'", self.name);
    }
}

/// Enhanced memory usage statistics
#[derive(Debug, Clone)]
pub struct EnhancedMemoryStats {
    pub name: String,
    pub allocations: usize,
    pub deallocations: usize,
    pub current_usage: usize,
    pub peak_usage: usize,
    pub leak_threshold: usize,
}

impl EnhancedMemoryStats {
    /// Get the number of unmatched allocations (potential leaks)
    pub fn leaked_allocations(&self) -> usize {
        self.allocations.saturating_sub(self.deallocations)
    }

    /// Format memory usage in human-readable form
    pub fn format_usage(&self) -> String {
        format_bytes(self.current_usage)
    }

    /// Format peak usage in human-readable form
    pub fn format_peak(&self) -> String {
        format_bytes(self.peak_usage)
    }

    /// Check if leak threshold is exceeded
    pub fn has_leaks(&self) -> bool {
        self.leaked_allocations() > self.leak_threshold
    }

    /// Calculate memory efficiency (deallocations / allocations)
    pub fn efficiency(&self) -> f64 {
        if self.allocations == 0 {
            1.0
        } else {
            self.deallocations as f64 / self.allocations as f64
        }
    }
}

/// Format bytes in human-readable form
pub fn format_bytes(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Trait for objects that can be reset for reuse in enhanced pools
pub trait EnhancedPoolable {
    /// Reset the object to its initial state for reuse
    fn reset(&mut self);
    
    /// Get the estimated memory size of this object
    fn memory_size(&self) -> usize {
        std::mem::size_of_val(self)
    }
}

impl EnhancedPoolable for Vec<u8> {
    fn reset(&mut self) {
        self.clear();
    }

    fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>() + (self.capacity() * std::mem::size_of::<u8>())
    }
}

impl EnhancedPoolable for String {
    fn reset(&mut self) {
        self.clear();
    }

    fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>() + self.capacity()
    }
}

/// Enhanced object pool that automatically resets objects and tracks memory usage
pub struct EnhancedResettingPool<T: EnhancedPoolable> {
    inner: Arc<EnhancedObjectPool<T>>,
    monitor: Arc<EnhancedMemoryMonitor>,
}

impl<T: EnhancedPoolable> EnhancedResettingPool<T> {
    /// Create a new enhanced resetting pool
    pub fn new<F>(name: String, factory: F, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        let monitor = Arc::new(EnhancedMemoryMonitor::new(
            format!("{}_monitor", name),
            100, // Default leak threshold
        ));
        
        Self {
            inner: Arc::new(EnhancedObjectPool::new(name, factory, max_size)),
            monitor,
        }
    }

    /// Get an object from the pool, automatically reset and monitored
    pub fn get(&self) -> EnhancedResettingPooledObject<T> {
        let mut obj = self.inner.get();
        let size = obj.memory_size();
        obj.get_mut().reset();
        self.monitor.record_allocation(size);
        
        EnhancedResettingPooledObject { 
            inner: obj,
            monitor: Arc::clone(&self.monitor),
            size,
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> EnhancedPoolStats {
        self.inner.stats()
    }

    /// Get memory statistics
    pub fn memory_stats(&self) -> EnhancedMemoryStats {
        self.monitor.stats()
    }

    /// Clear the pool
    pub fn clear(&self) {
        self.inner.clear();
        self.monitor.reset();
    }

    /// Check for memory leaks
    pub fn check_leaks(&self) -> bool {
        self.monitor.check_leaks()
    }
}

/// A pooled object that is automatically reset when retrieved and tracks memory usage
pub struct EnhancedResettingPooledObject<T: EnhancedPoolable> {
    inner: EnhancedPooledObject<T>,
    monitor: Arc<EnhancedMemoryMonitor>,
    size: usize,
}

impl<T: EnhancedPoolable> Drop for EnhancedResettingPooledObject<T> {
    fn drop(&mut self) {
        self.monitor.record_deallocation(self.size);
    }
}

impl<T: EnhancedPoolable> std::ops::Deref for EnhancedResettingPooledObject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: EnhancedPoolable> std::ops::DerefMut for EnhancedResettingPooledObject<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_object_pool_basic() {
        let pool = Arc::new(EnhancedObjectPool::new(
            "test_pool".to_string(),
            || Vec::<i32>::new(),
            10,
        ));

        // Get an object from the pool
        let mut obj1 = pool.get();
        obj1.push(42);
        assert_eq!(obj1.len(), 1);

        // Drop the object (returns to pool)
        drop(obj1);

        // Get another object - should be reused
        let obj2 = pool.get();
        assert_eq!(obj2.len(), 1); // Should contain previous data

        let stats = pool.stats();
        assert_eq!(stats.created_count, 1);
        assert_eq!(stats.reused_count, 1);
        assert_eq!(stats.name, "test_pool");
    }

    #[test]
    fn test_enhanced_memory_monitor() {
        let monitor = EnhancedMemoryMonitor::new("test_monitor".to_string(), 50);

        // Record some allocations
        monitor.record_allocation(1024);
        monitor.record_allocation(2048);

        let stats = monitor.stats();
        assert_eq!(stats.allocations, 2);
        assert_eq!(stats.current_usage, 3072);
        assert_eq!(stats.peak_usage, 3072);
        assert_eq!(stats.name, "test_monitor");

        // Record a deallocation
        monitor.record_deallocation(1024);

        let stats = monitor.stats();
        assert_eq!(stats.deallocations, 1);
        assert_eq!(stats.current_usage, 2048);
        assert_eq!(stats.peak_usage, 3072); // Peak should remain
    }

    #[test]
    fn test_enhanced_resetting_pool() {
        let pool = EnhancedResettingPool::new(
            "test_resetting".to_string(),
            || {
                let mut v = Vec::new();
                v.push(1);
                v.push(2);
                v
            },
            5,
        );

        // Get an object - should be reset
        let obj = pool.get();
        assert_eq!(obj.len(), 0); // Should be reset

        let stats = pool.stats();
        assert_eq!(stats.name, "test_resetting");
        
        let mem_stats = pool.memory_stats();
        assert_eq!(mem_stats.allocations, 1);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
    }

    #[test]
    fn test_enhanced_pool_stats() {
        let stats = EnhancedPoolStats {
            name: "test".to_string(),
            pool_size: 5,
            max_size: 10,
            created_count: 3,
            reused_count: 7,
        };

        assert_eq!(stats.reuse_ratio(), 0.7);
        assert_eq!(stats.utilization(), 0.5);
    }
}