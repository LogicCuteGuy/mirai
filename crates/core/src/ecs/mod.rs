//! Entity Component System (ECS) framework for Mirai
//! 
//! Provides the core ECS types and functionality integrated with Mirai's existing architecture.
//! This module is adapted from minecraft-server-core to work seamlessly with Mirai's Instance
//! and BedrockClient systems.

pub mod world;
pub mod entity;
pub mod component;
pub mod resource;
pub mod system;
pub mod query;
pub mod event;
pub mod bridge;
pub mod adapters;
pub mod migration;

// Re-export core types
pub use world::World;
pub use entity::{Entity, EntityManager};
pub use component::ComponentManager;
pub use resource::ResourceManager;
pub use system::{System, SystemScheduler};
pub use query::Query;
pub use event::{Event, EventBus, EventPriority};
pub use bridge::{MiraiWorld, MiraiClientBridge, MiraiInstanceBridge, BedrockClientEcsExt};
pub use adapters::{InstanceAdapter, ClientAdapter, ServiceAdapter, CompatibilityLayer, MigrationUtilities};
pub use migration::{MigrationGuide, InstanceEcsExt};

/// Trait marker for types that can be used as components
pub trait Component: Send + Sync + 'static {}

/// Trait marker for types that can be used as resources
pub trait Resource: Send + Sync + 'static {}

/// Unique identifier for entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);

impl EntityId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
    
    pub fn id(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Entity({})", self.0)
    }
}