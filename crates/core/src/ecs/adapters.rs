//! Adapters for integrating existing Mirai systems with the new ECS framework

use super::{World, EntityId, Component, Resource};
use crate::instance::Instance;
use crate::net::BedrockClient;
use crate::plugin::App;
use anyhow::Result;
use std::sync::{Arc, Weak};
use std::collections::HashMap;

/// Adapter that wraps Mirai's Instance for ECS integration
pub struct InstanceAdapter {
    instance: Weak<Instance>,
}

impl Clone for InstanceAdapter {
    fn clone(&self) -> Self {
        Self {
            instance: self.instance.clone(),
        }
    }
}

impl InstanceAdapter {
    /// Create a new instance adapter
    pub fn new(instance: Weak<Instance>) -> Self {
        Self { instance }
    }
    
    /// Get the instance if it's still alive
    pub fn instance(&self) -> Option<Arc<Instance>> {
        self.instance.upgrade()
    }
    
    /// Check if the instance is still alive
    pub fn is_alive(&self) -> bool {
        self.instance.strong_count() > 0
    }
    
    /// Get instance configuration
    pub fn config(&self) -> Option<Arc<crate::config::Config>> {
        self.instance.upgrade().map(|i| i.config())
    }
    
    /// Get client count
    pub fn client_count(&self) -> usize {
        self.instance()
            .map(|i| i.clients().total_connected())
            .unwrap_or(0)
    }
}

impl Resource for InstanceAdapter {}

/// Adapter that wraps BedrockClient for ECS integration
#[derive(Debug)]
pub struct ClientAdapter {
    client: Weak<BedrockClient>,
    entity_id: Option<EntityId>,
}

impl ClientAdapter {
    /// Create a new client adapter
    pub fn new(client: Weak<BedrockClient>) -> Self {
        Self {
            client,
            entity_id: None,
        }
    }
    
    /// Set the associated ECS entity ID
    pub fn set_entity_id(&mut self, entity_id: EntityId) {
        self.entity_id = Some(entity_id);
    }
    
    /// Get the associated ECS entity ID
    pub fn entity_id(&self) -> Option<EntityId> {
        self.entity_id
    }
    
    /// Get the client if it's still alive
    pub fn client(&self) -> Option<Arc<BedrockClient>> {
        self.client.upgrade()
    }
    
    /// Check if the client is still alive
    pub fn is_alive(&self) -> bool {
        self.client.strong_count() > 0
    }
    
    /// Get client name
    pub fn name(&self) -> Option<String> {
        self.client()
            .and_then(|c| c.name().map(|s| s.to_string()).ok())
    }
    
    /// Get client runtime ID
    pub fn runtime_id(&self) -> Option<u64> {
        self.client()
            .and_then(|c| c.runtime_id().ok())
    }
    
    /// Check if client is initialized
    pub fn is_initialized(&self) -> bool {
        self.client()
            .map(|c| c.initialized())
            .unwrap_or(false)
    }
}

impl Component for ClientAdapter {}

/// Service adapter that provides access to Mirai's existing services
#[derive(Debug)]
pub struct ServiceAdapter {
    instance: Weak<Instance>,
}

impl Clone for ServiceAdapter {
    fn clone(&self) -> Self {
        Self {
            instance: self.instance.clone(),
        }
    }
}

impl ServiceAdapter {
    /// Create a new service adapter
    pub fn new(instance: Weak<Instance>) -> Self {
        Self { instance }
    }
    
    /// Get the command service
    pub fn commands(&self) -> Option<Arc<crate::command::Service>> {
        self.instance().map(|i| Arc::clone(i.commands()))
    }
    
    /// Get the level service
    pub fn level(&self) -> Option<Arc<crate::level::Service>> {
        self.instance().map(|i| Arc::clone(i.level()))
    }
    
    /// Get the client manager
    pub fn clients(&self) -> Option<Arc<crate::net::Clients>> {
        self.instance().map(|i| Arc::clone(i.clients()))
    }
    
    /// Get the instance if it's still alive
    fn instance(&self) -> Option<Arc<Instance>> {
        self.instance.upgrade()
    }
}

impl Resource for ServiceAdapter {}

/// Migration utilities for transitioning from old APIs to new ECS APIs
pub struct MigrationUtilities {
    world: *mut World,
    app: *mut App,
}

impl MigrationUtilities {
    /// Create new migration utilities
    /// 
    /// # Safety
    /// The provided pointers must be valid for the lifetime of this struct
    pub unsafe fn new(world: &mut World, app: &mut App) -> Self {
        Self {
            world: world as *mut World,
            app: app as *mut App,
        }
    }
    
