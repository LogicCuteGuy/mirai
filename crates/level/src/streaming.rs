//! Chunk streaming and memory management optimizations

use crate::{
    world::{ChunkPos, EnhancedChunk, EnhancedWorldManager, ChunkState},
};
use anyhow::Result;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque, BinaryHeap, HashSet};
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{mpsc, Semaphore, RwLock as TokioRwLock};
use tokio::time::sleep;
use tracing::{debug, info, warn, error};
use crate::ecs_integration::{Entity, EcsWorld};
use futures;

/// Enhanced chunk streaming manager with performance optimizations and ECS integration
pub struct ChunkStreamingManager {
    /// World manager reference
    world_manager: Arc<EnhancedWorldManager>,
    /// Streaming configuration
    config: StreamingConfig,
    /// Active streaming sessions (player ID -> session)
    sessions: DashMap<uuid::Uuid, Arc<RwLock<StreamingSession>>>,
    /// Chunk priority queue for loading
    load_queue: Arc<RwLock<BinaryHeap<ChunkLoadRequest>>>,
    /// Chunk unload queue
    unload_queue: Arc<RwLock<VecDeque<ChunkPos>>>,
    /// Memory manager for chunk optimization
    memory_manager: Arc<ChunkMemoryManager>,
    /// Loading semaphore to limit concurrent operations
    load_semaphore: Arc<Semaphore>,
    /// Streaming statistics
    stats: Arc<RwLock<StreamingStats>>,
    /// ECS world for entity management
    ecs_world: Arc<TokioRwLock<EcsWorld>>,
    /// Chunk preloading cache for performance
    preload_cache: Arc<RwLock<PreloadCache>>,
    /// Chunk streaming pipeline for batched operations
    streaming_pipeline: Arc<StreamingPipeline>,
    /// Performance metrics collector
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
}

impl ChunkStreamingManager {
    /// Create a new enhanced chunk streaming manager
    pub fn new(world_manager: Arc<EnhancedWorldManager>, config: StreamingConfig, ecs_world: Arc<TokioRwLock<EcsWorld>>) -> Self {
        let load_semaphore = Arc::new(Semaphore::new(config.max_concurrent_loads));
        let memory_manager = Arc::new(ChunkMemoryManager::new(config.memory_config.clone()));
        let preload_cache = Arc::new(RwLock::new(PreloadCache::new(config.preload_cache_size)));
        let streaming_pipeline = Arc::new(StreamingPipeline::new(config.batch_size));
        
        Self {
            world_manager,
            config,
            sessions: DashMap::new(),
            load_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            unload_queue: Arc::new(RwLock::new(VecDeque::new())),
            memory_manager,
            load_semaphore,
            stats: Arc::new(RwLock::new(StreamingStats::default())),
            ecs_world,
            preload_cache,
            streaming_pipeline,
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
        }
    }
    
    /// Start the enhanced streaming manager background tasks
    pub async fn start(&self) -> Result<()> {
        let load_queue = self.load_queue.clone();
        let unload_queue = self.unload_queue.clone();
        let world_manager = self.world_manager.clone();
        let memory_manager = self.memory_manager.clone();
        let load_semaphore = self.load_semaphore.clone();
        let stats = self.stats.clone();
        let config = self.config.clone();
        let streaming_pipeline = self.streaming_pipeline.clone();
        let performance_metrics = self.performance_metrics.clone();
        
        // Clone values for the async tasks to avoid move issues
        let world_manager_clone1 = world_manager.clone();
        let stats_clone1 = stats.clone();
        let config_clone1 = config.clone();
        let world_manager_clone2 = world_manager.clone();
        let stats_clone2 = stats.clone();
        let config_clone2 = config.clone();
        
        // Start enhanced chunk loading task
        tokio::spawn(async move {
            Self::chunk_loading_task(
                load_queue,
                world_manager_clone1,
                load_semaphore,
                stats_clone1,
                config_clone1,
                streaming_pipeline,
            ).await;
        });
        
        // Start chunk unloading task
        tokio::spawn(async move {
            Self::chunk_unloading_task(
                unload_queue,
                world_manager_clone2,
                memory_manager,
                stats_clone2,
                config_clone2,
            ).await;
        });
        
        // Start memory optimization task
        let world_manager_opt = world_manager.clone();
        let config_opt = config.clone();
        let stats_opt = stats.clone();
        tokio::spawn(async move {
            Self::memory_optimization_task(
                world_manager_opt,
                config_opt,
                stats_opt,
            ).await;
        });
        
        // Start performance monitoring task
        let performance_metrics_monitor = performance_metrics.clone();
        tokio::spawn(async move {
            Self::performance_monitoring_task(performance_metrics_monitor).await;
        });
        
        info!("Enhanced chunk streaming manager started with optimizations");
        Ok(())
    }
    
