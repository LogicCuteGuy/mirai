//! Application builder that manages plugins and systems for Mirai

use super::{Plugin, PluginInfo, PluginState, PluginHandle};
use crate::ecs::{World, System, SystemScheduler, MiraiWorld};
use crate::instance::Instance;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};

/// Application builder that manages plugins and systems
/// 
/// The App is the central coordinator for the plugin system, managing
/// plugin registration, dependency resolution, and lifecycle management.
/// It integrates with Mirai's existing Instance and ECS systems.
pub struct App {
    /// ECS world integrated with Mirai
    world: MiraiWorld,
    /// System scheduler for running systems
    scheduler: SystemScheduler,
    /// Plugin handles indexed by name
    plugins: HashMap<String, PluginHandle>,
    /// Plugin load order for dependency resolution
    plugin_load_order: Vec<String>,
    /// Set of enabled plugins
    enabled_plugins: HashSet<String>,
    /// Reference to the Mirai instance
    instance: Weak<Instance>,
}

impl App {
    /// Create a new application with Mirai integration
    pub fn new(instance: Weak<Instance>) -> Self {
        Self {
            world: MiraiWorld::new(instance.clone()),
            scheduler: SystemScheduler::new(),
            plugins: HashMap::new(),
            plugin_load_order: Vec::new(),
            enabled_plugins: HashSet::new(),
            instance,
        }
    }
    
    /// Add a plugin to the application
    /// 
    /// This method registers a plugin and resolves its dependencies.
    /// The plugin will be loaded and built immediately if all dependencies are satisfied.
    pub fn add_plugin<P: Plugin + 'static>(&mut self, plugin: P) -> Result<&mut Self> {
        let info = plugin.info();
        let name = info.name.clone();
        
        // Check if plugin is already loaded
        if self.plugins.contains_key(&name) {
            return Err(anyhow::anyhow!("Plugin '{}' already loaded", name));
        }
        
        // Check dependencies
        self.check_dependencies(&info)?;
        
        // Create plugin handle
        let mut handle = PluginHandle::new(Box::new(plugin));
        
        // Load the plugin
        handle.load().map_err(|e| anyhow::anyhow!("Failed to load plugin '{}': {}", name, e))?;
        
        // Build and enable the plugin
        handle.enable(self).map_err(|e| anyhow::anyhow!("Failed to enable plugin '{}': {}", name, e))?;
        
        // Store the plugin
        self.plugins.insert(name.clone(), handle);
        self.plugin_load_order.push(name.clone());
        self.enabled_plugins.insert(name.clone());
        
        tracing::info!("Added plugin: {} v{}", name, info.version);
        
