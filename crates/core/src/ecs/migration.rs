//! Migration utilities for transitioning existing Mirai code to use ECS

use super::{World, EntityId, Component, Resource, MiraiWorld, adapters::*};
use crate::instance::Instance;
use crate::net::BedrockClient;
use crate::plugin::App;
use anyhow::Result;
use std::sync::{Arc, Weak};

/// Migration guide and utilities for existing Mirai code
pub struct MigrationGuide {
    world: MiraiWorld,
    compatibility: CompatibilityLayer,
}

impl MigrationGuide {
    /// Create a new migration guide
    pub fn new(instance: Weak<Instance>) -> Self {
        let world = MiraiWorld::new(instance.clone());
        let compatibility = CompatibilityLayer::new(instance);
        
        Self {
            world,
            compatibility,
        }
    }
    
    /// Initialize the migration guide with the ECS world
    pub fn initialize(&mut self) -> Result<()> {
        self.compatibility.initialize(self.world.world_mut())?;
        Ok(())
    }
    
    /// Get the ECS world
    pub fn world(&self) -> &World {
        self.world.world()
    }
    
    /// Get the mutable ECS world
    pub fn world_mut(&mut self) -> &mut World {
        self.world.world_mut()
    }
    
    /// Get the Mirai world
    pub fn mirai_world(&self) -> &MiraiWorld {
        &self.world
    }
    
    /// Get the mutable Mirai world
    pub fn mirai_world_mut(&mut self) -> &mut MiraiWorld {
        &mut self.world
    }
    
    /// Migrate existing client handling to ECS
    pub fn migrate_client(&mut self, client: Arc<BedrockClient>) -> Result<EntityId> {
        let entity = self.world.spawn_client_entity(client.clone())?;
        
        // Register with compatibility layer
        if let Ok(runtime_id) = client.runtime_id() {
            self.compatibility.register_client_entity(runtime_id, entity);
        }
        
        Ok(entity)
    }
    
    /// Remove a client entity
    pub fn remove_client(&mut self, entity: EntityId) -> Result<()> {
        // Unregister from compatibility layer
        if let Some(adapter) = self.world.world().get::<ClientAdapter>(entity) {
            if let Some(client) = adapter.client() {
                if let Ok(runtime_id) = client.runtime_id() {
                    self.compatibility.unregister_client_entity(runtime_id);
                }
            }
        }
        
        self.world.world_mut().despawn(entity)
    }
    
    /// Update the migration guide (should be called regularly)
    pub fn update(&mut self) -> Result<()> {
        self.compatibility.update(self.world.world_mut())?;
        self.world.cleanup_dead_clients()?;
        Ok(())
    }
}

/// Extension trait for Instance to provide ECS integration
pub trait InstanceEcsExt {
    /// Create a migration guide for this instance
    fn create_migration_guide(&self) -> MigrationGuide;
    
    /// Create an ECS app for this instance
    fn create_ecs_app(&self) -> App;
}

impl InstanceEcsExt for Instance {
    fn create_migration_guide(&self) -> MigrationGuide {
        // Create a weak reference to self
        // Note: This is a simplified approach - in practice, you'd want to store
        // the Arc<Instance> somewhere accessible
        let weak_self = Weak::new(); // Placeholder
        MigrationGuide::new(weak_self)
    }
    
    fn create_ecs_app(&self) -> App {
        // Create a weak reference to self
        let weak_self = Weak::new(); // Placeholder
        App::new(weak_self)
    }
}

/// Helper macros for migrating existing code patterns

/// Migrate a client handler to an ECS system
#[macro_export]
macro_rules! migrate_client_handler {
    ($name:expr, $handler:expr) => {
        $crate::ecs::bridge::helpers::create_client_processor_system($name, $handler)
    };
}

/// Migrate an instance handler to an ECS system
#[macro_export]
macro_rules! migrate_instance_handler {
    ($name:expr, $handler:expr) => {
        $crate::ecs::bridge::helpers::create_instance_processor_system($name, $handler)
    };
}

/// Migration examples and patterns
pub mod examples {
    use super::*;
    use crate::ecs::system::FunctionSystem;
    
    /// Example: Migrate a client message handler to ECS
    pub fn migrate_message_handler() -> FunctionSystem {
        migrate_client_handler!("message_handler", |entity, client, world| {
            // Old code: client.send(message)?;
            // New code: Use ECS components and systems
            
            if let Some(adapter) = world.get::<ClientAdapter>(entity) {
                if let Some(client) = adapter.client() {
                    // Handle client messages using ECS patterns
                    tracing::debug!("Processing messages for client: {:?}", entity);
                }
            }
            
            Ok(())
        })
    }
    