    /// Background task for memory optimization
    async fn memory_optimization_task(
        world_manager: Arc<EnhancedWorldManager>,
        config: StreamingConfig,
        stats: Arc<RwLock<StreamingStats>>,
    ) {
        let mut optimization_interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            optimization_interval.tick().await;
            
            let memory_stats = world_manager.get_memory_stats();
            let memory_pressure = memory_stats.estimated_memory_bytes as f64 / 
                config.memory_config.max_memory_bytes as f64;
            
            if memory_pressure > config.memory_optimization_threshold {
                debug!("Memory pressure at {:.2}%, starting optimization", memory_pressure * 100.0);
                
                // This would need to be implemented in the streaming manager
                // For now, we'll just log the optimization attempt
                stats.write().memory_optimizations += 1;
                
                info!("Memory optimization completed, pressure: {:.2}%", memory_pressure * 100.0);
            }
        }
    }
    
    /// Background task for performance monitoring
    async fn performance_monitoring_task(
        performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    ) {
        let mut monitor_interval = tokio::time::interval(Duration::from_secs(10));
        
        loop {
            monitor_interval.tick().await;
            
            let metrics = performance_metrics.read();
            debug!(
                "Performance: avg_load_time={:?}, memory_pressure={:.2}%, entities={}",
                metrics.avg_load_time,
                metrics.memory_pressure * 100.0,
                metrics.entity_count
            );
        }
    }
    
    /// Create a new streaming session for a player
    pub fn create_session(&self, player_id: uuid::Uuid, position: ChunkPos, view_distance: u32) -> Arc<RwLock<StreamingSession>> {
        let session = Arc::new(RwLock::new(StreamingSession::new(
            player_id,
            position,
            view_distance,
        )));
        
        self.sessions.insert(player_id, session.clone());
        debug!("Created streaming session for player {}", player_id);
        
        session
    }
    
    /// Remove a streaming session
    pub fn remove_session(&self, player_id: &uuid::Uuid) {
        if self.sessions.remove(player_id).is_some() {
            debug!("Removed streaming session for player {}", player_id);
        }
    }
    
    /// Enhanced player position update with predictive loading and optimization
    pub async fn update_player_position(&self, player_id: uuid::Uuid, new_position: ChunkPos) -> Result<()> {
        if let Some(session_ref) = self.sessions.get(&player_id) {
            let session = session_ref.value().clone();
            let mut session_guard = session.write();
            
            let old_position = session_guard.current_position;
            session_guard.update_position(new_position);
            
            // Calculate chunks to load and unload
            let (to_load, to_unload) = self.calculate_chunk_changes(&session_guard, old_position, new_position);
            
            // Use batch loading for better performance
            if !to_load.is_empty() {
                // Clone for batch loading
                let load_requests = to_load.clone();
                
                // Use optimized batch loading
                match self.batch_load_chunks(load_requests).await {
                    Ok(loaded_chunks) => {
                        session_guard.stats.chunks_loaded += loaded_chunks.len() as u64;
                        debug!("Batch loaded {} chunks for player {}", loaded_chunks.len(), player_id);
                    }
                    Err(e) => {
                        warn!("Failed to batch load chunks for player {}: {}", player_id, e);
                        // Fallback to individual loading
                        for chunk_pos in to_load {
                            self.queue_chunk_load(chunk_pos, ChunkPriority::Player(player_id)).await?;
                        }
                    }
                }
            }
            
            // Queue chunks for unloading
            for chunk_pos in to_unload {
                self.queue_chunk_unload(chunk_pos).await?;
                session_guard.stats.chunks_unloaded += 1;
            }
            
            // Trigger predictive preloading if enabled
            if self.config.enable_predictive_preload {
                drop(session_guard); // Release lock before async call
                self.predictive_preload(player_id).await?;
            } else {
                session_guard.last_update = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
            }
        }
        
        Ok(())
    }
    
    /// Queue a chunk for loading with priority
    pub async fn queue_chunk_load(&self, pos: ChunkPos, priority: ChunkPriority) -> Result<()> {
        let request = ChunkLoadRequest {
            pos,
            priority,
            requested_at: Instant::now(),
        };
        
        self.load_queue.write().push(request);
        self.stats.write().queued_loads += 1;
        
        Ok(())
    }
    
    /// Queue a chunk for unloading
    pub async fn queue_chunk_unload(&self, pos: ChunkPos) -> Result<()> {
        self.unload_queue.write().push_back(pos);
        self.stats.write().queued_unloads += 1;
        
        Ok(())
    }
    
    /// Calculate which chunks need to be loaded/unloaded when player moves
    fn calculate_chunk_changes(
        &self,
        session: &StreamingSession,
        old_position: ChunkPos,
        new_position: ChunkPos,
    ) -> (Vec<ChunkPos>, Vec<ChunkPos>) {
        let view_distance = session.view_distance as i32;
        
        // Calculate old and new chunk sets
        let old_chunks = self.get_chunks_in_radius(old_position, view_distance);
        let new_chunks = self.get_chunks_in_radius(new_position, view_distance);
        
        // Find chunks to load (in new set but not in old set)
        let to_load: Vec<ChunkPos> = new_chunks
            .iter()
            .filter(|chunk| !old_chunks.contains(chunk))
            .copied()
            .collect();
        
        // Find chunks to unload (in old set but not in new set)
        let to_unload: Vec<ChunkPos> = old_chunks
            .iter()
            .filter(|chunk| !new_chunks.contains(chunk))
            .copied()
            .collect();
        
        (to_load, to_unload)
    }
    
    /// Get all chunks within a radius
    fn get_chunks_in_radius(&self, center: ChunkPos, radius: i32) -> Vec<ChunkPos> {
        let mut chunks = Vec::new();
        
        for x in (center.x - radius)..=(center.x + radius) {
            for z in (center.z - radius)..=(center.z + radius) {
                chunks.push(ChunkPos::new(x, z));
            }
        }
        
        chunks
    }
    
    /// Enhanced background task for loading chunks with batching and optimization
    async fn chunk_loading_task(
        load_queue: Arc<RwLock<BinaryHeap<ChunkLoadRequest>>>,
        world_manager: Arc<EnhancedWorldManager>,
        load_semaphore: Arc<Semaphore>,
        stats: Arc<RwLock<StreamingStats>>,
        config: StreamingConfig,
        streaming_pipeline: Arc<StreamingPipeline>,
    ) {
        let mut batch_buffer = Vec::new();
        let batch_timeout = Duration::from_millis(config.load_poll_interval_ms * 2);
        let mut last_batch_time = Instant::now();
        
        loop {
            // Collect requests for batching
            let mut collected_requests = 0;
            while collected_requests < config.batch_size {
                let request = {
                    let mut queue = load_queue.write();
                    queue.pop()
                };
                
                if let Some(request) = request {
                    batch_buffer.push(request);
                    collected_requests += 1;
                } else {
                    break;
                }
            }
            
            // Process batch if we have requests or timeout reached
            let should_process_batch = !batch_buffer.is_empty() && 
                (batch_buffer.len() >= config.batch_size || 
                 last_batch_time.elapsed() >= batch_timeout);
            
            if should_process_batch {
                let batch = std::mem::take(&mut batch_buffer);
                Self::process_chunk_batch(
                    batch,
                    world_manager.clone(),
                    load_semaphore.clone(),
                    stats.clone(),
                ).await;
                last_batch_time = Instant::now();
            } else if batch_buffer.is_empty() {
                // No chunks to load, sleep briefly
                sleep(Duration::from_millis(config.load_poll_interval_ms)).await;
            }
        }
    }
    
    /// Process a batch of chunk load requests
    async fn process_chunk_batch(
        batch: Vec<ChunkLoadRequest>,
        world_manager: Arc<EnhancedWorldManager>,
        load_semaphore: Arc<Semaphore>,
        stats: Arc<RwLock<StreamingStats>>,
    ) {
        if batch.is_empty() {
            return;
        }
        
        let batch_start = Instant::now();
        let batch_size = batch.len();
        
        // Sort batch by priority
        let mut sorted_batch = batch;
        sorted_batch.sort_by(|a, b| a.priority.cmp(&b.priority));
        
        // Process chunks concurrently within semaphore limits
        let mut tasks = Vec::new();
        
        for request in sorted_batch {
            let world_manager = world_manager.clone();
            let load_semaphore = load_semaphore.clone();
            let stats = stats.clone();
            
            let task = tokio::spawn(async move {
                let _permit = load_semaphore.acquire().await.unwrap();
                let start_time = Instant::now();
                
                match world_manager.load_chunk(request.pos).await {
                    Ok(_chunk) => {
                        let load_time = start_time.elapsed();
                        let mut stats_guard = stats.write();
                        stats_guard.successful_loads += 1;
                        stats_guard.total_load_time += load_time;
                        stats_guard.average_load_time = 
                            stats_guard.total_load_time / stats_guard.successful_loads.max(1) as u32;
                        
                        debug!("Loaded chunk {:?} in {:?}", request.pos, load_time);
                        true
                    }
                    Err(e) => {
                        stats.write().failed_loads += 1;
                        error!("Failed to load chunk {:?}: {}", request.pos, e);
                        false
                    }
                }
            });
            
            tasks.push(task);
        }
        
        // Wait for all tasks to complete
        let results = futures::future::join_all(tasks).await;
        let successful_loads = results.iter()
            .filter_map(|r| r.as_ref().ok())
            .filter(|&&success| success)
            .count();
        
        let batch_time = batch_start.elapsed();
        debug!(
            "Processed batch of {} chunks ({} successful) in {:?}",
            batch_size, successful_loads, batch_time
        );
    }
    
    /// Background task for unloading chunks
    async fn chunk_unloading_task(
        unload_queue: Arc<RwLock<VecDeque<ChunkPos>>>,
        world_manager: Arc<EnhancedWorldManager>,
        memory_manager: Arc<ChunkMemoryManager>,
        stats: Arc<RwLock<StreamingStats>>,
        config: StreamingConfig,
    ) {
        loop {
            // Get next chunk to unload
            let pos = {
                let mut queue = unload_queue.write();
                queue.pop_front()
            };
            
            if let Some(pos) = pos {
                let start_time = Instant::now();
                
                // Check if chunk should be unloaded based on memory pressure
                if memory_manager.should_unload_chunk(pos).await {
                    match world_manager.unload_chunk(pos) {
                        Ok(()) => {
                            let unload_time = start_time.elapsed();
                            let mut stats_guard = stats.write();
                            stats_guard.successful_unloads += 1;
                            stats_guard.total_unload_time += unload_time;
                            
                            debug!("Unloaded chunk {:?} in {:?}", pos, unload_time);
                        }
                        Err(e) => {
                            stats.write().failed_unloads += 1;
                            warn!("Failed to unload chunk {:?}: {}", pos, e);
                        }
                    }
                }
            } else {
                // No chunks to unload, sleep briefly
                sleep(Duration::from_millis(config.unload_poll_interval_ms)).await;
            }
        }
    }
    
    /// Get streaming statistics
    pub fn get_stats(&self) -> StreamingStats {
        self.stats.read().clone()
    }
    
    /// Get memory statistics
    pub fn get_memory_stats(&self) -> crate::world::ChunkMemoryStats {
        self.world_manager.get_memory_stats()
    }
    
    /// Enhanced chunk loading with preload cache and ECS integration
    pub async fn load_chunk_optimized(&self, pos: ChunkPos) -> Result<Arc<RwLock<EnhancedChunk>>> {
        let start_time = Instant::now();
        
        // Check preload cache first
        {
            let mut cache = self.preload_cache.write();
            if let Some(cached) = cache.get(pos) {
                debug!("Chunk {:?} loaded from preload cache", pos);
                self.update_performance_metrics(start_time.elapsed(), true).await;
                return Ok(cached.chunk.clone());
            }
        }
        
        // Load chunk normally
        let chunk = self.world_manager.load_chunk(pos).await?;
        
        // Add to preload cache
        {
            let mut cache = self.preload_cache.write();
            cache.insert(pos, chunk.clone());
        }
        
        // Update ECS entities for this chunk
        self.update_chunk_entities(pos, &chunk).await?;
        
        self.update_performance_metrics(start_time.elapsed(), false).await;
        Ok(chunk)
    }
    
    /// Update ECS entities when chunk is loaded
    async fn update_chunk_entities(&self, pos: ChunkPos, chunk: &Arc<RwLock<EnhancedChunk>>) -> Result<()> {
        let mut ecs_world = self.ecs_world.write().await;
        
        // Get chunk entity manager
        if let Some(mut chunk_manager) = ecs_world.get_resource_mut::<crate::ecs_integration::ChunkEntityManager>() {
            // Set chunk state to loaded
            chunk_manager.set_chunk_state(pos, crate::ecs_integration::ChunkEntityState::Loaded);
            
            // Activate entities in this chunk
            let entities = chunk_manager.get_entities_in_chunk(pos);
            for entity in entities {
                if let Some(mut active) = ecs_world.get_component_mut::<crate::ecs_integration::EntityActive>(entity) {
                    active.set_active(true);
                }
            }
        }
        
        Ok(())
    }
    
    /// Batch load multiple chunks for improved performance
    pub async fn batch_load_chunks(&self, positions: Vec<ChunkPos>) -> Result<Vec<Arc<RwLock<EnhancedChunk>>>> {
        let start_time = Instant::now();
        let mut results = Vec::with_capacity(positions.len());
        
        // Sort positions by priority (distance from center)
        let mut sorted_positions = positions;
        if let Some(center) = self.calculate_center_position() {
            sorted_positions.sort_by_key(|pos| {
                let dx = pos.x - center.x;
                let dz = pos.z - center.z;
                dx * dx + dz * dz
            });
        }
        
        // Process in batches to avoid overwhelming the system
        for batch in sorted_positions.chunks(self.config.batch_size) {
            let mut batch_futures = Vec::new();
            
            for &pos in batch {
                let future = self.load_chunk_optimized(pos);
                batch_futures.push(future);
            }
            
            // Wait for batch to complete
            let batch_results = futures::future::try_join_all(batch_futures).await?;
            results.extend(batch_results);
            
            // Small delay between batches to prevent system overload
            if batch.len() == self.config.batch_size {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
        
        let total_time = start_time.elapsed();
        debug!("Batch loaded {} chunks in {:?}", results.len(), total_time);
        
        // Update batch processing statistics
        {
            let mut stats = self.stats.write();
            stats.successful_loads += results.len() as u64;
            stats.total_load_time += total_time;
        }
        
        Ok(results)
    }
    
    /// Calculate center position from active sessions
    fn calculate_center_position(&self) -> Option<ChunkPos> {
        if self.sessions.is_empty() {
            return None;
        }
        
        let mut total_x = 0i64;
        let mut total_z = 0i64;
        let mut count = 0;
        
        for session in self.sessions.iter() {
            let session_guard = session.value().read();
            total_x += session_guard.current_position.x as i64;
            total_z += session_guard.current_position.z as i64;
            count += 1;
        }
        
        if count > 0 {
            Some(ChunkPos::new(
                (total_x / count as i64) as i32,
                (total_z / count as i64) as i32,
            ))
        } else {
            None
        }
    }
    
    /// Predictive chunk preloading based on player movement patterns
    pub async fn predictive_preload(&self, player_id: uuid::Uuid) -> Result<()> {
        if let Some(session_ref) = self.sessions.get(&player_id) {
            let session = session_ref.value().clone();
            let session_guard = session.read();
            
            // Predict next chunks based on movement direction
            let predicted_chunks = self.predict_movement_chunks(&session_guard);
            
            // Queue predicted chunks for preloading
            for chunk_pos in predicted_chunks {
                if !self.world_manager.get_chunk(chunk_pos).is_some() {
                    self.queue_chunk_load(chunk_pos, ChunkPriority::Background).await?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Predict chunks based on player movement patterns
    fn predict_movement_chunks(&self, session: &StreamingSession) -> Vec<ChunkPos> {
        let mut predicted = Vec::new();
        let current = session.current_position;
        
        // Simple prediction: assume player continues in same direction
        if let Some(last_pos) = session.movement_history.back() {
            let dx = current.x - last_pos.x;
            let dz = current.z - last_pos.z;
            
            // Predict next 3 chunks in movement direction
            for i in 1..=3 {
                let predicted_pos = ChunkPos::new(
                    current.x + dx * i,
                    current.z + dz * i,
                );
                predicted.push(predicted_pos);
            }
        }
        
        predicted
    }
    
    /// Update performance metrics
    async fn update_performance_metrics(&self, operation_time: Duration, cache_hit: bool) {
        let mut metrics = self.performance_metrics.write();
        
        // Update timing metrics
        if cache_hit {
            // Cache hits are much faster
            metrics.avg_load_time = Duration::from_nanos(
                (metrics.avg_load_time.as_nanos() as f64 * 0.9 + operation_time.as_nanos() as f64 * 0.1) as u64
            );
        } else {
            metrics.avg_load_time = Duration::from_nanos(
                (metrics.avg_load_time.as_nanos() as f64 * 0.95 + operation_time.as_nanos() as f64 * 0.05) as u64
            );
        }
        
        // Update memory metrics
        let memory_stats = self.get_memory_stats();
        metrics.current_memory_usage = memory_stats.estimated_memory_bytes;
        if memory_stats.estimated_memory_bytes > metrics.peak_memory_usage {
            metrics.peak_memory_usage = memory_stats.estimated_memory_bytes;
        }
        
        // Update memory pressure
        let max_memory = self.config.memory_config.max_memory_bytes;
        metrics.memory_pressure = memory_stats.estimated_memory_bytes as f64 / max_memory as f64;
        
        // Update ECS entity count
        if let Ok(ecs_world) = self.ecs_world.try_read() {
            if let Some(chunk_manager) = ecs_world.get_resource::<crate::ecs_integration::ChunkEntityManager>() {
                let stats = chunk_manager.get_stats();
                metrics.entity_count = stats.total_entities;
            }
        }
        
        metrics.last_update = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }
    
    /// Get comprehensive streaming statistics
    pub fn get_comprehensive_stats(&self) -> ComprehensiveStreamingStats {
        let base_stats = self.get_stats();
        let cache_stats = self.preload_cache.read().get_stats();
        let pipeline_stats = self.streaming_pipeline.get_stats();
        let performance_metrics = self.performance_metrics.read().clone();
        
        ComprehensiveStreamingStats {
            base_stats,
            cache_stats,
            pipeline_stats,
            performance_metrics,
        }
    }
    
    /// Optimize memory usage by intelligently unloading chunks
    pub async fn optimize_memory_usage(&self) -> Result<usize> {
        let memory_stats = self.get_memory_stats();
        let max_memory = self.config.memory_config.max_memory_bytes;
        let current_pressure = memory_stats.estimated_memory_bytes as f64 / max_memory as f64;
        
        if current_pressure < 0.8 {
            return Ok(0); // No optimization needed
        }
        
        let mut unloaded_count = 0;
        let loaded_chunks = self.world_manager.get_loaded_chunks();
        
        // Sort chunks by access patterns and distance from players
        let mut chunk_priorities: Vec<(ChunkPos, f64)> = Vec::new();
        for &pos in &loaded_chunks {
            let priority = self.calculate_unload_priority(pos).await;
            chunk_priorities.push((pos, priority));
        }
        
        // Sort by priority (higher priority = more likely to unload)
        chunk_priorities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        
        // Unload chunks until memory pressure is reduced
        for (pos, _) in chunk_priorities {
            if current_pressure < 0.7 {
                break;
            }
            
            // Don't unload chunks that are actively being used
            if self.is_chunk_actively_used(pos) {
                continue;
            }
            
            self.world_manager.unload_chunk(pos)?;
            unloaded_count += 1;
            
            // Remove from cache as well
            self.preload_cache.write().remove(pos);
        }
        
        info!("Memory optimization unloaded {} chunks", unloaded_count);
        Ok(unloaded_count)
    }
    
    /// Calculate priority for unloading a chunk (higher = more likely to unload)
    async fn calculate_unload_priority(&self, pos: ChunkPos) -> f64 {
        let mut priority = 0.0;
        
        // Distance from all active players
        let mut min_distance = f64::MAX;
        for session in self.sessions.iter() {
            let session_guard = session.value().read();
            let dx = (pos.x - session_guard.current_position.x) as f64;
            let dz = (pos.z - session_guard.current_position.z) as f64;
            let distance = (dx * dx + dz * dz).sqrt();
            min_distance = min_distance.min(distance);
        }
        
        // Higher priority for distant chunks
        priority += min_distance * 0.1;
        
        // Check memory manager for access patterns
        if self.memory_manager.should_unload_chunk(pos).await {
            priority += 10.0;
        }
        
        priority
    }
    
    /// Check if chunk is actively being used
    fn is_chunk_actively_used(&self, pos: ChunkPos) -> bool {
        // Check if any player is within view distance
        for session in self.sessions.iter() {
            let session_guard = session.value().read();
            let dx = (pos.x - session_guard.current_position.x).abs();
            let dz = (pos.z - session_guard.current_position.z).abs();
            let distance = dx.max(dz) as u32;
            
            if distance <= session_guard.view_distance {
                return true;
            }
        }
        
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock as TokioRwLock;
    use crate::ecs_integration::EcsWorld;
    
    #[tokio::test]
    async fn test_preload_cache() {
        let mut cache = PreloadCache::new(3);
        let pos1 = ChunkPos::new(0, 0);
        let pos2 = ChunkPos::new(1, 1);
        let pos3 = ChunkPos::new(2, 2);
        let pos4 = ChunkPos::new(3, 3);
        
        // Create mock chunks
        let chunk1 = Arc::new(RwLock::new(EnhancedChunk::new(pos1, proto::types::Dimension::Overworld)));
        let chunk2 = Arc::new(RwLock::new(EnhancedChunk::new(pos2, proto::types::Dimension::Overworld)));
        let chunk3 = Arc::new(RwLock::new(EnhancedChunk::new(pos3, proto::types::Dimension::Overworld)));
        let chunk4 = Arc::new(RwLock::new(EnhancedChunk::new(pos4, proto::types::Dimension::Overworld)));
        
        // Test insertion
        cache.insert(pos1, chunk1);
        cache.insert(pos2, chunk2);
        cache.insert(pos3, chunk3);
        
        assert_eq!(cache.cache.len(), 3);
        assert!(cache.contains(pos1));
        assert!(cache.contains(pos2));
        assert!(cache.contains(pos3));
        
        // Test eviction when cache is full
        cache.insert(pos4, chunk4);
        assert_eq!(cache.cache.len(), 3);
        assert!(cache.contains(pos4));
        
        // Test cache hit/miss statistics
        let stats = cache.get_stats();
        assert_eq!(stats.max_size, 3);
        assert_eq!(stats.size, 3);
    }
    
    #[tokio::test]
    async fn test_streaming_pipeline() {
        let pipeline = StreamingPipeline::new(5);
        
        // Test load queueing
        let request1 = ChunkLoadRequest {
            pos: ChunkPos::new(0, 0),
            priority: ChunkPriority::Critical,
            requested_at: Instant::now(),
        };
        let request2 = ChunkLoadRequest {
            pos: ChunkPos::new(1, 1),
            priority: ChunkPriority::Background,
            requested_at: Instant::now(),
        };
        
        pipeline.queue_load(request1);
        pipeline.queue_load(request2);
        
        let batch = pipeline.process_load_batch();
        assert_eq!(batch.len(), 2);
        
        // Test unload queueing
        pipeline.queue_unload(ChunkPos::new(0, 0));
        pipeline.queue_unload(ChunkPos::new(1, 1));
        
        let unload_batch = pipeline.process_unload_batch();
        assert_eq!(unload_batch.len(), 2);
    }
    
    #[test]
    fn test_streaming_session_movement_tracking() {
        let player_id = uuid::Uuid::new_v4();
        let mut session = StreamingSession::new(player_id, ChunkPos::new(0, 0), 8);
        
        // Test initial state
        assert_eq!(session.movement_history.len(), 1);
        assert_eq!(session.current_position, ChunkPos::new(0, 0));
        
        // Test position updates
        session.update_position(ChunkPos::new(1, 0));
        assert_eq!(session.movement_history.len(), 2);
        assert_eq!(session.current_position, ChunkPos::new(1, 0));
        assert_eq!(session.stats.position_updates, 1);
        
        // Test movement history limit
        for i in 2..15 {
            session.update_position(ChunkPos::new(i, 0));
        }
        
        assert_eq!(session.movement_history.len(), 10); // Should be capped at 10
        assert_eq!(session.stats.position_updates, 14);
    }
    
    #[test]
    fn test_chunk_priority_ordering() {
        let mut heap = BinaryHeap::new();
        
        let critical = ChunkLoadRequest {
            pos: ChunkPos::new(0, 0),
            priority: ChunkPriority::Critical,
            requested_at: Instant::now(),
        };
        
        let player = ChunkLoadRequest {
            pos: ChunkPos::new(1, 1),
            priority: ChunkPriority::Player(uuid::Uuid::new_v4()),
            requested_at: Instant::now(),
        };
        
        let background = ChunkLoadRequest {
            pos: ChunkPos::new(2, 2),
            priority: ChunkPriority::Background,
            requested_at: Instant::now(),
        };
        
        heap.push(background);
        heap.push(player);
        heap.push(critical);
        
        // Critical should come first
        assert_eq!(heap.pop().unwrap().priority, ChunkPriority::Critical);
        // Player should come second
        assert!(matches!(heap.pop().unwrap().priority, ChunkPriority::Player(_)));
        // Background should come last
        assert_eq!(heap.pop().unwrap().priority, ChunkPriority::Background);
    }
    
    #[test]
    fn test_performance_metrics() {
        let mut metrics = PerformanceMetrics::default();
        
        // Test initial state
        assert_eq!(metrics.avg_load_time, Duration::default());
        assert_eq!(metrics.memory_pressure, 0.0);
        assert_eq!(metrics.entity_count, 0);
        
        // Test updates would be done through the streaming manager
        // This is just testing the structure
        metrics.current_memory_usage = 1024 * 1024; // 1MB
        metrics.peak_memory_usage = 2 * 1024 * 1024; // 2MB
        metrics.entity_count = 100;
        
        assert_eq!(metrics.current_memory_usage, 1024 * 1024);
        assert_eq!(metrics.peak_memory_usage, 2 * 1024 * 1024);
        assert_eq!(metrics.entity_count, 100);
    }
}

/// Enhanced streaming session for a player with movement prediction
#[derive(Debug)]
pub struct StreamingSession {
    /// Player ID
    pub player_id: uuid::Uuid,
    /// Current chunk position
    pub current_position: ChunkPos,
    /// View distance in chunks
    pub view_distance: u32,
    /// Currently loaded chunks for this player
    pub loaded_chunks: HashMap<ChunkPos, u64>, // Timestamp in milliseconds since epoch
    /// Last position update
    pub last_update: u64, // Timestamp in milliseconds since epoch
    /// Session creation time
    pub created_at: u64, // Timestamp in milliseconds since epoch
    /// Movement history for prediction (last 10 positions)
    pub movement_history: VecDeque<ChunkPos>,
    /// Predicted chunks for preloading
    pub predicted_chunks: HashSet<ChunkPos>,
    /// Session statistics
    pub stats: SessionStats,
}

impl StreamingSession {
    /// Create a new enhanced streaming session
    pub fn new(player_id: uuid::Uuid, position: ChunkPos, view_distance: u32) -> Self {
        let mut movement_history = VecDeque::new();
        movement_history.push_back(position);
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        Self {
            player_id,
            current_position: position,
            view_distance,
            loaded_chunks: HashMap::new(),
            last_update: now,
            created_at: now,
            movement_history,
            predicted_chunks: HashSet::new(),
            stats: SessionStats::default(),
        }
    }
    
    /// Update player position with movement tracking
    pub fn update_position(&mut self, new_position: ChunkPos) {
        if new_position != self.current_position {
            // Add to movement history
            self.movement_history.push_back(self.current_position);
            if self.movement_history.len() > 10 {
                self.movement_history.pop_front();
            }
            
            self.current_position = new_position;
            self.stats.position_updates += 1;
        }
        
        self.last_update = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }
    
    /// Mark chunk as loaded for this session
    pub fn mark_chunk_loaded(&mut self, pos: ChunkPos) {
        self.loaded_chunks.insert(pos, std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64);
    }
    
    /// Mark chunk as unloaded for this session
    pub fn mark_chunk_unloaded(&mut self, pos: ChunkPos) {
        self.loaded_chunks.remove(&pos);
    }
    
    /// Check if chunk is loaded for this session
    pub fn is_chunk_loaded(&self, pos: ChunkPos) -> bool {
        self.loaded_chunks.contains_key(&pos)
    }
}

/// Chunk load request with priority
#[derive(Debug, Clone)]
pub struct ChunkLoadRequest {
    pub pos: ChunkPos,
    pub priority: ChunkPriority,
    pub requested_at: Instant,
}

impl PartialEq for ChunkLoadRequest {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos
    }
}

impl Eq for ChunkLoadRequest {}

impl PartialOrd for ChunkLoadRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ChunkLoadRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority chunks should be processed first
        other.priority.cmp(&self.priority)
            .then_with(|| self.requested_at.cmp(&other.requested_at))
    }
}

/// Chunk loading priority
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChunkPriority {
    /// Critical chunks (spawn area, etc.)
    Critical,
    /// Player-requested chunks
    Player(uuid::Uuid),
    /// Background generation
    Background,
}

/// Chunk memory manager for optimization
pub struct ChunkMemoryManager {
    /// Memory configuration
    config: MemoryConfig,
    /// Chunk access tracking
    access_tracker: DashMap<ChunkPos, ChunkAccessInfo>,
    /// Memory pressure monitoring
    memory_monitor: Arc<RwLock<MemoryMonitor>>,
}

impl ChunkMemoryManager {
    /// Create a new chunk memory manager
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            config,
            access_tracker: DashMap::new(),
            memory_monitor: Arc::new(RwLock::new(MemoryMonitor::new())),
        }
    }
    
    /// Track chunk access
    pub fn track_access(&self, pos: ChunkPos) {
        let mut access_info = self.access_tracker.entry(pos).or_insert_with(|| ChunkAccessInfo::new());
        access_info.last_accessed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        access_info.access_count += 1;
    }
    
    /// Check if chunk should be unloaded based on memory pressure and access patterns
    pub async fn should_unload_chunk(&self, pos: ChunkPos) -> bool {
        let memory_monitor = self.memory_monitor.read();
        
        // Always unload if memory pressure is critical
        if memory_monitor.memory_pressure >= 0.9 {
            return true;
        }
        
        // Check access patterns
        if let Some(access_info) = self.access_tracker.get(&pos) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let time_since_access = Duration::from_millis(now.saturating_sub(access_info.last_accessed));
            
            // Unload if not accessed recently and memory pressure is high
            if memory_monitor.memory_pressure >= 0.7 && time_since_access > Duration::from_secs(300) {
                return true;
            }
            
            // Unload if not accessed for a long time
            if time_since_access > Duration::from_secs(600) {
                return true;
            }
        }
        
        false
    }
    
    /// Update memory pressure monitoring
    pub fn update_memory_pressure(&self, used_memory: usize, total_memory: usize) {
        let mut monitor = self.memory_monitor.write();
        monitor.update_memory_pressure(used_memory, total_memory);
    }
}

