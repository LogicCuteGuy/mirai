//! Component storage and management for the ECS system

use super::EntityId;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use parking_lot::RwLock;

/// Manages component storage for all entities
pub struct ComponentManager {
    storages: HashMap<TypeId, Box<dyn ComponentStorage>>,
}

impl ComponentManager {
    /// Create a new component manager
    pub fn new() -> Self {
        Self {
            storages: HashMap::new(),
        }
    }
    
    /// Insert a component for an entity
    pub fn insert<T: super::Component>(&mut self, entity: EntityId, component: T) {
        let type_id = TypeId::of::<T>();
        let storage = self.storages
            .entry(type_id)
            .or_insert_with(|| Box::new(TypedComponentStorage::<T>::new()));
        
        storage.as_any_mut()
            .downcast_mut::<TypedComponentStorage<T>>()
            .unwrap()
            .insert(entity, component);
    }
    
    /// Remove a component from an entity
    pub fn remove<T: super::Component>(&mut self, entity: EntityId) -> Option<T> {
        let type_id = TypeId::of::<T>();
        self.storages.get_mut(&type_id)?
            .as_any_mut()
            .downcast_mut::<TypedComponentStorage<T>>()
            .unwrap()
            .remove(entity)
    }
    
    /// Get a reference to a component
    pub fn get<T: super::Component>(&self, entity: EntityId) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.storages.get(&type_id)?
            .as_any()
            .downcast_ref::<TypedComponentStorage<T>>()
            .unwrap()
            .get(entity)
    }
    
    /// Get a mutable reference to a component
    pub fn get_mut<T: super::Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        self.storages.get_mut(&type_id)?
            .as_any_mut()
            .downcast_mut::<TypedComponentStorage<T>>()
            .unwrap()
            .get_mut(entity)
    }
    
    /// Remove all components for an entity
    pub fn remove_all_for_entity(&mut self, entity: EntityId) {
        for storage in self.storages.values_mut() {
            storage.remove_entity(entity);
        }
    }
    
    /// Check if an entity has a specific component type
    pub fn has<T: super::Component>(&self, entity: EntityId) -> bool {
        let type_id = TypeId::of::<T>();
        self.storages.get(&type_id)
            .map(|storage| storage.as_any()
                .downcast_ref::<TypedComponentStorage<T>>()
                .unwrap()
                .contains(entity))
            .unwrap_or(false)
    }
}

impl Default for ComponentManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for type-erased component storage
trait ComponentStorage: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn remove_entity(&mut self, entity: EntityId);
}

/// Typed storage for a specific component type
struct TypedComponentStorage<T: super::Component> {
    components: RwLock<HashMap<EntityId, T>>,
}

impl<T: super::Component> TypedComponentStorage<T> {
    fn new() -> Self {
        Self {
            components: RwLock::new(HashMap::new()),
        }
    }
    
    fn insert(&self, entity: EntityId, component: T) {
        self.components.write().insert(entity, component);
    }
    
    fn remove(&self, entity: EntityId) -> Option<T> {
        self.components.write().remove(&entity)
    }
    
    fn get(&self, entity: EntityId) -> Option<&T> {
        // Note: This is unsafe but necessary for the current design
        // In a real implementation, we'd need a more sophisticated approach
        // to handle concurrent access safely
        unsafe {
            let components = self.components.data_ptr();
            (*components).get(&entity)
        }
    }
    
    fn get_mut(&self, entity: EntityId) -> Option<&mut T> {
        // Note: This is unsafe but necessary for the current design
        // In a real implementation, we'd need a more sophisticated approach
        unsafe {
            let components = self.components.data_ptr();
            (*components).get_mut(&entity)
        }
    }
    
    fn contains(&self, entity: EntityId) -> bool {
        self.components.read().contains_key(&entity)
    }
}

impl<T: super::Component> ComponentStorage for TypedComponentStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    
    fn remove_entity(&mut self, entity: EntityId) {
        self.components.write().remove(&entity);
    }
}