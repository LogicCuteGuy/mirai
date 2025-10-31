//! ECS World implementation integrated with Mirai

use super::{EntityManager, ComponentManager, ResourceManager, EntityId};
use super::event::EventBus;
use anyhow::Result;

/// The main ECS World that contains all entities, components, and resources
/// Integrated with Mirai's existing architecture
pub struct World {
    entities: EntityManager,
    components: ComponentManager,
    resources: ResourceManager,
    events: EventBus,
}

impl World {
    /// Create a new empty world
    pub fn new() -> Self {
        Self {
            entities: EntityManager::new(),
            components: ComponentManager::new(),
            resources: ResourceManager::new(),
            events: EventBus::new(),
        }
    }
    
    /// Spawn a new entity and return its ID
    pub fn spawn(&mut self) -> EntityId {
        self.entities.spawn()
    }
    
    /// Despawn an entity and remove all its components
    pub fn despawn(&mut self, entity: EntityId) -> Result<()> {
        if !self.entities.is_alive(entity) {
            return Err(anyhow::anyhow!("Entity {:?} not found", entity));
        }
        
        self.components.remove_all_for_entity(entity);
        self.entities.despawn(entity);
        Ok(())
    }
    
    /// Add a component to an entity
    pub fn insert<T: super::Component>(&mut self, entity: EntityId, component: T) -> Result<()> {
        if !self.entities.is_alive(entity) {
            return Err(anyhow::anyhow!("Entity {:?} not found", entity));
        }
        
        self.components.insert(entity, component);
        Ok(())
    }
    
    /// Remove a component from an entity
    pub fn remove<T: super::Component>(&mut self, entity: EntityId) -> Result<Option<T>> {
        if !self.entities.is_alive(entity) {
            return Err(anyhow::anyhow!("Entity {:?} not found", entity));
        }
        
        Ok(self.components.remove::<T>(entity))
    }
    
    /// Get a reference to a component
    pub fn get<T: super::Component>(&self, entity: EntityId) -> Option<&T> {
        self.components.get::<T>(entity)
    }
    
    /// Get a mutable reference to a component
    pub fn get_mut<T: super::Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        self.components.get_mut::<T>(entity)
    }
    
    /// Insert a resource
    pub fn insert_resource<T: super::Resource>(&mut self, resource: T) {
        self.resources.insert(resource);
    }
    
    /// Remove a resource
    pub fn remove_resource<T: super::Resource>(&mut self) -> Option<T> {
        self.resources.remove::<T>()
    }
    
    /// Get a reference to a resource
    pub fn get_resource<T: super::Resource>(&self) -> Option<&T> {
        self.resources.get::<T>()
    }
    
    /// Get a mutable reference to a resource
    pub fn get_resource_mut<T: super::Resource>(&mut self) -> Option<&mut T> {
        self.resources.get_mut::<T>()
    }
    
    /// Check if an entity is alive
    pub fn is_alive(&self, entity: EntityId) -> bool {
        self.entities.is_alive(entity)
    }
    
    /// Get the number of alive entities
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
    
    /// Send an event
    pub fn send_event<T: super::Event>(&mut self, event: T) {
        self.events.send(event);
    }
    
    /// Send an event with priority
    pub fn send_event_with_priority<T: super::Event>(&mut self, event: T, priority: super::event::EventPriority) {
        self.events.send_with_priority(event, priority);
    }
    
    /// Read events without consuming them
    pub fn read_events<T: super::Event>(&self) -> super::event::EventReader<'_, T> {
        self.events.read_events::<T>()
    }
    
    /// Consume events
    pub fn drain_events<T: super::Event>(&mut self) -> super::event::EventDrainer<T> {
        self.events.drain_events::<T>()
    }
    
    /// Process all pending events
    pub fn process_events(&mut self) {
        self.events.process_events();
    }
    
    /// Check if there are events of a specific type
    pub fn has_events<T: super::Event>(&self) -> bool {
        self.events.has_events::<T>()
    }
    
    /// Get the number of events of a specific type
    pub fn event_count<T: super::Event>(&self) -> usize {
        self.events.event_count::<T>()
    }
    
    /// Subscribe to events
    pub fn subscribe_to_events<T: super::Event, H: super::event::EventHandler + 'static>(&mut self, handler: H) {
        self.events.subscribe::<T, H>(handler);
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}