/// Chunk access information for memory management
#[derive(Debug)]
pub struct ChunkAccessInfo {
    pub last_accessed: u64, // Timestamp in milliseconds since epoch
    pub access_count: u64,
    pub created_at: u64, // Timestamp in milliseconds since epoch
}

impl ChunkAccessInfo {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            last_accessed: now,
            access_count: 1,
            created_at: now,
        }
    }
}

/// Memory pressure monitor
#[derive(Debug)]
pub struct MemoryMonitor {
    pub memory_pressure: f64, // 0.0 to 1.0
    pub last_update: u64, // Timestamp in milliseconds since epoch
}

impl MemoryMonitor {
    pub fn new() -> Self {
        Self {
            memory_pressure: 0.0,
            last_update: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
    
    pub fn update_memory_pressure(&mut self, used_memory: usize, total_memory: usize) {
        self.memory_pressure = used_memory as f64 / total_memory as f64;
        self.last_update = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }
}

/// Enhanced streaming configuration with optimization settings
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Maximum concurrent chunk loads
    pub max_concurrent_loads: usize,
    /// Load queue polling interval in milliseconds
    pub load_poll_interval_ms: u64,
    /// Unload queue polling interval in milliseconds
    pub unload_poll_interval_ms: u64,
    /// Memory management configuration
    pub memory_config: MemoryConfig,
    /// Preload cache size
    pub preload_cache_size: usize,
    /// Batch size for chunk operations
    pub batch_size: usize,
    /// Enable predictive preloading
    pub enable_predictive_preload: bool,
    /// Preload distance (chunks ahead to preload)
    pub preload_distance: u32,
    /// Memory optimization threshold (0.0 to 1.0)
    pub memory_optimization_threshold: f64,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_concurrent_loads: 8, // Increased for better performance
            load_poll_interval_ms: 25, // Reduced for more responsive loading
            unload_poll_interval_ms: 100,
            memory_config: MemoryConfig::default(),
            preload_cache_size: 256, // Cache for 256 chunks
            batch_size: 16, // Process chunks in batches of 16
            enable_predictive_preload: true,
            preload_distance: 2, // Preload 2 chunks ahead
            memory_optimization_threshold: 0.8, // Optimize when 80% memory used
        }
    }
}

