//! Query system for efficient component access

use super::{World, EntityId};
use std::marker::PhantomData;

/// A query for accessing components from entities
pub struct Query<'w, T> {
    world: &'w World,
    _phantom: PhantomData<T>,
}

impl<'w, T> Query<'w, T> {
    /// Create a new query
    pub fn new(world: &'w World) -> Self {
        Self {
            world,
            _phantom: PhantomData,
        }
    }
}

/// Query for a single component type
impl<'w, T: super::Component> Query<'w, &'w T> {
    /// Get a component for an entity
    pub fn get(&self, entity: EntityId) -> Option<&'w T> {
        self.world.get::<T>(entity)
    }
    
    /// Iterate over all entities with this component
    pub fn iter(&self) -> QueryIter<'w, T> {
        QueryIter::new(self.world)
    }
    
    /// Get the first entity with this component
    pub fn single(&self) -> Option<(EntityId, &'w T)> {
        self.iter().next()
    }
}

/// Iterator for query results
pub struct QueryIter<'w, T> {
    world: &'w World,
    entities: Vec<EntityId>,
    current: usize,
    _phantom: PhantomData<T>,
}

impl<'w, T: super::Component> QueryIter<'w, T> {
    fn new(world: &'w World) -> Self {
        // In a real implementation, this would be more efficient
        // For now, we'll iterate through all entities
        let entities: Vec<EntityId> = (1..=world.entity_count() as u64)
            .map(EntityId::new)
            .filter(|&e| world.is_alive(e))
            .collect();
        
        Self {
            world,
            entities,
            current: 0,
            _phantom: PhantomData,
        }
    }
}

impl<'w, T: super::Component> Iterator for QueryIter<'w, T> {
    type Item = (EntityId, &'w T);
    
    fn next(&mut self) -> Option<Self::Item> {
        while self.current < self.entities.len() {
            let entity = self.entities[self.current];
            self.current += 1;
            
            if let Some(component) = self.world.get::<T>(entity) {
                return Some((entity, component));
            }
        }
        None
    }
}

/// Query for a mutable single component type
pub struct QueryMut<'w, T> {
    world: &'w mut World,
    _phantom: PhantomData<T>,
}

impl<'w, T> QueryMut<'w, T> {
    /// Create a new mutable query
    pub fn new(world: &'w mut World) -> Self {
        Self {
            world,
            _phantom: PhantomData,
        }
    }
}

impl<'w, T: super::Component> QueryMut<'w, &'w mut T> {
    /// Get a mutable component for an entity
    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut T> {
        self.world.get_mut::<T>(entity)
    }
    
    /// Iterate over all entities with this component (mutable)
    pub fn iter_mut(&mut self) -> QueryIterMut<'_, T> {
        QueryIterMut::new(self.world)
    }
    
    /// Get the first entity with this component (mutable)
    pub fn single_mut(&mut self) -> Option<(EntityId, &mut T)> {
        self.iter_mut().next()
    }
}

/// Mutable iterator for query results
pub struct QueryIterMut<'w, T> {
    world: &'w mut World,
    entities: Vec<EntityId>,
    current: usize,
    _phantom: PhantomData<T>,
}

impl<'w, T: super::Component> QueryIterMut<'w, T> {
    fn new(world: &'w mut World) -> Self {
        // Get entities that are alive
        let entities: Vec<EntityId> = (1..=world.entity_count() as u64)
            .map(EntityId::new)
            .filter(|&e| world.is_alive(e))
            .collect();
        
        Self {
            world,
            entities,
            current: 0,
            _phantom: PhantomData,
        }
    }
}