    /// Get a reference to the world
    /// 
    /// # Safety
    /// This is safe as long as the utilities are used within the correct context
    pub fn world(&self) -> &World {
        unsafe { &*self.world }
    }
    
    /// Get a mutable reference to the world
    /// 
    /// # Safety
    /// This is safe as long as the utilities are used within the correct context
    pub fn world_mut(&mut self) -> &mut World {
        unsafe { &mut *self.world }
    }
    
    /// Get a reference to the app
    /// 
    /// # Safety
    /// This is safe as long as the utilities are used within the correct context
    pub fn app(&self) -> &App {
        unsafe { &*self.app }
    }
    
    /// Get a mutable reference to the app
    /// 
    /// # Safety
    /// This is safe as long as the utilities are used within the correct context
    pub fn app_mut(&mut self) -> &mut App {
        unsafe { &mut *self.app }
    }
    
    /// Migrate a BedrockClient to an ECS entity
    pub fn migrate_client_to_entity(&mut self, client: Arc<BedrockClient>) -> Result<EntityId> {
        let world = self.world_mut();
        let entity = world.spawn();
        
        // Add client adapter component
        let mut adapter = ClientAdapter::new(Arc::downgrade(&client));
        adapter.set_entity_id(entity);
        world.insert(entity, adapter)?;
        
        // Add Mirai client bridge component
        world.insert(entity, super::bridge::MiraiClientBridge {
            client: Arc::downgrade(&client),
        })?;
        
        Ok(entity)
    }
    
    /// Find entity associated with a client
    pub fn find_client_entity(&self, client: &Arc<BedrockClient>) -> Option<EntityId> {
        let world = self.world();
        
        // Search through all entities for the one with matching client
        for entity_id in 1..=world.entity_count() as u64 {
            let entity = EntityId::new(entity_id);
            if !world.is_alive(entity) {
                continue;
            }
            
            if let Some(adapter) = world.get::<ClientAdapter>(entity) {
                if let Some(entity_client) = adapter.client() {
                    if Arc::ptr_eq(client, &entity_client) {
                        return Some(entity);
                    }
                }
            }
        }
        
        None
    }
    
    /// Get all client entities
    pub fn get_client_entities(&self) -> Vec<EntityId> {
        let world = self.world();
        let mut entities = Vec::new();
        
        for entity_id in 1..=world.entity_count() as u64 {
            let entity = EntityId::new(entity_id);
            if !world.is_alive(entity) {
                continue;
            }
            
            if world.get::<ClientAdapter>(entity).is_some() {
                entities.push(entity);
            }
        }
        
        entities
    }
    
    /// Clean up entities for disconnected clients
    pub fn cleanup_disconnected_clients(&mut self) -> Result<usize> {
        let world = self.world_mut();
        let mut entities_to_remove = Vec::new();
        
        // Find entities with dead client references
        for entity_id in 1..=world.entity_count() as u64 {
            let entity = EntityId::new(entity_id);
            if !world.is_alive(entity) {
                continue;
            }
            
            if let Some(adapter) = world.get::<ClientAdapter>(entity) {
                if !adapter.is_alive() {
                    entities_to_remove.push(entity);
                }
            }
        }
        
        // Remove entities with dead clients
        let count = entities_to_remove.len();
        for entity in entities_to_remove {
            world.despawn(entity)?;
        }
        
        Ok(count)
    }
}

/// Compatibility layer that provides seamless integration between old and new APIs
pub struct CompatibilityLayer {
    instance_adapter: InstanceAdapter,
    service_adapter: ServiceAdapter,
    client_entities: HashMap<u64, EntityId>, // runtime_id -> entity_id mapping
}

impl CompatibilityLayer {
    /// Create a new compatibility layer
    pub fn new(instance: Weak<Instance>) -> Self {
        Self {
            instance_adapter: InstanceAdapter::new(instance.clone()),
            service_adapter: ServiceAdapter::new(instance),
            client_entities: HashMap::new(),
        }
    }
    
    /// Initialize the compatibility layer with the ECS world
    pub fn initialize(&mut self, world: &mut World) -> Result<()> {
        // Insert adapters as resources
        world.insert_resource(self.instance_adapter.clone());
        world.insert_resource(self.service_adapter.clone());
        
        Ok(())
    }
    
    /// Register a client entity mapping
    pub fn register_client_entity(&mut self, runtime_id: u64, entity_id: EntityId) {
        self.client_entities.insert(runtime_id, entity_id);
    }
    