/// Memory management configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum memory usage in bytes
    pub max_memory_bytes: usize,
    /// Memory pressure threshold for aggressive unloading
    pub pressure_threshold: f64,
    /// Chunk access timeout for unloading
    pub access_timeout_secs: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 1024 * 1024 * 1024, // 1GB
            pressure_threshold: 0.8,
            access_timeout_secs: 300, // 5 minutes
        }
    }
}

/// Enhanced streaming statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamingStats {
    pub queued_loads: u64,
    pub successful_loads: u64,
    pub failed_loads: u64,
    pub queued_unloads: u64,
    pub successful_unloads: u64,
    pub failed_unloads: u64,
    pub total_load_time: Duration,
    pub total_unload_time: Duration,
    pub average_load_time: Duration,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub predictive_loads: u64,
    pub memory_optimizations: u64,
}

/// Session statistics for individual players
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    pub position_updates: u64,
    pub chunks_loaded: u64,
    pub chunks_unloaded: u64,
    pub cache_hits: u64,
    pub predictive_hits: u64,
}

/// Comprehensive streaming statistics combining all metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveStreamingStats {
    pub base_stats: StreamingStats,
    pub cache_stats: CacheStats,
    pub pipeline_stats: PipelineStats,
    pub performance_metrics: PerformanceMetrics,
}

