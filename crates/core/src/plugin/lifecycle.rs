//! Plugin lifecycle management for Mirai

use super::{Plugin, PluginInfo, PluginState, App};
use anyhow::Result;
use std::collections::HashMap;
use std::time::Instant;

/// Plugin handle for managing plugin lifecycle
pub struct PluginHandle {
    /// Plugin information
    pub info: PluginInfo,
    /// Current plugin state
    pub state: PluginState,
    /// Plugin instance (boxed trait object)
    plugin: Box<dyn Plugin>,
    /// Plugin configuration
    config: Option<serde_json::Value>,
    /// Plugin metadata
    metadata: HashMap<String, serde_json::Value>,
    /// Last state change time
    last_state_change: Instant,
    /// Plugin statistics
    stats: PluginStats,
}

/// Plugin statistics
#[derive(Debug, Clone, Default)]
pub struct PluginStats {
    /// Number of times the plugin has been loaded
    pub load_count: u64,
    /// Number of times the plugin has been enabled
    pub enable_count: u64,
    /// Number of times the plugin has failed
    pub failure_count: u64,
    /// Time when the plugin was last enabled
    pub last_enabled_time: Option<Instant>,
}

impl PluginHandle {
    /// Create a new plugin handle
    pub fn new(plugin: Box<dyn Plugin>) -> Self {
        let info = plugin.info();
        Self {
            info,
            state: PluginState::Unloaded,
            plugin,
            config: None,
            metadata: HashMap::new(),
            last_state_change: Instant::now(),
            stats: PluginStats::default(),
        }
    }
    
    /// Get plugin name
    pub fn name(&self) -> &str {
        &self.info.name
    }
    
    /// Get plugin version
    pub fn version(&self) -> &semver::Version {
        &self.info.version
    }
    
    /// Get plugin state
    pub fn state(&self) -> PluginState {
        self.state
    }
    
    /// Get plugin statistics
    pub fn stats(&self) -> &PluginStats {
        &self.stats
    }
    
    /// Load the plugin
    pub fn load(&mut self) -> Result<()> {
        if self.state != PluginState::Unloaded {
            return Err(anyhow::anyhow!(
                "Plugin '{}' is in state {:?}, expected Unloaded", 
                self.info.name, self.state
            ));
        }
        
        match self.plugin.on_load() {
            Ok(()) => {
                self.set_state(PluginState::Loaded)?;
                self.stats.load_count += 1;
                tracing::info!("Loaded plugin: {} v{}", self.info.name, self.info.version);
                Ok(())
            }
            Err(e) => {
                self.set_state(PluginState::Failed)?;
                self.stats.failure_count += 1;
                Err(anyhow::anyhow!("Failed to load plugin '{}': {}", self.info.name, e))
            }
        }
    }
    
    /// Enable the plugin
    pub fn enable(&mut self, app: &mut App) -> Result<()> {
        if self.state != PluginState::Loaded && self.state != PluginState::Disabled {
            return Err(anyhow::anyhow!(
                "Plugin '{}' is in state {:?}, expected Loaded or Disabled", 
                self.info.name, self.state
            ));
        }
        
        // Build the plugin first
        if let Err(e) = self.plugin.build(app) {
            self.set_state(PluginState::Failed)?;
            self.stats.failure_count += 1;
            return Err(anyhow::anyhow!("Failed to build plugin '{}': {}", self.info.name, e));
        }
        
        // Then enable it
        match self.plugin.on_enable() {
            Ok(()) => {
                self.set_state(PluginState::Enabled)?;
                self.stats.enable_count += 1;
                self.stats.last_enabled_time = Some(Instant::now());
                tracing::info!("Enabled plugin: {} v{}", self.info.name, self.info.version);
                Ok(())
            }
            Err(e) => {
                self.set_state(PluginState::Failed)?;
                self.stats.failure_count += 1;
                Err(anyhow::anyhow!("Failed to enable plugin '{}': {}", self.info.name, e))
            }
        }
    }
    