    /// Unregister a client entity mapping
    pub fn unregister_client_entity(&mut self, runtime_id: u64) -> Option<EntityId> {
        self.client_entities.remove(&runtime_id)
    }
    
    /// Get entity ID for a client runtime ID
    pub fn get_client_entity(&self, runtime_id: u64) -> Option<EntityId> {
        self.client_entities.get(&runtime_id).copied()
    }
    
    /// Get all client entity mappings
    pub fn client_entities(&self) -> &HashMap<u64, EntityId> {
        &self.client_entities
    }
    
    /// Update the compatibility layer (should be called regularly)
    pub fn update(&mut self, world: &mut World) -> Result<()> {
        // Clean up dead client mappings
        let mut to_remove = Vec::new();
        
        for (&runtime_id, &entity_id) in &self.client_entities {
            if !world.is_alive(entity_id) {
                to_remove.push(runtime_id);
            }
        }
        
        for runtime_id in to_remove {
            self.client_entities.remove(&runtime_id);
        }
        
        Ok(())
    }
}

impl Resource for CompatibilityLayer {}

/// Helper functions for working with the compatibility layer
pub mod helpers {
    use super::*;
    
    /// Get the instance adapter from the world
    pub fn get_instance_adapter(world: &World) -> Option<&InstanceAdapter> {
        world.get_resource::<InstanceAdapter>()
    }
    
    /// Get the service adapter from the world
    pub fn get_service_adapter(world: &World) -> Option<&ServiceAdapter> {
        world.get_resource::<ServiceAdapter>()
    }
    
    /// Get the compatibility layer from the world
    pub fn get_compatibility_layer(world: &World) -> Option<&CompatibilityLayer> {
        world.get_resource::<CompatibilityLayer>()
    }
    
    /// Get a mutable reference to the compatibility layer from the world
    pub fn get_compatibility_layer_mut(world: &mut World) -> Option<&mut CompatibilityLayer> {
        world.get_resource_mut::<CompatibilityLayer>()
    }
    
    /// Create a system that maintains the compatibility layer
    pub fn create_compatibility_system() -> crate::ecs::system::FunctionSystem {
        crate::ecs::system::FunctionSystem::new("compatibility_maintenance", |world| {
            if let Some(mut compat) = world.remove_resource::<CompatibilityLayer>() {
                compat.update(world)?;
                world.insert_resource(compat);
            }
            Ok(())
        })
    }
    
    /// Create a system that cleans up disconnected clients
    pub fn create_cleanup_system() -> crate::ecs::system::FunctionSystem {
        crate::ecs::system::FunctionSystem::new("client_cleanup", |world| {
            let mut entities_to_remove = Vec::new();
            
            // Find entities with dead client references
            for entity_id in 1..=world.entity_count() as u64 {
                let entity = EntityId::new(entity_id);
                if !world.is_alive(entity) {
                    continue;
                }
                
                if let Some(adapter) = world.get::<ClientAdapter>(entity) {
                    if !adapter.is_alive() {
                        entities_to_remove.push(entity);
                    }
                }
            }
            
            // Remove entities with dead clients
            for entity in entities_to_remove {
                world.despawn(entity)?;
            }
            
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Weak;
    
    #[test]
    fn test_instance_adapter() {
        let instance: Weak<Instance> = Weak::new();
        let adapter = InstanceAdapter::new(instance);
        
        assert!(!adapter.is_alive());
        assert!(adapter.instance().is_none());
        assert_eq!(adapter.client_count(), 0);
    }
    
    #[test]
    fn test_client_adapter() {
        let client: Weak<BedrockClient> = Weak::new();
        let mut adapter = ClientAdapter::new(client);
        
        assert!(!adapter.is_alive());
        assert!(adapter.client().is_none());
        assert!(adapter.entity_id().is_none());
        
        let entity_id = EntityId::new(1);
        adapter.set_entity_id(entity_id);
        assert_eq!(adapter.entity_id(), Some(entity_id));
    }
    
    #[test]
    fn test_compatibility_layer() {
        let instance: Weak<Instance> = Weak::new();
        let mut compat = CompatibilityLayer::new(instance);
        
        let entity_id = EntityId::new(1);
        compat.register_client_entity(123, entity_id);
        
        assert_eq!(compat.get_client_entity(123), Some(entity_id));
        assert_eq!(compat.unregister_client_entity(123), Some(entity_id));
        assert_eq!(compat.get_client_entity(123), None);
    }
}