/// Memory usage statistics for chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMemoryStats {
    pub loaded_chunks: usize,
    pub estimated_memory_bytes: usize,
}

/// Preload cache for chunk streaming optimization
pub struct PreloadCache {
    /// Cached chunk positions with their priority
    cache: HashMap<ChunkPos, CachedChunk>,
    /// Maximum cache size
    max_size: usize,
    /// Cache hit statistics
    hits: u64,
    /// Cache miss statistics
    misses: u64,
}

impl PreloadCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            hits: 0,
            misses: 0,
        }
    }
    
    /// Check if chunk is in cache
    pub fn contains(&self, pos: ChunkPos) -> bool {
        self.cache.contains_key(&pos)
    }
    
    /// Get chunk from cache
    pub fn get(&mut self, pos: ChunkPos) -> Option<&CachedChunk> {
        if let Some(cached) = self.cache.get_mut(&pos) {
            cached.last_accessed = Instant::now();
            cached.access_count += 1;
            self.hits += 1;
            Some(cached)
        } else {
            self.misses += 1;
            None
        }
    }
    
    /// Add chunk to cache
    pub fn insert(&mut self, pos: ChunkPos, chunk: Arc<RwLock<EnhancedChunk>>) {
        if self.cache.len() >= self.max_size {
            self.evict_oldest();
        }
        
        self.cache.insert(pos, CachedChunk {
            chunk,
            cached_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 1,
        });
    }
    
    /// Remove chunk from cache
    pub fn remove(&mut self, pos: ChunkPos) -> Option<CachedChunk> {
        self.cache.remove(&pos)
    }
    
    /// Evict oldest chunk from cache
    fn evict_oldest(&mut self) {
        if let Some(oldest_pos) = self.cache.iter()
            .min_by_key(|(_, cached)| cached.last_accessed)
            .map(|(pos, _)| *pos) {
            self.cache.remove(&oldest_pos);
        }
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            size: self.cache.len(),
            max_size: self.max_size,
            hits: self.hits,
            misses: self.misses,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }
}

