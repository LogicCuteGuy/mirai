//! Resource management for the ECS system

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Manages global resources for the ECS world
#[derive(Debug)]
pub struct ResourceManager {
    resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }
    
    /// Insert a resource
    pub fn insert<T: super::Resource>(&mut self, resource: T) {
        let type_id = TypeId::of::<T>();
        self.resources.insert(type_id, Box::new(resource));
    }
    
    /// Remove a resource
    pub fn remove<T: super::Resource>(&mut self) -> Option<T> {
        let type_id = TypeId::of::<T>();
        self.resources.remove(&type_id)?
            .downcast::<T>()
            .ok()
            .map(|boxed| *boxed)
    }
    
    /// Get a reference to a resource
    pub fn get<T: super::Resource>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.resources.get(&type_id)?
            .downcast_ref::<T>()
    }
    
    /// Get a mutable reference to a resource
    pub fn get_mut<T: super::Resource>(&mut self) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        self.resources.get_mut(&type_id)?
            .downcast_mut::<T>()
    }
    
    /// Check if a resource exists
    pub fn contains<T: super::Resource>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        self.resources.contains_key(&type_id)
    }
    
    /// Get the number of resources
    pub fn len(&self) -> usize {
        self.resources.len()
    }
    
    /// Check if there are no resources
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }
    
    /// Clear all resources
    pub fn clear(&mut self) {
        self.resources.clear();
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}