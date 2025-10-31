//! ECS integration for chunk and world management

use crate::world::{ChunkPos, EnhancedChunk, Position, BlockPos};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

// Define ECS types locally to avoid circular dependency
// These will be compatible with the core ECS system when integrated

/// Entity identifier for ECS integration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Entity(pub u64);

impl Entity {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
    
    pub fn id(&self) -> u64 {
        self.0
    }
}

/// Trait marker for ECS components
pub trait Component: Send + Sync + 'static {}

/// Trait marker for ECS resources  
pub trait Resource: Send + Sync + 'static {}

/// Placeholder for ECS World - will be integrated with core ECS system
pub struct EcsWorld {
    // This is a placeholder - actual integration will use the core ECS World
    _placeholder: (),
}

impl EcsWorld {
    pub fn new() -> Self {
        Self { _placeholder: () }
    }
    
    pub fn get_resource<T: Resource>(&self) -> Option<&T> {
        // Placeholder implementation
        None
    }
    
    pub fn get_resource_mut<T: Resource>(&mut self) -> Option<&mut T> {
        // Placeholder implementation
        None
    }
    
    pub fn get_component_mut<T: Component>(&mut self, _entity: Entity) -> Option<&mut T> {
        // Placeholder implementation
        None
    }
    
    pub fn query<T>(&self) -> QueryResult<T> {
        // Placeholder implementation
        QueryResult::new()
    }
}

/// Placeholder query result
pub struct QueryResult<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> QueryResult<T> {
    fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
    
    pub fn iter(&self) -> impl Iterator<Item = (Entity, T)> {
        std::iter::empty()
    }
}

/// ECS component for entities that have a position in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldPosition {
    pub position: Position,
    pub chunk: ChunkPos,
    pub last_chunk_update: u64, // Timestamp in milliseconds since epoch
}

impl Component for WorldPosition {}

impl WorldPosition {
    pub fn new(position: Position) -> Self {
        let chunk = ChunkPos::from_block_pos(BlockPos::from(position));
        Self {
            position,
            chunk,
            last_chunk_update: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
    
    pub fn update_position(&mut self, new_position: Position) {
        let new_chunk = ChunkPos::from_block_pos(BlockPos::from(new_position));
        
        if new_chunk != self.chunk {
            self.chunk = new_chunk;
            self.last_chunk_update = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
        }
        
        self.position = new_position;
    }
    
    pub fn distance_to(&self, other: &WorldPosition) -> f64 {
        self.position.distance_to(&other.position)
    }
}

/// ECS component for entities that are bound to a specific chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkBound {
    pub chunk: ChunkPos,
    pub local_position: Position, // Position within the chunk (0-16 range)
    pub bound_at: std::time::SystemTime,
}

impl Component for ChunkBound {}

impl ChunkBound {
    pub fn new(chunk: ChunkPos, local_position: Position) -> Self {
        Self {
            chunk,
            local_position,
            bound_at: std::time::SystemTime::now(),
        }
    }
    
    pub fn world_position(&self) -> Position {
        Position::new(
            self.chunk.x as f64 * 16.0 + self.local_position.x,
            self.local_position.y,
            self.chunk.z as f64 * 16.0 + self.local_position.z,
        )
    }
}

/// ECS component for tracking which chunks an entity is interested in
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInterest {
    pub center_chunk: ChunkPos,
    pub radius: u32,
    pub interested_chunks: Vec<ChunkPos>,
    pub last_update: u64, // Timestamp in milliseconds since epoch
}

impl Component for ChunkInterest {}

impl ChunkInterest {
    pub fn new(center_chunk: ChunkPos, radius: u32) -> Self {
        let mut interest = Self {
            center_chunk,
            radius,
            interested_chunks: Vec::new(),
            last_update: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };
        interest.update_interested_chunks();
        interest
    }
    
    pub fn update_center(&mut self, new_center: ChunkPos) {
        if new_center != self.center_chunk {
            self.center_chunk = new_center;
            self.update_interested_chunks();
            self.last_update = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
        }
    }
    
