//! Bridge between Mirai's existing systems and the new ECS framework

use super::{World, EntityId, Component, Resource};
use crate::instance::Instance;
use crate::net::BedrockClient;
use std::sync::{Arc, Weak};
use anyhow::Result;

/// Bridge component that connects ECS entities to Mirai's BedrockClient
#[derive(Debug)]
pub struct MiraiClientBridge {
    pub client: Weak<BedrockClient>,
}

impl Component for MiraiClientBridge {}

/// Bridge component that connects ECS entities to Mirai's Instance
#[derive(Debug)]
pub struct MiraiInstanceBridge {
    pub instance: Weak<Instance>,
}

impl Component for MiraiInstanceBridge {}

/// Resource that holds the main Mirai instance
#[derive(Debug)]
pub struct MiraiInstanceResource {
    pub instance: Weak<Instance>,
}

impl Resource for MiraiInstanceResource {}

/// Enhanced World that integrates with Mirai's existing architecture
pub struct MiraiWorld {
    pub ecs_world: World,
    pub instance: Weak<Instance>,
}

impl MiraiWorld {
    /// Create a new Mirai-integrated ECS world
    pub fn new(instance: Weak<Instance>) -> Self {
        let mut ecs_world = World::new();
        
        // Insert the instance as a resource
        ecs_world.insert_resource(MiraiInstanceResource {
            instance: instance.clone(),
        });
        
        Self {
            ecs_world,
            instance,
        }
    }
    
    /// Spawn an entity that represents a Mirai client
    pub fn spawn_client_entity(&mut self, client: Arc<BedrockClient>) -> Result<EntityId> {
        let entity = self.ecs_world.spawn();
        
        // Add the client bridge component
        self.ecs_world.insert(entity, MiraiClientBridge {
            client: Arc::downgrade(&client),
        })?;
        
        // Add the instance bridge component
        self.ecs_world.insert(entity, MiraiInstanceBridge {
            instance: self.instance.clone(),
        })?;
        
        Ok(entity)
    }
    
    /// Get the Mirai instance if it's still alive
    pub fn get_instance(&self) -> Option<Arc<Instance>> {
        self.instance.upgrade()
    }
    
    /// Get a client from an entity if it exists and is still alive
    pub fn get_client(&self, entity: EntityId) -> Option<Arc<BedrockClient>> {
        let bridge = self.ecs_world.get::<MiraiClientBridge>(entity)?;
        bridge.client.upgrade()
    }
    
    /// Update all client entities, removing those with dead clients
    pub fn cleanup_dead_clients(&mut self) -> Result<()> {
        let mut entities_to_remove = Vec::new();
        
        // Find entities with dead client references
        for entity_id in 1..=self.ecs_world.entity_count() as u64 {
            let entity = EntityId::new(entity_id);
            if !self.ecs_world.is_alive(entity) {
                continue;
            }
            
            if let Some(bridge) = self.ecs_world.get::<MiraiClientBridge>(entity) {
                if bridge.client.upgrade().is_none() {
                    entities_to_remove.push(entity);
                }
            }
        }
        
        // Remove entities with dead clients
        for entity in entities_to_remove {
            self.ecs_world.despawn(entity)?;
        }
        
        Ok(())
    }
    
    /// Get access to the underlying ECS world
    pub fn world(&self) -> &World {
        &self.ecs_world
    }
    
    /// Get mutable access to the underlying ECS world
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.ecs_world
    }
}

/// Extension trait for BedrockClient to work with ECS
pub trait BedrockClientEcsExt {
    /// Get the ECS entity ID associated with this client, if any
    fn ecs_entity(&self) -> Option<EntityId>;
    
    /// Set the ECS entity ID for this client
    fn set_ecs_entity(&self, entity: EntityId);
}

// Note: This would require modifications to BedrockClient to store the entity ID
// For now, we'll provide a placeholder implementation
impl BedrockClientEcsExt for BedrockClient {
    fn ecs_entity(&self) -> Option<EntityId> {
        // TODO: This would need to be stored in BedrockClient
        // For now, return None as a placeholder
        None
    }
    
    fn set_ecs_entity(&self, _entity: EntityId) {
        // TODO: This would need to store the entity ID in BedrockClient
        // For now, this is a no-op
    }
}

/// Helper functions for working with Mirai ECS integration
pub mod helpers {
    use super::*;
    
    /// Create a system that processes all client entities
    pub fn create_client_processor_system<F>(name: &str, processor: F) -> crate::ecs::system::FunctionSystem
    where
        F: Fn(EntityId, &Arc<BedrockClient>, &mut World) -> Result<()> + Send + Sync + 'static,
    {
        let name = name.to_string();
        crate::ecs::system::FunctionSystem::new(name, move |world: &mut World| {
            let mut clients_to_process = Vec::new();
            
            // Collect all client entities
            for entity_id in 1..=world.entity_count() as u64 {
                let entity = EntityId::new(entity_id);
                if !world.is_alive(entity) {
                    continue;
                }
                
                if let Some(bridge) = world.get::<MiraiClientBridge>(entity) {
                    if let Some(client) = bridge.client.upgrade() {
                        clients_to_process.push((entity, client));
                    }
                }
            }
            
            // Process each client
            for (entity, client) in clients_to_process {
                if let Err(e) = processor(entity, &client, world) {
                    tracing::error!("Error processing client entity {:?}: {}", entity, e);
                }
            }
            
            Ok(())
        })
    }
    
    /// Create a system that processes the Mirai instance
    pub fn create_instance_processor_system<F>(name: &str, processor: F) -> crate::ecs::system::FunctionSystem
    where
        F: Fn(&Arc<Instance>, &mut World) -> Result<()> + Send + Sync + 'static,
    {
        let name = name.to_string();
        crate::ecs::system::FunctionSystem::new(name, move |world: &mut World| {
            if let Some(instance_resource) = world.get_resource::<MiraiInstanceResource>() {
                if let Some(instance) = instance_resource.instance.upgrade() {
                    if let Err(e) = processor(&instance, world) {
                        tracing::error!("Error processing instance: {}", e);
                    }
                }
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
    fn test_mirai_world_creation() {
        let instance: Weak<Instance> = Weak::new();
        let world = MiraiWorld::new(instance);
        
        assert!(world.get_instance().is_none());
        assert!(world.world().get_resource::<MiraiInstanceResource>().is_some());
    }
    
    #[test]
    fn test_cleanup_dead_clients() {
        let instance: Weak<Instance> = Weak::new();
        let mut world = MiraiWorld::new(instance);
        
        // This test would need actual BedrockClient instances to be meaningful
        // For now, just test that cleanup doesn't crash
        let result = world.cleanup_dead_clients();
        assert!(result.is_ok());
    }
}