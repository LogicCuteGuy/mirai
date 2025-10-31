//! Enhanced threading and concurrency system for the unified mirai server
//!
//! This module combines the work-stealing thread pool from minecraft-server-core
//! with mirai's existing tokio-based async architecture to provide optimal
//! performance for both CPU-bound and I/O-bound operations.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use crossbeam::deque::{Injector, Stealer, Worker};
use crossbeam::channel::{self, Receiver, Sender};
use parking_lot::Mutex;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, warn, error};

/// Enhanced thread manager that combines work-stealing with async execution
pub struct EnhancedThreadManager {
    cpu_pool: WorkStealingThreadPool,
    async_runtime: tokio::runtime::Handle,
    scheduler: EnhancedScheduler,
    stats: Arc<ThreadingStats>,
}

impl EnhancedThreadManager {
    /// Create a new enhanced thread manager
    pub fn new() -> Self {
        let num_cpu_threads = num_cpus::get().max(2);
        let cpu_pool = WorkStealingThreadPool::new(num_cpu_threads);
        
        // Use the current tokio runtime or create a new one
        let async_runtime = tokio::runtime::Handle::try_current()
            .unwrap_or_else(|_| {
                tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime")
                    .handle()
                    .clone()
            });

        let scheduler = EnhancedScheduler::new(num_cpu_threads);
        let stats = Arc::new(ThreadingStats::new());

        info!("Enhanced thread manager initialized with {} CPU threads", num_cpu_threads);

        Self {
            cpu_pool,
            async_runtime,
            scheduler,
            stats,
        }
    }

    /// Execute a CPU-intensive task on the work-stealing thread pool
    pub fn execute_cpu_task<F, R>(&self, task: F) -> Result<CpuTaskHandle<R>, ThreadingError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();
        
        self.cpu_pool.submit(move || {
            let result = task();
            let _ = sender.send(result);
        })?;