    fn update_interested_chunks(&mut self) {
        self.interested_chunks.clear();
        let radius = self.radius as i32;
        
        for x in (self.center_chunk.x - radius)..=(self.center_chunk.x + radius) {
            for z in (self.center_chunk.z - radius)..=(self.center_chunk.z + radius) {
                self.interested_chunks.push(ChunkPos::new(x, z));
            }
        }
    }
}

/// ECS resource for managing chunk entities
#[derive(Debug)]
pub struct ChunkEntityManager {
    /// Entities per chunk
    chunk_entities: HashMap<ChunkPos, Vec<Entity>>,
    /// Entity to chunk mapping
    entity_chunks: HashMap<Entity, ChunkPos>,
    /// Chunk loading state
    chunk_states: HashMap<ChunkPos, ChunkEntityState>,
}

impl Resource for ChunkEntityManager {}

impl ChunkEntityManager {
    pub fn new() -> Self {
        Self {
            chunk_entities: HashMap::new(),
            entity_chunks: HashMap::new(),
            chunk_states: HashMap::new(),
        }
    }
    
    /// Add an entity to a chunk
    pub fn add_entity_to_chunk(&mut self, entity: Entity, chunk: ChunkPos) {
        // Remove from old chunk if exists
        if let Some(old_chunk) = self.entity_chunks.get(&entity) {
            if let Some(entities) = self.chunk_entities.get_mut(old_chunk) {
                entities.retain(|&e| e != entity);
            }
        }
        
        // Add to new chunk
        self.chunk_entities.entry(chunk).or_insert_with(Vec::new).push(entity);
        self.entity_chunks.insert(entity, chunk);
    }
    
    /// Remove an entity from chunk tracking
    pub fn remove_entity(&mut self, entity: Entity) {
        if let Some(chunk) = self.entity_chunks.remove(&entity) {
            if let Some(entities) = self.chunk_entities.get_mut(&chunk) {
                entities.retain(|&e| e != entity);
                
                // Clean up empty chunk entries
                if entities.is_empty() {
                    self.chunk_entities.remove(&chunk);
                }
            }
        }
    }
    
    /// Get all entities in a chunk
    pub fn get_entities_in_chunk(&self, chunk: ChunkPos) -> Vec<Entity> {
        self.chunk_entities.get(&chunk).cloned().unwrap_or_default()
    }
    
    /// Get the chunk an entity is in
    pub fn get_entity_chunk(&self, entity: Entity) -> Option<ChunkPos> {
        self.entity_chunks.get(&entity).copied()
    }
    
    /// Get all entities in a radius around a chunk
    pub fn get_entities_in_radius(&self, center: ChunkPos, radius: u32) -> Vec<Entity> {
        let mut entities = Vec::new();
        let radius = radius as i32;
        
        for x in (center.x - radius)..=(center.x + radius) {
            for z in (center.z - radius)..=(center.z + radius) {
                let chunk = ChunkPos::new(x, z);
                entities.extend(self.get_entities_in_chunk(chunk));
            }
        }
        
        entities
    }
    
    /// Set chunk entity state
    pub fn set_chunk_state(&mut self, chunk: ChunkPos, state: ChunkEntityState) {
        self.chunk_states.insert(chunk, state);
    }
    