    /// Enable the plugin without building (for internal use)
    pub fn enable_without_app(&mut self) -> Result<()> {
        if self.state != PluginState::Loaded && self.state != PluginState::Disabled {
            return Err(anyhow::anyhow!(
                "Plugin '{}' is in state {:?}, expected Loaded or Disabled", 
                self.info.name, self.state
            ));
        }
        
        // Just enable the plugin without building
        match self.plugin.on_enable() {
            Ok(()) => {
                self.set_state(PluginState::Enabled)?;
                self.stats.enable_count += 1;
                self.stats.last_enabled_time = Some(Instant::now());
                tracing::info!("Enabled plugin: {} v{}", self.info.name, self.info.version);
                Ok(())
            }
            Err(e) => {
                self.set_state(PluginState::Failed)?;
                self.stats.failure_count += 1;
                Err(anyhow::anyhow!("Failed to enable plugin '{}': {}", self.info.name, e))
            }
        }
    }
    
    /// Disable the plugin
    pub fn disable(&mut self) -> Result<()> {
        if self.state != PluginState::Enabled {
            return Err(anyhow::anyhow!(
                "Plugin '{}' is in state {:?}, expected Enabled", 
                self.info.name, self.state
            ));
        }
        
        match self.plugin.on_disable() {
            Ok(()) => {
                self.set_state(PluginState::Disabled)?;
                tracing::info!("Disabled plugin: {} v{}", self.info.name, self.info.version);
                Ok(())
            }
            Err(e) => {
                self.set_state(PluginState::Failed)?;
                self.stats.failure_count += 1;
                Err(anyhow::anyhow!("Failed to disable plugin '{}': {}", self.info.name, e))
            }
        }
    }
    
    /// Unload the plugin
    pub fn unload(&mut self) -> Result<()> {
        if self.state == PluginState::Enabled {
            self.disable()?;
        }
        
        if self.state != PluginState::Loaded && self.state != PluginState::Disabled && self.state != PluginState::Failed {
            return Err(anyhow::anyhow!(
                "Plugin '{}' is in state {:?}, expected Loaded, Disabled, or Failed", 
                self.info.name, self.state
            ));
        }
        
        match self.plugin.on_unload() {
            Ok(()) => {
                self.set_state(PluginState::Unloaded)?;
                tracing::info!("Unloaded plugin: {} v{}", self.info.name, self.info.version);
                Ok(())
            }
            Err(e) => {
                self.set_state(PluginState::Failed)?;
                self.stats.failure_count += 1;
                Err(anyhow::anyhow!("Failed to unload plugin '{}': {}", self.info.name, e))
            }
        }
    }
    
    /// Shutdown the plugin
    pub fn shutdown(&mut self) -> Result<()> {
        match self.plugin.on_shutdown() {
            Ok(()) => {
                tracing::debug!("Plugin {} shutdown completed", self.info.name);
                Ok(())
            }
            Err(e) => {
                tracing::error!("Plugin {} shutdown failed: {}", self.info.name, e);
                Err(anyhow::anyhow!("Plugin '{}' shutdown failed: {}", self.info.name, e))
            }
        }
    }
    
    /// Set plugin state with validation
    fn set_state(&mut self, new_state: PluginState) -> Result<()> {
        if !self.state.can_transition_to(new_state) {
            return Err(anyhow::anyhow!(
                "Plugin '{}' cannot transition from {:?} to {:?}", 
                self.info.name, self.state, new_state
            ));
        }
        
        tracing::debug!("Plugin {} state: {:?} -> {:?}", self.info.name, self.state, new_state);
        self.state = new_state;
        self.last_state_change = Instant::now();
        Ok(())
    }
    
    /// Set plugin configuration
    pub fn set_config(&mut self, config: serde_json::Value) -> Result<()> {
        self.plugin.validate_config(&config)?;
        self.plugin.apply_config(&config)?;
        self.config = Some(config);
        Ok(())
    }
    
    /// Get plugin configuration
    pub fn config(&self) -> Option<&serde_json::Value> {
        self.config.as_ref()
    }
    
    /// Set metadata
    pub fn set_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.insert(key.into(), value);
    }
    
    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }
    
    /// Get all metadata
    pub fn metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.metadata
    }
}

/// Configuration for plugin lifecycle management
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    /// Whether to isolate plugin errors (prevent cascade failures)
    pub error_isolation: bool,
    /// Whether to automatically restart failed plugins
    pub auto_restart: bool,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            error_isolation: true,
            auto_restart: false,
        }
    }
}