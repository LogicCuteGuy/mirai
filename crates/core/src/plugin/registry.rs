//! Plugin registry for managing loaded plugins in Mirai

use super::{Plugin, PluginHandle, PluginState, App};
use anyhow::Result;
use std::collections::{HashMap, HashSet};

/// Plugin registry that manages all loaded plugins
pub struct PluginRegistry {
    /// Map of plugin name to plugin handle
    plugins: HashMap<String, PluginHandle>,
    /// Plugin load order for dependency resolution
    load_order: Vec<String>,
    /// Set of enabled plugins
    enabled_plugins: HashSet<String>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            load_order: Vec::new(),
            enabled_plugins: HashSet::new(),
        }
    }
    
    /// Register a plugin
    pub fn register<P: Plugin + 'static>(&mut self, plugin: P) -> Result<()> {
        let info = plugin.info();
        let name = info.name.clone();
        
        // Check if plugin is already registered
        if self.plugins.contains_key(&name) {
            return Err(anyhow::anyhow!("Plugin '{}' already registered", name));
        }
        
        // Create plugin handle
        let handle = PluginHandle::new(Box::new(plugin));
        
        tracing::debug!("Registered plugin: {} v{}", name, info.version);
        self.plugins.insert(name.clone(), handle);
        
        Ok(())
    }
    
    /// Load a plugin by name
    pub fn load_plugin(&mut self, name: &str) -> Result<()> {
        // Check dependencies first
        self.check_dependencies(name)?;
        
        // Load the plugin
        let plugin = self.plugins.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", name))?;
        
        plugin.load()?;
        
        // Add to load order if not already present
        if !self.load_order.contains(&name.to_string()) {
            self.load_order.push(name.to_string());
        }
        
        Ok(())
    }
    
    /// Enable a plugin by name
    pub fn enable_plugin(&mut self, name: &str, app: &mut App) -> Result<()> {
        // Ensure plugin is loaded
        if let Some(plugin) = self.plugins.get(name) {
            if plugin.state() == PluginState::Unloaded {
                self.load_plugin(name)?;
            }
        }
        
        // Enable the plugin
        let plugin = self.plugins.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", name))?;
        
        plugin.enable(app)?;
        self.enabled_plugins.insert(name.to_string());
        
        Ok(())
    }
    
    /// Disable a plugin by name
    pub fn disable_plugin(&mut self, name: &str) -> Result<()> {
        let plugin = self.plugins.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", name))?;
        
        plugin.disable()?;
        self.enabled_plugins.remove(name);
        
        Ok(())
    }
    
    /// Unload a plugin by name
    pub fn unload_plugin(&mut self, name: &str) -> Result<()> {
        // Check if other plugins depend on this one
        self.check_dependents(name)?;
        
        let plugin = self.plugins.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", name))?;
        
        plugin.unload()?;
        self.enabled_plugins.remove(name);
        self.load_order.retain(|n| n != name);
        
        Ok(())
    }
    
    /// Remove a plugin from the registry
    pub fn unregister_plugin(&mut self, name: &str) -> Result<()> {
        // Unload first if loaded
        if let Some(plugin) = self.plugins.get(name) {
            if plugin.state().is_loaded() {
                self.unload_plugin(name)?;
            }
        }
        
        self.plugins.remove(name);
        tracing::debug!("Unregistered plugin: {}", name);
        
        Ok(())
    }
    
    /// Get a plugin handle by name
    pub fn get_plugin(&self, name: &str) -> Option<&PluginHandle> {
        self.plugins.get(name)
    }
    
    /// Get a mutable plugin handle by name
    pub fn get_plugin_mut(&mut self, name: &str) -> Option<&mut PluginHandle> {
        self.plugins.get_mut(name)
    }
    
    /// Check if a plugin is registered
    pub fn has_plugin(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }
    
    /// Check if a plugin is loaded
    pub fn is_plugin_loaded(&self, name: &str) -> bool {
        self.plugins.get(name)
            .map(|p| p.state().is_loaded())
            .unwrap_or(false)
    }
    
    /// Check if a plugin is enabled
    pub fn is_plugin_enabled(&self, name: &str) -> bool {
        self.enabled_plugins.contains(name)
    }
    
    /// Get all plugin names
    pub fn plugin_names(&self) -> impl Iterator<Item = &String> {
        self.plugins.keys()
    }
    
    /// Get all plugins
    pub fn plugins(&self) -> impl Iterator<Item = &PluginHandle> {
        self.plugins.values()
    }
    
    /// Get all mutable plugins
    pub fn plugins_mut(&mut self) -> impl Iterator<Item = &mut PluginHandle> {
        self.plugins.values_mut()
    }
    
    /// Get enabled plugins
    pub fn enabled_plugins(&self) -> impl Iterator<Item = &PluginHandle> {
        self.enabled_plugins.iter()
            .filter_map(|name| self.plugins.get(name))
    }
    
    /// Get plugin count
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
    
    /// Get enabled plugin count
    pub fn enabled_plugin_count(&self) -> usize {
        self.enabled_plugins.len()
    }
    
    /// Load all registered plugins in dependency order
    pub fn load_all(&mut self) -> Result<()> {
        let load_order = self.resolve_load_order()?;
        
        for name in load_order {
            if let Some(plugin) = self.plugins.get(&name) {
                if plugin.state() == PluginState::Unloaded {
                    self.load_plugin(&name)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Enable all loaded plugins
    pub fn enable_all(&mut self, app: &mut App) -> Result<()> {
        let plugin_names: Vec<String> = self.plugins.keys().cloned().collect();
        
        for name in plugin_names {
            if let Some(plugin) = self.plugins.get(&name) {
                if plugin.state() == PluginState::Loaded || plugin.state() == PluginState::Disabled {
                    self.enable_plugin(&name, app)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Disable all enabled plugins
    pub fn disable_all(&mut self) -> Result<()> {
        let enabled_plugins: Vec<String> = self.enabled_plugins.iter().cloned().collect();
        
        for name in enabled_plugins {
            self.disable_plugin(&name)?;
        }
        
        Ok(())
    }
    
    /// Unload all plugins
    pub fn unload_all(&mut self) -> Result<()> {
        // Disable all first
        self.disable_all()?;
        
        // Unload in reverse order
        let mut load_order = self.load_order.clone();
        load_order.reverse();
        
        for name in load_order {
            if let Some(plugin) = self.plugins.get(&name) {
                if plugin.state().is_loaded() {
                    self.unload_plugin(&name)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Shutdown all plugins
    pub fn shutdown_all(&mut self) -> Result<()> {
        for plugin in self.plugins.values_mut() {
            if let Err(e) = plugin.shutdown() {
                tracing::error!("Failed to shutdown plugin {}: {}", plugin.name(), e);
            }
        }
        
        Ok(())
    }
    
    /// Check plugin dependencies
    fn check_dependencies(&self, plugin_name: &str) -> Result<()> {
        let plugin = self.plugins.get(plugin_name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", plugin_name))?;
        
        for dep in &plugin.info.dependencies {
            if !dep.optional {
                // Check if dependency is registered
                let dep_plugin = self.plugins.get(&dep.name)
                    .ok_or_else(|| anyhow::anyhow!(
                        "Plugin '{}' depends on '{}' which is not registered", 
                        plugin_name, dep.name
                    ))?;
                
                // Check version compatibility
                if !dep.is_satisfied_by(&dep_plugin.info.version) {
                    return Err(anyhow::anyhow!(
                        "Plugin '{}' dependency '{}' version {} does not satisfy requirement {}",
                        plugin_name, dep.name, dep_plugin.info.version, dep.version_req
                    ));
                }
                
                // Check if dependency is loaded
                if !dep_plugin.state().is_loaded() {
                    return Err(anyhow::anyhow!(
                        "Plugin '{}' depends on '{}' which is not loaded", 
                        plugin_name, dep.name
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
            
            if plugin.state().is_loaded() {
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
    
    /// Resolve plugin load order based on dependencies
    fn resolve_load_order(&self) -> Result<Vec<String>> {
        let mut order = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut visiting: HashSet<String> = HashSet::new();
        
        for name in self.plugins.keys() {
            if !visited.contains(name) {
                self.visit_plugin(name, &mut order, &mut visited, &mut visiting)?;
            }
        }
        
        Ok(order)
    }
    
    /// Visit a plugin for dependency resolution (topological sort)
    fn visit_plugin(
        &self,
        name: &String,
        order: &mut Vec<String>,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
    ) -> Result<()> {
        if visiting.contains(name) {
            return Err(anyhow::anyhow!("Circular dependency detected involving plugin '{}'", name));
        }
        
        if visited.contains(name) {
            return Ok(());
        }
        
        visiting.insert(name.clone());
        
        if let Some(plugin) = self.plugins.get(name) {
            for dep in &plugin.info.dependencies {
                if !dep.optional && self.plugins.contains_key(&dep.name) {
                    self.visit_plugin(&dep.name, order, visited, visiting)?;
                }
            }
        }
        
        visiting.remove(name);
        visited.insert(name.clone());
        order.push(name.clone());
        
        Ok(())
    }
    
    /// Get plugin statistics
    pub fn get_stats(&self) -> PluginRegistryStats {
        let mut stats = PluginRegistryStats::default();
        
        stats.total_plugins = self.plugins.len();
        stats.enabled_plugins = self.enabled_plugins.len();
        
        for plugin in self.plugins.values() {
            match plugin.state() {
                PluginState::Unloaded => stats.unloaded_plugins += 1,
                PluginState::Loaded => stats.loaded_plugins += 1,
                PluginState::Enabled => {}, // Already counted in enabled_plugins
                PluginState::Disabled => stats.disabled_plugins += 1,
                PluginState::Failed => stats.failed_plugins += 1,
            }
        }
        
        stats
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin registry statistics
#[derive(Debug, Clone, Default)]
pub struct PluginRegistryStats {
    pub total_plugins: usize,
    pub enabled_plugins: usize,
    pub loaded_plugins: usize,
    pub disabled_plugins: usize,
    pub unloaded_plugins: usize,
    pub failed_plugins: usize,
}