    /// Get chunk entity state
    pub fn get_chunk_state(&self, chunk: ChunkPos) -> ChunkEntityState {
        self.chunk_states.get(&chunk).copied().unwrap_or(ChunkEntityState::Unloaded)
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> ChunkEntityStats {
        ChunkEntityStats {
            total_chunks: self.chunk_entities.len(),
            total_entities: self.entity_chunks.len(),
            loaded_chunks: self.chunk_states.values()
                .filter(|&&state| state == ChunkEntityState::Loaded)
                .count(),
        }
    }
}

impl Default for ChunkEntityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// State of entities in a chunk
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkEntityState {
    /// Chunk is not loaded, entities are dormant
    Unloaded,
    /// Chunk is loading, entities are being activated
    Loading,
    /// Chunk is loaded, entities are active
    Loaded,
    /// Chunk is unloading, entities are being deactivated
    Unloading,
}

/// Statistics for chunk entity management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkEntityStats {
    pub total_chunks: usize,
    pub total_entities: usize,
    pub loaded_chunks: usize,
}

/// ECS system for updating entity positions and chunk assignments
pub struct EntityPositionSystem;

impl EntityPositionSystem {
    pub fn update(world: &mut EcsWorld) -> Result<()> {
        // Get chunk entity manager
        let mut chunk_manager = world.get_resource_mut::<ChunkEntityManager>()
            .ok_or_else(|| anyhow::anyhow!("ChunkEntityManager resource not found"))?;
        
        // TODO: Query entities with WorldPosition component when ECS is integrated
        let entities_with_positions: Vec<(Entity, WorldPosition)> = Vec::new();
        
        // Update chunk assignments
        for (entity, position) in entities_with_positions {
            let current_chunk = position.chunk;
            
            // Check if entity moved to a different chunk
            if let Some(old_chunk) = chunk_manager.get_entity_chunk(entity) {
                if old_chunk != current_chunk {
                    chunk_manager.add_entity_to_chunk(entity, current_chunk);
                }
            } else {
                // New entity, add to chunk
                chunk_manager.add_entity_to_chunk(entity, current_chunk);
            }
        }
        
        Ok(())
    }
}

/// ECS system for managing chunk interest updates
pub struct ChunkInterestSystem;

impl ChunkInterestSystem {
    pub fn update(world: &mut EcsWorld) -> Result<()> {
        // Query entities with both WorldPosition and ChunkInterest
        let mut updates = Vec::new();
        
        for (entity, (position, interest)) in world.query::<(&WorldPosition, &ChunkInterest)>().iter() {
            if position.chunk != interest.center_chunk {
                updates.push((entity, position.chunk));
            }
        }
        
        // Apply updates
        for (entity, new_center) in updates {
            if let Some(mut interest) = world.get_component_mut::<ChunkInterest>(entity) {
                interest.update_center(new_center);
            }
        }
        
        Ok(())
    }
}

/// ECS system for chunk entity lifecycle management
pub struct ChunkEntityLifecycleSystem;

impl ChunkEntityLifecycleSystem {
    pub fn update(world: &mut EcsWorld, loaded_chunks: &[ChunkPos]) -> Result<()> {
        let mut chunk_manager = world.get_resource_mut::<ChunkEntityManager>()
            .ok_or_else(|| anyhow::anyhow!("ChunkEntityManager resource not found"))?;
        
        // Update chunk states based on loaded chunks
        for &chunk in loaded_chunks {
            chunk_manager.set_chunk_state(chunk, ChunkEntityState::Loaded);
        }
        
        // Deactivate entities in unloaded chunks
        for (chunk, entities) in &chunk_manager.chunk_entities {
            let state = chunk_manager.get_chunk_state(*chunk);
            
            match state {
                ChunkEntityState::Unloaded => {
                    // Deactivate entities in unloaded chunks
                    for &entity in entities {
                        // TODO: Deactivate entity when ECS is integrated
                        let _ = entity;
                    }
                }
                ChunkEntityState::Loaded => {
                    // Activate entities in loaded chunks
                    for &entity in entities {
                        // TODO: Activate entity when ECS is integrated
                        let _ = entity;
                    }
                }
                _ => {} // Loading/Unloading states handled elsewhere
            }
        }
        
        Ok(())
    }
}

/// ECS component for entity activation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityActive {
    pub active: bool,
    pub last_state_change: u64, // Timestamp in milliseconds since epoch
}

impl Component for EntityActive {}

impl EntityActive {
    pub fn new(active: bool) -> Self {
        Self {
            active,
            last_state_change: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
    
    pub fn set_active(&mut self, active: bool) {
        if self.active != active {
            self.active = active;
            self.last_state_change = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
        }
    }
}

impl Default for EntityActive {
    fn default() -> Self {
        Self::new(true)
    }
}

/// Enhanced chunk with ECS entity support
impl EnhancedChunk {
    /// Get all entities in this chunk from the ECS world
    pub fn get_entities(&self, world: &EcsWorld) -> Vec<Entity> {
        if let Some(chunk_manager) = world.get_resource::<ChunkEntityManager>() {
            chunk_manager.get_entities_in_chunk(self.pos)
        } else {
            Vec::new()
        }
    }
    
    /// Add an entity to this chunk
    pub fn add_entity(&self, world: &mut EcsWorld, entity: Entity) -> Result<()> {
        let mut chunk_manager = world.get_resource_mut::<ChunkEntityManager>()
            .ok_or_else(|| anyhow::anyhow!("ChunkEntityManager resource not found"))?;
        
        chunk_manager.add_entity_to_chunk(entity, self.pos);
        Ok(())
    }
    
    /// Remove an entity from this chunk
    pub fn remove_entity(&self, world: &mut EcsWorld, entity: Entity) -> Result<()> {
        let mut chunk_manager = world.get_resource_mut::<ChunkEntityManager>()
            .ok_or_else(|| anyhow::anyhow!("ChunkEntityManager resource not found"))?;
        
        chunk_manager.remove_entity(entity);
        Ok(())
    }
    
    /// Update entity states when chunk loads/unloads
    pub fn update_entity_states(&self, world: &mut EcsWorld, loaded: bool) -> Result<()> {
        let entities = self.get_entities(world);
        
        for entity in entities {
            if let Some(mut active) = world.get_component_mut::<EntityActive>(entity) {
                active.set_active(loaded);
            }
        }
        
        Ok(())
    }
}