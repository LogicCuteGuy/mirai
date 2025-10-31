//! Entity management for the ECS system

use super::EntityId;
use std::collections::HashSet;

/// Manages entity creation, destruction, and lifecycle
#[derive(Debug)]
pub struct EntityManager {
    next_id: u64,
    alive_entities: HashSet<EntityId>,
    recycled_ids: Vec<u64>,
}

impl EntityManager {
    /// Create a new entity manager
    pub fn new() -> Self {
        Self {
            next_id: 1, // Start from 1, reserve 0 for null entity
            alive_entities: HashSet::new(),
            recycled_ids: Vec::new(),
        }
    }
    
    /// Spawn a new entity and return its ID
    pub fn spawn(&mut self) -> EntityId {
        let id = if let Some(recycled_id) = self.recycled_ids.pop() {
            recycled_id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            id
        };
        
        let entity_id = EntityId::new(id);
        self.alive_entities.insert(entity_id);
        entity_id
    }
    
    /// Despawn an entity
    pub fn despawn(&mut self, entity: EntityId) {
        if self.alive_entities.remove(&entity) {
            self.recycled_ids.push(entity.id());
        }
    }
    
    /// Check if an entity is alive
    pub fn is_alive(&self, entity: EntityId) -> bool {
        self.alive_entities.contains(&entity)
    }
    
    /// Get the number of alive entities
    pub fn len(&self) -> usize {
        self.alive_entities.len()
    }
    
    /// Check if there are no alive entities
    pub fn is_empty(&self) -> bool {
        self.alive_entities.is_empty()
    }
    
    /// Get an iterator over all alive entities
    pub fn iter(&self) -> impl Iterator<Item = &EntityId> {
        self.alive_entities.iter()
    }
}

impl Default for EntityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Entity handle that provides a convenient interface for entity operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    id: EntityId,
}

impl Entity {
    /// Create a new entity handle
    pub fn new(id: EntityId) -> Self {
        Self { id }
    }
    
    /// Get the entity ID
    pub fn id(&self) -> EntityId {
        self.id
    }
}

impl From<EntityId> for Entity {
    fn from(id: EntityId) -> Self {
        Self::new(id)
    }
}

impl From<Entity> for EntityId {
    fn from(entity: Entity) -> Self {
        entity.id
    }
}