        self.stats.cpu_tasks_submitted.fetch_add(1, Ordering::Relaxed);
        Ok(CpuTaskHandle { receiver })
    }

    /// Execute an async task on the tokio runtime
    pub fn execute_async_task<F, Fut, R>(&self, task: F) -> AsyncTaskHandle<R>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = R> + Send + 'static,
        R: Send + 'static,
    {
        let handle = self.async_runtime.spawn(async move {
            task().await
        });

        self.stats.async_tasks_submitted.fetch_add(1, Ordering::Relaxed);
        AsyncTaskHandle { handle }
    }

    /// Execute a hybrid task that combines CPU and async work
    pub fn execute_hybrid_task<F, Fut, R>(&self, task: F) -> Result<AsyncTaskHandle<R>, ThreadingError>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = R> + Send + 'static,
        R: Send + 'static,
    {
        let cpu_pool = self.cpu_pool.clone();
        let (sender, receiver) = oneshot::channel();
        
        cpu_pool.submit(move || {
            let future = task();
            let _ = sender.send(future);
        })?;

        let handle = self.async_runtime.spawn(async move {
            let future = receiver.await.map_err(|_| "CPU task cancelled").unwrap_or_else(|_| panic!("CPU task cancelled"));
            future.await
        });

        self.stats.hybrid_tasks_submitted.fetch_add(1, Ordering::Relaxed);
        Ok(AsyncTaskHandle { handle })
    }

    /// Schedule a system for execution with load balancing
    pub fn schedule_system<F>(&self, system: F) -> Result<(), ThreadingError>
    where
        F: FnOnce() + Send + 'static,
    {
        let worker_id = self.scheduler.select_worker();
        debug!("Scheduling system on worker {}", worker_id);
        
        self.cpu_pool.submit_to_worker(worker_id, system)?;
        self.stats.systems_scheduled.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Schedule a high-priority system
    pub fn schedule_priority_system<F>(&self, system: F) -> Result<(), ThreadingError>
    where
        F: FnOnce() + Send + 'static,
    {
        self.cpu_pool.submit_priority(system)?;
        self.stats.priority_systems_scheduled.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Get comprehensive threading statistics
    pub fn stats(&self) -> ThreadingStats {
        let mut stats = (*self.stats).clone();
        let cpu_stats = self.cpu_pool.stats();
        let scheduler_stats = self.scheduler.stats();

        // Merge CPU pool stats
        stats.cpu_tasks_completed = cpu_stats.tasks_completed;
        stats.cpu_tasks_active = cpu_stats.active_tasks;
        stats.average_cpu_execution_time = cpu_stats.average_execution_time();

        // Merge scheduler stats
        stats.load_balance_ratio = scheduler_stats.balance_ratio();
        stats.worker_loads = scheduler_stats.worker_loads;

        stats
    }

    /// Perform maintenance tasks
    pub fn maintenance(&self) {
        self.cpu_pool.maintenance();
        self.scheduler.rebalance();
        
        // Update statistics
        let stats = self.stats();
        if stats.load_balance_ratio > 0.5 {
            warn!("Thread pool load imbalance detected: {:.2}", stats.load_balance_ratio);
        }
    }

    /// Shutdown all threading systems gracefully
    pub fn shutdown(&self) {
        info!("Shutting down enhanced thread manager");
        self.cpu_pool.shutdown();
    }
}

impl Default for EnhancedThreadManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle for a CPU-bound task
pub struct CpuTaskHandle<R> {
    receiver: oneshot::Receiver<R>,
}

impl<R> CpuTaskHandle<R> {
    /// Wait for the task to complete
    pub async fn await_result(self) -> Result<R, ThreadingError> {
        self.receiver.await.map_err(|_| ThreadingError::TaskCancelled)
    }
}

/// Handle for an async task
pub struct AsyncTaskHandle<R> {
    handle: tokio::task::JoinHandle<R>,
}

impl<R> AsyncTaskHandle<R> {
    /// Wait for the task to complete
    pub async fn await_result(self) -> Result<R, ThreadingError> {
        self.handle.await.map_err(|e| ThreadingError::ExecutionError(e.to_string()))
    }

    /// Abort the task
    pub fn abort(&self) {
        self.handle.abort();
    }
}

/// Work-stealing thread pool optimized for game systems
#[derive(Clone)]
pub struct WorkStealingThreadPool {
    inner: Arc<ThreadPoolInner>,
}

struct ThreadPoolInner {
    workers: Vec<EnhancedWorker>,
    global_queue: Arc<Injector<EnhancedTask>>,
    stealers: Vec<Stealer<EnhancedTask>>,
    shutdown: Arc<AtomicBool>,
    task_sender: Sender<EnhancedTask>,
    stats: Arc<WorkStealingStats>,
}

impl WorkStealingThreadPool {
    /// Create a new work-stealing thread pool
    pub fn new(num_threads: usize) -> Self {
        let global_queue = Arc::new(Injector::new());
        let (task_sender, task_receiver) = channel::unbounded();
        let shutdown = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(WorkStealingStats::new());
        
        let mut workers = Vec::with_capacity(num_threads);
        let mut stealers = Vec::with_capacity(num_threads);
        
        for id in 0..num_threads {
            let worker = EnhancedWorker::new(
                id,
                global_queue.clone(),
                stealers.clone(),
                shutdown.clone(),
                task_receiver.clone(),
                stats.clone(),
            );
            
            workers.push(worker);
        }
        
        let inner = Arc::new(ThreadPoolInner {
            workers,
            global_queue,
            stealers,
            shutdown,
            task_sender,
            stats,
        });

        Self { inner }
    }

    /// Submit a task to the thread pool
    pub fn submit<F>(&self, task: F) -> Result<(), ThreadingError>
    where
        F: FnOnce() + Send + 'static,
    {
        if self.inner.shutdown.load(Ordering::Relaxed) {
            return Err(ThreadingError::ShuttingDown);
        }

        let task = EnhancedTask::new(Box::new(task), false);
        
        // Try to push to global queue first
        self.inner.global_queue.push(task.clone());
        
        // Also send through channel for immediate pickup
        if let Err(_) = self.inner.task_sender.try_send(task) {
            debug!("Task channel full, task queued in global queue");
        }
        
        self.inner.stats.tasks_submitted.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Submit a high-priority task
    pub fn submit_priority<F>(&self, task: F) -> Result<(), ThreadingError>
    where
        F: FnOnce() + Send + 'static,
    {
        if self.inner.shutdown.load(Ordering::Relaxed) {
            return Err(ThreadingError::ShuttingDown);
        }

        let task = EnhancedTask::new(Box::new(task), true);
        
        // Send priority tasks directly through channel
        self.inner.task_sender.send(task)
            .map_err(|_| ThreadingError::ShuttingDown)?;
        
        self.inner.stats.priority_tasks_submitted.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Submit a task to a specific worker (best effort - may be stolen by other workers)
    pub fn submit_to_worker<F>(&self, worker_id: usize, task: F) -> Result<(), ThreadingError>
    where
        F: FnOnce() + Send + 'static,
    {
        if worker_id >= self.inner.workers.len() {
            return Err(ThreadingError::InvalidWorker(worker_id));
        }

        if self.inner.shutdown.load(Ordering::Relaxed) {
            return Err(ThreadingError::ShuttingDown);
        }

        let task = EnhancedTask::new(Box::new(task), false);
        
        // Push to global queue - the specified worker may steal it
        self.inner.global_queue.push(task);
        
        self.inner.stats.tasks_submitted.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Get thread pool statistics
    pub fn stats(&self) -> WorkStealingStatsSnapshot {
        self.inner.stats.snapshot()
    }

    /// Perform maintenance tasks
    pub fn maintenance(&self) {
        // Check for stuck workers or other issues
        let stats = self.stats();
        if stats.active_tasks > 0 && stats.tasks_completed == stats.last_completed_count {
            warn!("Potential stuck tasks detected in thread pool");
        }
    }

    /// Shutdown the thread pool gracefully
    pub fn shutdown(&self) {
        info!("Shutting down work-stealing thread pool");
        self.inner.shutdown.store(true, Ordering::Relaxed);
        
        // Wake up all workers
        for worker in &self.inner.workers {
            worker.wake();
        }
    }

    /// Get the number of worker threads
    pub fn worker_count(&self) -> usize {
        self.inner.workers.len()
    }
}

/// Enhanced worker thread with better task handling
struct EnhancedWorker {
    id: usize,
    handle: Option<JoinHandle<()>>,
}

impl EnhancedWorker {
    fn new(
        id: usize,
        global_queue: Arc<Injector<EnhancedTask>>,
        stealers: Vec<Stealer<EnhancedTask>>,
        shutdown: Arc<AtomicBool>,
        task_receiver: Receiver<EnhancedTask>,
        stats: Arc<WorkStealingStats>,
    ) -> Self {
        let handle = thread::Builder::new()
            .name(format!("enhanced-worker-{}", id))
            .spawn(move || {
                // Create the worker queue inside the thread
                let worker_queue = Arc::new(Worker::new_fifo());
                Self::worker_loop(
                    id,
                    worker_queue,
                    global_queue,
                    stealers,
                    shutdown,
                    task_receiver,
                    stats,
                );
            })
            .expect("Failed to spawn enhanced worker thread");

        Self {
            id,
            handle: Some(handle),
        }
    }

    fn worker_loop(
        worker_id: usize,
        worker_queue: Arc<Worker<EnhancedTask>>,
        global_queue: Arc<Injector<EnhancedTask>>,
        stealers: Vec<Stealer<EnhancedTask>>,
        shutdown: Arc<AtomicBool>,
        task_receiver: Receiver<EnhancedTask>,
        stats: Arc<WorkStealingStats>,
    ) {
        debug!("Enhanced worker {} started", worker_id);
        
        while !shutdown.load(Ordering::Relaxed) {
            let task = Self::find_task(
                &worker_queue,
                &global_queue,
                &stealers,
                &task_receiver,
                worker_id,
            );

            match task {
                Some(task) => {
                    stats.active_tasks.fetch_add(1, Ordering::Relaxed);
                    let start = Instant::now();
                    
                    // Execute the task
                    (task.function)();
                    
                    let duration = start.elapsed();
                    stats.tasks_completed.fetch_add(1, Ordering::Relaxed);
                    stats.total_execution_time.fetch_add(
                        duration.as_nanos() as usize,
                        Ordering::Relaxed,
                    );
                    stats.active_tasks.fetch_sub(1, Ordering::Relaxed);
                    
                    if task.priority {
                        stats.priority_tasks_completed.fetch_add(1, Ordering::Relaxed);
                    }
                    
                    debug!("Enhanced worker {} completed task in {:?}", worker_id, duration);
                }
                None => {
                    // No task found, sleep briefly
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }
        
        debug!("Enhanced worker {} shutting down", worker_id);
    }

    fn find_task(
        worker_queue: &Worker<EnhancedTask>,
        global_queue: &Arc<Injector<EnhancedTask>>,
        stealers: &[Stealer<EnhancedTask>],
        task_receiver: &Receiver<EnhancedTask>,
        worker_id: usize,
    ) -> Option<EnhancedTask> {
        // 1. Check for priority tasks from channel (non-blocking)
        if let Ok(task) = task_receiver.try_recv() {
            debug!("Enhanced worker {} got priority task from channel", worker_id);
            return Some(task);
        }

        // 2. Check local worker queue
        if let Some(task) = worker_queue.pop() {
            debug!("Enhanced worker {} got task from local queue", worker_id);
            return Some(task);
        }

        // 3. Check global queue
        match global_queue.steal() {
            crossbeam::deque::Steal::Success(task) => {
                debug!("Enhanced worker {} got task from global queue", worker_id);
                return Some(task);
            }
            _ => {}
        }

        // 4. Try to steal from other workers
        for (i, stealer) in stealers.iter().enumerate() {
            if i != worker_id {
                match stealer.steal() {
                    crossbeam::deque::Steal::Success(task) => {
                        debug!("Enhanced worker {} stole task from worker {}", worker_id, i);
                        return Some(task);
                    }
                    _ => {}
                }
            }
        }

        None
    }

    fn wake(&self) {
        if let Some(handle) = &self.handle {
            handle.thread().unpark();
        }
    }
}

/// Enhanced task with priority and metadata
#[derive(Clone)]
struct EnhancedTask {
    function: Arc<dyn Fn() + Send + Sync>,
    priority: bool,
    created_at: Instant,
}

impl EnhancedTask {
    fn new(function: Box<dyn FnOnce() + Send>, priority: bool) -> Self {
        // Convert FnOnce to Fn using a Mutex
        let function = Arc::new(Mutex::new(Some(function)));
        let function = Arc::new(move || {
            if let Some(f) = function.lock().take() {
                f();
            }
        });

        Self {
            function,
            priority,
            created_at: Instant::now(),
        }
    }
}

/// Enhanced scheduler with load balancing
pub struct EnhancedScheduler {
    worker_loads: Vec<AtomicUsize>,
    next_worker: AtomicUsize,
    rebalance_threshold: f64,
}

impl EnhancedScheduler {
    fn new(num_workers: usize) -> Self {
        Self {
            worker_loads: (0..num_workers).map(|_| AtomicUsize::new(0)).collect(),
            next_worker: AtomicUsize::new(0),
            rebalance_threshold: 0.3, // Rebalance if imbalance > 30%
        }
    }

    /// Select the best worker for the next task
    pub fn select_worker(&self) -> usize {
        // Use least-loaded worker selection
        let mut min_load = usize::MAX;
        let mut best_worker = 0;

        for (i, load) in self.worker_loads.iter().enumerate() {
            let current_load = load.load(Ordering::Relaxed);
            if current_load < min_load {
                min_load = current_load;
                best_worker = i;
            }
        }

        self.worker_loads[best_worker].fetch_add(1, Ordering::Relaxed);
        best_worker
    }

    /// Rebalance worker loads if necessary
    pub fn rebalance(&self) {
        let stats = self.stats();
        if stats.balance_ratio() > self.rebalance_threshold {
            // Reset all loads to average
            let average_load = stats.average_load() as usize;
            for load in &self.worker_loads {
                load.store(average_load, Ordering::Relaxed);
            }
            debug!("Rebalanced worker loads");
        }
    }

    /// Get scheduler statistics
    pub fn stats(&self) -> LoadBalancerStats {
        let loads: Vec<usize> = self.worker_loads
            .iter()
            .map(|load| load.load(Ordering::Relaxed))
            .collect();

        LoadBalancerStats { worker_loads: loads }
    }
}

/// Work-stealing thread pool statistics
struct WorkStealingStats {
    tasks_submitted: AtomicUsize,
    tasks_completed: AtomicUsize,
    priority_tasks_submitted: AtomicUsize,
    priority_tasks_completed: AtomicUsize,
    active_tasks: AtomicUsize,
    total_execution_time: AtomicUsize, // in nanoseconds
    last_completed_count: AtomicUsize,
}

impl WorkStealingStats {
    fn new() -> Self {
        Self {
            tasks_submitted: AtomicUsize::new(0),
            tasks_completed: AtomicUsize::new(0),
            priority_tasks_submitted: AtomicUsize::new(0),
            priority_tasks_completed: AtomicUsize::new(0),
            active_tasks: AtomicUsize::new(0),
            total_execution_time: AtomicUsize::new(0),
            last_completed_count: AtomicUsize::new(0),
        }
    }

    fn snapshot(&self) -> WorkStealingStatsSnapshot {
        let completed = self.tasks_completed.load(Ordering::Relaxed);
        self.last_completed_count.store(completed, Ordering::Relaxed);
        
        WorkStealingStatsSnapshot {
            tasks_submitted: self.tasks_submitted.load(Ordering::Relaxed),
            tasks_completed: completed,
            priority_tasks_submitted: self.priority_tasks_submitted.load(Ordering::Relaxed),
            priority_tasks_completed: self.priority_tasks_completed.load(Ordering::Relaxed),
            active_tasks: self.active_tasks.load(Ordering::Relaxed),
            total_execution_time_ns: self.total_execution_time.load(Ordering::Relaxed),
            last_completed_count: completed,
        }
    }
}

/// Snapshot of work-stealing thread pool statistics
#[derive(Debug, Clone)]
pub struct WorkStealingStatsSnapshot {
    pub tasks_submitted: usize,
    pub tasks_completed: usize,
    pub priority_tasks_submitted: usize,
    pub priority_tasks_completed: usize,
    pub active_tasks: usize,
    pub total_execution_time_ns: usize,
    pub last_completed_count: usize,
}

impl WorkStealingStatsSnapshot {
    /// Get the average task execution time
    pub fn average_execution_time(&self) -> Duration {
        if self.tasks_completed == 0 {
            Duration::ZERO
        } else {
            Duration::from_nanos((self.total_execution_time_ns / self.tasks_completed) as u64)
        }
    }

    /// Get the task completion rate (0.0 to 1.0)
    pub fn completion_rate(&self) -> f64 {
        if self.tasks_submitted == 0 {
            0.0
        } else {
            self.tasks_completed as f64 / self.tasks_submitted as f64
        }
    }

    /// Get the number of pending tasks
    pub fn pending_tasks(&self) -> usize {
        self.tasks_submitted.saturating_sub(self.tasks_completed)
    }
}

/// Load balancer statistics
#[derive(Debug, Clone)]
pub struct LoadBalancerStats {
    pub worker_loads: Vec<usize>,
}

impl LoadBalancerStats {
    /// Calculate load balance ratio (0.0 = perfectly balanced, 1.0 = completely unbalanced)
    pub fn balance_ratio(&self) -> f64 {
        if self.worker_loads.is_empty() {
            return 0.0;
        }

        let min_load = *self.worker_loads.iter().min().unwrap_or(&0);
        let max_load = *self.worker_loads.iter().max().unwrap_or(&0);

        if max_load == 0 {
            0.0
        } else {
            1.0 - (min_load as f64 / max_load as f64)
        }
    }

    /// Get the total load across all workers
    pub fn total_load(&self) -> usize {
        self.worker_loads.iter().sum()
    }

    /// Get the average load per worker
    pub fn average_load(&self) -> f64 {
        if self.worker_loads.is_empty() {
            0.0
        } else {
            self.total_load() as f64 / self.worker_loads.len() as f64
        }
    }
}

/// Comprehensive threading statistics
#[derive(Debug)]
pub struct ThreadingStats {
    // Task counts
    pub cpu_tasks_submitted: AtomicUsize,
    pub cpu_tasks_completed: usize,
    pub cpu_tasks_active: usize,
    pub async_tasks_submitted: AtomicUsize,
    pub hybrid_tasks_submitted: AtomicUsize,
    pub systems_scheduled: AtomicUsize,
    pub priority_systems_scheduled: AtomicUsize,

    // Performance metrics
    pub average_cpu_execution_time: Duration,
    pub worker_loads: Vec<usize>,
    pub load_balance_ratio: f64,
}

impl Clone for ThreadingStats {
    fn clone(&self) -> Self {
        Self {
            cpu_tasks_submitted: AtomicUsize::new(self.cpu_tasks_submitted.load(Ordering::Relaxed)),
            cpu_tasks_completed: self.cpu_tasks_completed,
            cpu_tasks_active: self.cpu_tasks_active,
            async_tasks_submitted: AtomicUsize::new(self.async_tasks_submitted.load(Ordering::Relaxed)),
            hybrid_tasks_submitted: AtomicUsize::new(self.hybrid_tasks_submitted.load(Ordering::Relaxed)),
            systems_scheduled: AtomicUsize::new(self.systems_scheduled.load(Ordering::Relaxed)),
            priority_systems_scheduled: AtomicUsize::new(self.priority_systems_scheduled.load(Ordering::Relaxed)),
            average_cpu_execution_time: self.average_cpu_execution_time,
            worker_loads: self.worker_loads.clone(),
            load_balance_ratio: self.load_balance_ratio,
        }
    }
}

impl ThreadingStats {
    fn new() -> Self {
        Self {
            cpu_tasks_submitted: AtomicUsize::new(0),
            cpu_tasks_completed: 0,
            cpu_tasks_active: 0,
            async_tasks_submitted: AtomicUsize::new(0),
            hybrid_tasks_submitted: AtomicUsize::new(0),
            systems_scheduled: AtomicUsize::new(0),
            priority_systems_scheduled: AtomicUsize::new(0),
            average_cpu_execution_time: Duration::ZERO,
            worker_loads: Vec::new(),
            load_balance_ratio: 0.0,
        }
    }

    /// Calculate thread pool utilization (0.0 to 1.0)
    pub fn utilization(&self) -> f64 {
        let total_workers = self.worker_loads.len();
        if total_workers == 0 {
            return 0.0;
        }

        let active_workers = self.worker_loads.iter().filter(|&&load| load > 0).count();
        active_workers as f64 / total_workers as f64
    }

    /// Get total number of active threads
    pub fn active_threads(&self) -> usize {
        self.cpu_tasks_active + self.worker_loads.len()
    }

    /// Get total tasks submitted across all categories
    pub fn total_tasks_submitted(&self) -> usize {
        self.cpu_tasks_submitted.load(Ordering::Relaxed) +
        self.async_tasks_submitted.load(Ordering::Relaxed) +
        self.hybrid_tasks_submitted.load(Ordering::Relaxed)
    }
}

/// Threading system errors
#[derive(Debug, thiserror::Error)]
pub enum ThreadingError {
    #[error("Threading system is shutting down")]
    ShuttingDown,
    #[error("Task was cancelled")]
    TaskCancelled,
    #[error("Invalid worker ID: {0}")]
    InvalidWorker(usize),
    #[error("Task execution failed: {0}")]
    ExecutionError(String),
    #[error("Runtime error: {0}")]
    RuntimeError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    #[test]
    fn test_work_stealing_pool_creation() {
        let pool = WorkStealingThreadPool::new(4);
        assert_eq!(pool.worker_count(), 4);
        
        let stats = pool.stats();
        assert_eq!(stats.tasks_submitted, 0);
        assert_eq!(stats.tasks_completed, 0);
        assert_eq!(stats.active_tasks, 0);
    }

    #[test]
    fn test_enhanced_scheduler() {
        let scheduler = EnhancedScheduler::new(4);
        
        // Select workers multiple times
        let mut selections = Vec::new();
        for _ in 0..10 {
            selections.push(scheduler.select_worker());
        }
        
        // Should distribute across workers
        assert!(selections.iter().any(|&w| w == 0));
        assert!(selections.len() == 10);
        
        let stats = scheduler.stats();
        assert_eq!(stats.total_load(), 10);
    }

    #[tokio::test]
    async fn test_enhanced_thread_manager() {
        let manager = EnhancedThreadManager::new();
        let counter = Arc::new(AtomicUsize::new(0));
        
        // Test CPU task
        let counter_clone = counter.clone();
        let handle = manager.execute_cpu_task(move || {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            42
        }).unwrap();
        
        let result = handle.await_result().await.unwrap();
        assert_eq!(result, 42);
        assert_eq!(counter.load(Ordering::Relaxed), 1);
        
        // Test async task
        let counter_clone = counter.clone();
        let handle = manager.execute_async_task(move || async move {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            "hello"
        });
        
        let result = handle.await_result().await.unwrap();
        assert_eq!(result, "hello");
        assert_eq!(counter.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_load_balancer_stats() {
        let stats = LoadBalancerStats {
            worker_loads: vec![10, 10, 10, 10],
        };
        
        assert_eq!(stats.balance_ratio(), 0.0); // Perfectly balanced
        assert_eq!(stats.total_load(), 40);
        assert_eq!(stats.average_load(), 10.0);
        
        let unbalanced_stats = LoadBalancerStats {
            worker_loads: vec![0, 20, 10, 5],
        };
        
        assert!(unbalanced_stats.balance_ratio() > 0.0); // Not balanced
        assert_eq!(unbalanced_stats.total_load(), 35);
    }

    #[test]
    fn test_threading_stats() {
        let stats = ThreadingStats::new();
        stats.cpu_tasks_submitted.store(100, Ordering::Relaxed);
        stats.async_tasks_submitted.store(50, Ordering::Relaxed);
        
        assert_eq!(stats.total_tasks_submitted(), 150);
    }
}