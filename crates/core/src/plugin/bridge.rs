//! Bridge between Mirai-specific functionality and the plugin system

use super::{Plugin, PluginInfo, App};
use crate::instance::Instance;
use crate::net::BedrockClient;
use crate::ecs::{World, EntityId};
use anyhow::Result;
use std::sync::{Arc, Weak};

/// Trait for Mirai-specific plugins
pub trait MiraiPlugin: Plugin {
    /// Get the plugin category
    fn category(&self) -> PluginCategory {
        PluginCategory::Gameplay
    }
    
    /// Get the minimum Mirai version required
    fn min_mirai_version(&self) -> semver::Version {
        semver::Version::new(0, 1, 0)
    }
    
    /// Get the maximum Mirai version supported
    fn max_mirai_version(&self) -> Option<semver::Version> {
        None
    }
    
    /// Check if this plugin is compatible with the given Mirai version
    fn is_compatible_with(&self, mirai_version: &semver::Version) -> bool {
        if mirai_version < &self.min_mirai_version() {
            return false;
        }
        
        if let Some(max_version) = self.max_mirai_version() {
            if mirai_version > &max_version {
                return false;
            }
        }
        
        true
    }
    
    /// Called when a client connects (optional)
    fn on_client_connect(&self, _client: &Arc<BedrockClient>, _world: &mut World) -> Result<()> {
        Ok(())
    }
    
    /// Called when a client disconnects (optional)
    fn on_client_disconnect(&self, _client: &Arc<BedrockClient>, _world: &mut World) -> Result<()> {
        Ok(())
    }
    
    /// Called when the server starts (optional)
    fn on_server_start(&self, _instance: &Arc<Instance>) -> Result<()> {
        Ok(())
    }
    
    /// Called when the server stops (optional)
    fn on_server_stop(&self, _instance: &Arc<Instance>) -> Result<()> {
        Ok(())
    }
}

/// Categories for Mirai plugins
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginCategory {
    /// Core server functionality
    Core,
    /// Gameplay mechanics
    Gameplay,
    /// World generation and management
    World,
    /// Networking and protocol
    Network,
    /// User interface and commands
    Interface,
    /// Performance and monitoring
    Performance,
    /// Third-party integrations
    Integration,
    /// Development and debugging tools
    Development,
}

impl std::fmt::Display for PluginCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core => write!(f, "Core"),
            Self::Gameplay => write!(f, "Gameplay"),
            Self::World => write!(f, "World"),
            Self::Network => write!(f, "Network"),
            Self::Interface => write!(f, "Interface"),
            Self::Performance => write!(f, "Performance"),
            Self::Integration => write!(f, "Integration"),
            Self::Development => write!(f, "Development"),
        }
    }
}

/// Context provided to Mirai plugins
pub struct MiraiPluginContext {
    /// Reference to the Mirai instance
    pub instance: Weak<Instance>,
    /// Reference to the ECS world
    pub world: *mut World,
}

impl MiraiPluginContext {
    /// Create a new plugin context
    pub fn new(instance: Weak<Instance>, world: &mut World) -> Self {
        Self {
            instance,
            world: world as *mut World,
        }
    }
    
    /// Get the Mirai instance if it's still alive
    pub fn instance(&self) -> Option<Arc<Instance>> {
        self.instance.upgrade()
    }
    
    /// Get a reference to the world
    /// 
    /// # Safety
    /// This is safe as long as the context is used within the plugin's execution context
    pub fn world(&self) -> &World {
        unsafe { &*self.world }
    }
    
    /// Get a mutable reference to the world
    /// 
    /// # Safety
    /// This is safe as long as the context is used within the plugin's execution context
    pub fn world_mut(&mut self) -> &mut World {
        unsafe { &mut *self.world }
    }
}

/// Helper macro for implementing MiraiPlugin
#[macro_export]
macro_rules! mirai_plugin {
    (
        $plugin_type:ty,
        name = $name:expr,
        version = $version:expr,
        category = $category:expr
    ) => {
        impl $crate::plugin::Plugin for $plugin_type {
            fn info(&self) -> $crate::plugin::PluginInfo {
                $crate::plugin::PluginInfo::new($name, semver::Version::parse($version).expect("Invalid version"))
                    .with_description(String::new())
                    .with_author(String::new())
            }
            
            fn build(&self, app: &mut $crate::plugin::App) -> anyhow::Result<()> {
                self.build_plugin(app)
            }
        }
        
        impl $crate::plugin::MiraiPlugin for $plugin_type {
            fn category(&self) -> $crate::plugin::PluginCategory {
                $category
            }
        }
    };
    
    (
        $plugin_type:ty,
        name = $name:expr,
        version = $version:expr,
        category = $category:expr,
        min_mirai_version = $min_version:expr
    ) => {
        mirai_plugin!($plugin_type, name = $name, version = $version, category = $category);
        
        impl $crate::plugin::MiraiPlugin for $plugin_type {
            fn category(&self) -> $crate::plugin::PluginCategory {
                $category
            }
            
            fn min_mirai_version(&self) -> semver::Version {
                semver::Version::parse($min_version).expect("Invalid minimum Mirai version")
            }
        }
    };
}

/// Helper functions for creating Mirai-specific systems
pub mod systems {
    use super::*;
    use crate::ecs::{system::FunctionSystem, MiraiClientBridge};
    
    /// Create a system that processes all client entities
    pub fn create_client_system<F>(name: &str, processor: F) -> FunctionSystem
    where
        F: Fn(EntityId, &Arc<BedrockClient>, &mut World) -> Result<()> + Send + Sync + 'static,
    {
        let name = name.to_string();
        FunctionSystem::new(name, move |world: &mut World| {
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
    pub fn create_instance_system<F>(name: &str, processor: F) -> FunctionSystem
    where
        F: Fn(&Arc<Instance>, &mut World) -> Result<()> + Send + Sync + 'static,
    {
        let name = name.to_string();
        FunctionSystem::new(name, move |world: &mut World| {
            if let Some(instance_resource) = world.get_resource::<crate::ecs::bridge::MiraiInstanceResource>() {
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

/// Example Mirai plugin implementation
pub struct ExampleMiraiPlugin {
    name: String,
}

impl ExampleMiraiPlugin {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
        }
    }
    
    pub fn build_plugin(&self, app: &mut App) -> Result<()> {
        // Add a simple system that logs every 100 ticks
        app.add_system(systems::create_instance_system("example_logger", |instance, _world| {
            tracing::debug!("Example plugin running on instance: {}", instance.config().name);
            Ok(())
        }));
        
        Ok(())
    }
}

impl Plugin for ExampleMiraiPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new(&self.name, semver::Version::new(1, 0, 0))
            .with_description("Example Mirai plugin")
            .with_author("Mirai Core")
    }
    
    fn build(&self, app: &mut App) -> Result<()> {
        self.build_plugin(app)
    }
}

impl MiraiPlugin for ExampleMiraiPlugin {
    fn category(&self) -> PluginCategory {
        PluginCategory::Development
    }
    
    fn on_server_start(&self, instance: &Arc<Instance>) -> Result<()> {
        tracing::info!("Example plugin started on server: {}", instance.config().name);
        Ok(())
    }
    
    fn on_server_stop(&self, instance: &Arc<Instance>) -> Result<()> {
        tracing::info!("Example plugin stopped on server: {}", instance.config().name);
        Ok(())
    }
}