        Ok(self)
    }
    
    /// Add multiple plugins in dependency order
    /// 
    /// This method adds multiple plugins and resolves their dependencies
    /// automatically, loading them in the correct order.
    pub fn add_plugins<I>(&mut self, plugins: I) -> Result<&mut Self>
    where
        I: IntoIterator<Item = Box<dyn Plugin>>,
    {
        let mut pending_plugins: Vec<Box<dyn Plugin>> = plugins.into_iter().collect();
        let mut added_plugins = HashSet::new();
        
        // Keep trying to add plugins until all are added or we can't make progress
        while !pending_plugins.is_empty() {
            let initial_count = pending_plugins.len();
            let mut i = 0;
            
            while i < pending_plugins.len() {
                let plugin_info = pending_plugins[i].info();
                
                // Check if all dependencies are satisfied
                let dependencies_satisfied = plugin_info.dependencies.iter()
                    .all(|dep| !dep.optional && (added_plugins.contains(&dep.name) || self.has_plugin(&dep.name)));
                
                if dependencies_satisfied {
                    let plugin = pending_plugins.remove(i);
                    let plugin_name = plugin.info().name.clone();
                    self.add_plugin_internal(plugin)?;
                    added_plugins.insert(plugin_name);
                } else {
                    i += 1;
                }
            }
            
            // If we didn't make progress, we have unresolvable dependencies
            if pending_plugins.len() == initial_count {
                let unresolved: Vec<String> = pending_plugins.iter()
                    .map(|p| p.info().name.clone())
                    .collect();
                return Err(anyhow::anyhow!(
                    "Circular or missing dependencies for plugins: {}", 
                    unresolved.join(", ")
                ));
            }
        }
        
        Ok(self)
    }
    
    /// Internal method to add a plugin without dependency checking
    fn add_plugin_internal(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        let info = plugin.info();
        let name = info.name.clone();
        
        // Create plugin handle
        let mut handle = PluginHandle::new(plugin);
        
        // Load the plugin
        handle.load()?;
        
        // Build and enable the plugin
        handle.enable(self)?;
        
        // Store the plugin
        self.plugins.insert(name.clone(), handle);
        self.plugin_load_order.push(name.clone());
        self.enabled_plugins.insert(name);
        
        Ok(())
    }
    
    /// Add a system to the application
    pub fn add_system<S: System + 'static>(&mut self, system: S) -> &mut Self {
        self.scheduler.add_system(system);
        self
    }
    
    /// Insert a resource into the world
    pub fn insert_resource<R: crate::ecs::Resource>(&mut self, resource: R) -> &mut Self {
        self.world.world_mut().insert_resource(resource);
        self
    }
    
    /// Get a reference to the world
    pub fn world(&self) -> &World {
        self.world.world()
    }
    
    /// Get a mutable reference to the world
    pub fn world_mut(&mut self) -> &mut World {
        self.world.world_mut()
    }
    
    /// Get a reference to the Mirai world
    pub fn mirai_world(&self) -> &MiraiWorld {
        &self.world
    }
    
    /// Get a mutable reference to the Mirai world
    pub fn mirai_world_mut(&mut self) -> &mut MiraiWorld {
        &mut self.world
    }
    
    /// Get the Mirai instance if it's still alive
    pub fn instance(&self) -> Option<Arc<Instance>> {
        self.instance.upgrade()
    }
    
    /// Run the application for one frame
    pub fn update(&mut self) -> Result<()> {
        // Clean up dead clients first
        self.world.cleanup_dead_clients()?;
        
        // Run all systems
        self.scheduler.run_systems(self.world.world_mut())
    }
    
    /// Get the list of loaded plugins in load order
    pub fn loaded_plugins(&self) -> Vec<&str> {
        self.plugin_load_order.iter().map(|s| s.as_str()).collect()
    }
    
    /// Get the list of enabled plugins
    pub fn enabled_plugins(&self) -> Vec<&str> {
        self.enabled_plugins.iter().map(|s| s.as_str()).collect()
    }
    
    /// Check if a plugin is loaded
    pub fn has_plugin(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }
    
    /// Check if a plugin is enabled
    pub fn is_plugin_enabled(&self, name: &str) -> bool {
        self.enabled_plugins.contains(name)
    }
    
    /// Get plugin information
    pub fn get_plugin_info(&self, name: &str) -> Option<&PluginInfo> {
        self.plugins.get(name).map(|handle| &handle.info)
    }
    
    /// Disable a plugin
    pub fn disable_plugin(&mut self, name: &str) -> Result<()> {
        let plugin = self.plugins.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", name))?;
        
        plugin.disable()?;
        self.enabled_plugins.remove(name);
        
        tracing::info!("Disabled plugin: {}", name);
        Ok(())
    }
    
    /// Enable a plugin
    pub fn enable_plugin(&mut self, name: &str) -> Result<()> {
        // Check dependencies first
        if let Some(plugin) = self.plugins.get(name) {
            self.check_dependencies(&plugin.info)?;
        }
        
        // Check if plugin exists first
        if !self.plugins.contains_key(name) {
            return Err(anyhow::anyhow!("Plugin '{}' not found", name));
        }
        
        // Get the plugin and enable it
        let plugin = self.plugins.get_mut(name).unwrap();
        plugin.enable_without_app()?;
        self.enabled_plugins.insert(name.to_string());
        
        tracing::info!("Enabled plugin: {}", name);
        Ok(())
    }
    
    /// Unload a plugin
    pub fn unload_plugin(&mut self, name: &str) -> Result<()> {
        // Check if other plugins depend on this one
        self.check_dependents(name)?;
        
        if let Some(mut plugin) = self.plugins.remove(name) {
            plugin.unload().map_err(|e| anyhow::anyhow!("Failed to unload plugin '{}': {}", name, e))?;
            
            self.plugin_load_order.retain(|p| p != name);
            self.enabled_plugins.remove(name);
            tracing::info!("Unloaded plugin: {}", name);
        }
        
        Ok(())
    }
    
    /// Shutdown all plugins
    pub fn shutdown(&mut self) -> Result<()> {
        // Shutdown in reverse load order
        let mut shutdown_order = self.plugin_load_order.clone();
        shutdown_order.reverse();
        
        for name in shutdown_order {
            if let Some(plugin) = self.plugins.get_mut(&name) {
                if let Err(e) = plugin.shutdown() {
                    tracing::error!("Failed to shutdown plugin {}: {}", name, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Check plugin dependencies
    fn check_dependencies(&self, info: &PluginInfo) -> Result<()> {
        for dep in &info.dependencies {
            if !dep.optional {
                // Check if dependency is loaded
                let dep_plugin = self.plugins.get(&dep.name)
                    .ok_or_else(|| anyhow::anyhow!(
                        "Plugin '{}' depends on '{}' which is not loaded", 
                        info.name, dep.name
                    ))?;
                
                // Check version compatibility
                if !dep.is_satisfied_by(&dep_plugin.info.version) {
                    return Err(anyhow::anyhow!(
                        "Plugin '{}' dependency '{}' version {} does not satisfy requirement {}",
                        info.name, dep.name, dep_plugin.info.version, dep.version_req
                    ));
                }
                
                // Check if dependency is enabled
                if !self.enabled_plugins.contains(&dep.name) {
                    return Err(anyhow::anyhow!(
                        "Plugin '{}' depends on '{}' which is not enabled", 
                        info.name, dep.name
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if other plugins depend on this plugin
    fn check_dependents(&self, plugin_name: &str) -> Result<()> {
        for (name, plugin) in &self.plugins {
            if name == plugin_name {
                continue;
            }
            
            if self.enabled_plugins.contains(name) {
                for dep in &plugin.info.dependencies {
                    if !dep.optional && dep.name == plugin_name {
                        return Err(anyhow::anyhow!(
                            "Cannot unload plugin '{}' because plugin '{}' depends on it", 
                            plugin_name, name
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new(Weak::new())
    }
}