impl<'w, T: super::Component> Iterator for QueryIterMut<'w, T> {
    type Item = (EntityId, &'w mut T);
    
    fn next(&mut self) -> Option<Self::Item> {
        while self.current < self.entities.len() {
            let entity = self.entities[self.current];
            self.current += 1;
            
            // This is unsafe but necessary for the current design
            // In a real implementation, we'd need a more sophisticated approach
            unsafe {
                let world_ptr = self.world as *mut World;
                if let Some(component) = (*world_ptr).get_mut::<T>(entity) {
                    return Some((entity, component));
                }
            }
        }
        None
    }
}

/// Multi-component query for two components
pub struct Query2<'w, T1, T2> {
    world: &'w World,
    _phantom: PhantomData<(T1, T2)>,
}

impl<'w, T1: super::Component, T2: super::Component> Query2<'w, T1, T2> {
    /// Create a new two-component query
    pub fn new(world: &'w World) -> Self {
        Self {
            world,
            _phantom: PhantomData,
        }
    }
    
    /// Get components for an entity
    pub fn get(&self, entity: EntityId) -> Option<(&'w T1, &'w T2)> {
        let comp1 = self.world.get::<T1>(entity)?;
        let comp2 = self.world.get::<T2>(entity)?;
        Some((comp1, comp2))
    }
    
    /// Iterate over all entities with both components
    pub fn iter(&self) -> Query2Iter<'w, T1, T2> {
        Query2Iter::new(self.world)
    }
}

/// Iterator for two-component queries
pub struct Query2Iter<'w, T1, T2> {
    world: &'w World,
    entities: Vec<EntityId>,
    current: usize,
    _phantom: PhantomData<(T1, T2)>,
}

impl<'w, T1: super::Component, T2: super::Component> Query2Iter<'w, T1, T2> {
    fn new(world: &'w World) -> Self {
        let entities: Vec<EntityId> = (1..=world.entity_count() as u64)
            .map(EntityId::new)
            .filter(|&e| world.is_alive(e))
            .collect();
        
        Self {
            world,
            entities,
            current: 0,
            _phantom: PhantomData,
        }
    }
}

impl<'w, T1: super::Component, T2: super::Component> Iterator for Query2Iter<'w, T1, T2> {
    type Item = (EntityId, &'w T1, &'w T2);
    
    fn next(&mut self) -> Option<Self::Item> {
        while self.current < self.entities.len() {
            let entity = self.entities[self.current];
            self.current += 1;
            
            if let (Some(comp1), Some(comp2)) = (
                self.world.get::<T1>(entity),
                self.world.get::<T2>(entity)
            ) {
                return Some((entity, comp1, comp2));
            }
        }
        None
    }
}

/// Query builder for more complex queries
pub struct QueryBuilder<'w> {
    world: &'w World,
}

impl<'w> QueryBuilder<'w> {
    /// Create a new query builder
    pub fn new(world: &'w World) -> Self {
        Self { world }
    }
    
    /// Build a query for a single component type
    pub fn with<T: super::Component>(self) -> Query<'w, &'w T> {
        Query::new(self.world)
    }
    
    /// Build a query for two component types
    pub fn with2<T1: super::Component, T2: super::Component>(self) -> Query2<'w, T1, T2> {
        Query2::new(self.world)
    }
}

/// Mutable query builder
pub struct QueryBuilderMut<'w> {
    world: &'w mut World,
}

impl<'w> QueryBuilderMut<'w> {
    /// Create a new mutable query builder
    pub fn new(world: &'w mut World) -> Self {
        Self { world }
    }
    
    /// Build a mutable query for a single component type
    pub fn with_mut<T: super::Component>(self) -> QueryMut<'w, &'w mut T> {
        QueryMut::new(self.world)
    }
}

/// Extension trait for World to provide query functionality
pub trait WorldQueryExt {
    /// Create a query builder
    fn query(&self) -> QueryBuilder<'_>;
    
    /// Create a mutable query builder
    fn query_mut(&mut self) -> QueryBuilderMut<'_>;
}

impl WorldQueryExt for World {
    fn query(&self) -> QueryBuilder<'_> {
        QueryBuilder::new(self)
    }
    
    fn query_mut(&mut self) -> QueryBuilderMut<'_> {
        QueryBuilderMut::new(self)
    }
}