    /// Example: Migrate server tick handler to ECS
    pub fn migrate_tick_handler() -> FunctionSystem {
        migrate_instance_handler!("tick_handler", |instance, world| {
            // Old code: Direct instance manipulation
            // New code: Use ECS resources and components
            
            tracing::debug!("Server tick for instance: {}", instance.config().name);
            
            // Process all client entities
            let client_count = world.entity_count();
            if client_count > 0 {
                tracing::debug!("Processing {} client entities", client_count);
            }
            
            Ok(())
        })
    }
    
    /// Example: Create a system that bridges old and new APIs
    pub fn create_bridge_system() -> FunctionSystem {
        FunctionSystem::new("api_bridge", |world| {
            // Get service adapter to access old APIs
            if let Some(services) = world.get_resource::<ServiceAdapter>() {
                if let Some(commands) = services.commands() {
                    // Bridge command system with ECS
                    tracing::debug!("Bridging command system with ECS");
                }
                
                if let Some(level) = services.level() {
                    // Bridge level system with ECS
                    tracing::debug!("Bridging level system with ECS");
                }
            }
            
            Ok(())
        })
    }
}

/// Migration checklist and best practices
pub mod checklist {
    //! Migration checklist for converting existing Mirai code to ECS
    //! 
    //! ## Phase 1: Preparation
    //! - [ ] Identify existing systems that need migration
    //! - [ ] Create compatibility layer for critical systems
    //! - [ ] Set up ECS world and basic components
    //! 
    //! ## Phase 2: Component Migration
    //! - [ ] Convert data structures to ECS components
    //! - [ ] Create adapters for existing APIs
    //! - [ ] Implement migration utilities
    //! 
    //! ## Phase 3: System Migration
    //! - [ ] Convert handlers to ECS systems
    //! - [ ] Implement system scheduling
    //! - [ ] Test system interactions
    //! 
    //! ## Phase 4: Integration
    //! - [ ] Integrate with existing Mirai instance
    //! - [ ] Test client connections and disconnections
    //! - [ ] Verify performance characteristics
    //! 
    //! ## Phase 5: Cleanup
    //! - [ ] Remove old code paths
    //! - [ ] Update documentation
    //! - [ ] Performance optimization
    
    use super::*;
    
    /// Validate that migration is complete and working
    pub fn validate_migration(world: &World) -> Result<MigrationStatus> {
        let mut status = MigrationStatus::default();
        
        // Check if compatibility layer is present
        status.has_compatibility_layer = world.get_resource::<CompatibilityLayer>().is_some();
        
        // Check if adapters are present
        status.has_instance_adapter = world.get_resource::<InstanceAdapter>().is_some();
        status.has_service_adapter = world.get_resource::<ServiceAdapter>().is_some();
        
        // Count client entities
        status.client_entity_count = world.entity_count();
        
        // Check for common components
        let mut has_client_components = false;
        for entity_id in 1..=world.entity_count() as u64 {
            let entity = EntityId::new(entity_id);
            if world.is_alive(entity) && world.get::<ClientAdapter>(entity).is_some() {
                has_client_components = true;
                break;
            }
        }
        status.has_client_components = has_client_components;
        
        Ok(status)
    }
    
    /// Migration status information
    #[derive(Debug, Default)]
    pub struct MigrationStatus {
        pub has_compatibility_layer: bool,
        pub has_instance_adapter: bool,
        pub has_service_adapter: bool,
        pub client_entity_count: usize,
        pub has_client_components: bool,
    }
    
    impl MigrationStatus {
        /// Check if migration appears to be complete
        pub fn is_complete(&self) -> bool {
            self.has_compatibility_layer
                && self.has_instance_adapter
                && self.has_service_adapter
        }
        
        /// Get migration progress as a percentage
        pub fn progress(&self) -> f32 {
            let mut completed = 0;
            let total = 5;
            
            if self.has_compatibility_layer { completed += 1; }
            if self.has_instance_adapter { completed += 1; }
            if self.has_service_adapter { completed += 1; }
            if self.client_entity_count > 0 { completed += 1; }
            if self.has_client_components { completed += 1; }
            
            (completed as f32 / total as f32) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Weak;
    
    #[test]
    fn test_migration_guide_creation() {
        let instance: Weak<Instance> = Weak::new();
        let mut guide = MigrationGuide::new(instance);
        
        // Should be able to initialize
        assert!(guide.initialize().is_ok());
        
        // Should have access to world
        assert_eq!(guide.world().entity_count(), 0);
    }
    
    #[test]
    fn test_migration_status() {
        use checklist::*;
        
        let status = MigrationStatus::default();
        assert!(!status.is_complete());
        assert_eq!(status.progress(), 0.0);
        
        let complete_status = MigrationStatus {
            has_compatibility_layer: true,
            has_instance_adapter: true,
            has_service_adapter: true,
            client_entity_count: 1,
            has_client_components: true,
        };
        assert!(complete_status.is_complete());
        assert_eq!(complete_status.progress(), 100.0);
    }
}