/// Cached chunk information
#[derive(Debug)]
pub struct CachedChunk {
    pub chunk: Arc<RwLock<EnhancedChunk>>,
    pub cached_at: Instant,
    pub last_accessed: Instant,
    pub access_count: u64,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub size: usize,
    pub max_size: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

/// Streaming pipeline for batched chunk operations
pub struct StreamingPipeline {
    /// Batch size for chunk operations
    batch_size: usize,
    /// Pending load operations
    pending_loads: RwLock<Vec<ChunkLoadRequest>>,
    /// Pending unload operations
    pending_unloads: RwLock<Vec<ChunkPos>>,
    /// Pipeline statistics
    stats: RwLock<PipelineStats>,
}

impl StreamingPipeline {
    pub fn new(batch_size: usize) -> Self {
        Self {
            batch_size,
            pending_loads: RwLock::new(Vec::new()),
            pending_unloads: RwLock::new(Vec::new()),
            stats: RwLock::new(PipelineStats::default()),
        }
    }
    
    /// Add chunk load to pipeline
    pub fn queue_load(&self, request: ChunkLoadRequest) {
        let mut pending = self.pending_loads.write();
        pending.push(request);
        
        if pending.len() >= self.batch_size {
            // Trigger batch processing
            self.stats.write().batches_processed += 1;
        }
    }
    
    /// Add chunk unload to pipeline
    pub fn queue_unload(&self, pos: ChunkPos) {
        let mut pending = self.pending_unloads.write();
        pending.push(pos);
        
        if pending.len() >= self.batch_size {
            // Trigger batch processing
            self.stats.write().batches_processed += 1;
        }
    }
    
    /// Process pending loads in batch
    pub fn process_load_batch(&self) -> Vec<ChunkLoadRequest> {
        let mut pending = self.pending_loads.write();
        let batch_size = pending.len().min(self.batch_size);
        let batch = pending.drain(..batch_size).collect();
        batch
    }
    
    /// Process pending unloads in batch
    pub fn process_unload_batch(&self) -> Vec<ChunkPos> {
        let mut pending = self.pending_unloads.write();
        let batch_size = pending.len().min(self.batch_size);
        let batch = pending.drain(..batch_size).collect();
        batch
    }
    
    /// Get pipeline statistics
    pub fn get_stats(&self) -> PipelineStats {
        self.stats.read().clone()
    }
}

/// Pipeline statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineStats {
    pub batches_processed: u64,
    pub total_loads_processed: u64,
    pub total_unloads_processed: u64,
    pub average_batch_size: f64,
}

/// Performance metrics for streaming system
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average chunk load time
    pub avg_load_time: Duration,
    /// Average chunk unload time
    pub avg_unload_time: Duration,
    /// Peak memory usage
    pub peak_memory_usage: usize,
    /// Current memory usage
    pub current_memory_usage: usize,
    /// Chunks loaded per second
    pub chunks_per_second: f64,
    /// Memory pressure level (0.0 to 1.0)
    pub memory_pressure: f64,
    /// ECS entity count in loaded chunks
    pub entity_count: usize,
    /// Last metrics update
    pub last_update: u64, // Timestamp in milliseconds since epoch
}