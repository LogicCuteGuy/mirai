//! Core plugin trait and types for Mirai

use crate::ecs::{World, System};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Core trait that all Mirai plugins must implement
pub trait Plugin: Send + Sync {
    /// Get plugin information and metadata
    fn info(&self) -> PluginInfo;
    
    /// Build the plugin (add systems, resources, etc.)
    /// 
    /// This is called when the plugin is being registered with the app.
    /// Use this method to add systems, insert resources, and configure
    /// the plugin's functionality.
    fn build(&self, app: &mut super::App) -> Result<()>;
    
    /// Called when the plugin is loaded
    /// 
    /// This is called after the plugin is registered but before it's built.
    /// Use this for initialization that doesn't require the app context.
    fn on_load(&self) -> Result<()> {
        Ok(())
    }
    
    /// Called when the plugin is enabled
    /// 
    /// This is called after the plugin is built and ready to be used.
    fn on_enable(&self) -> Result<()> {
        Ok(())
    }
    
    /// Called when the plugin is disabled
    /// 
    /// This is called when the plugin is being temporarily disabled.
    fn on_disable(&self) -> Result<()> {
        Ok(())
    }
    
    /// Called when the plugin is unloaded
    /// 
    /// This is called when the plugin is being permanently removed.
    /// Use this for cleanup that doesn't require the app context.
    fn on_unload(&self) -> Result<()> {
        Ok(())
    }
    
    /// Called when the server is shutting down
    /// 
    /// This is called during server shutdown for final cleanup.
    fn on_shutdown(&self) -> Result<()> {
        Ok(())
    }
    
    /// Get plugin configuration schema (optional)
    /// 
    /// Return a JSON schema that describes the plugin's configuration format.
    fn config_schema(&self) -> Option<serde_json::Value> {
        None
    }
    
    /// Validate plugin configuration (optional)
    /// 
    /// Validate the provided configuration against the plugin's requirements.
    fn validate_config(&self, _config: &serde_json::Value) -> Result<()> {
        Ok(())
    }
    
    /// Apply plugin configuration (optional)
    /// 
    /// Apply the validated configuration to the plugin.
    fn apply_config(&self, _config: &serde_json::Value) -> Result<()> {
        Ok(())
    }
}

/// Plugin information and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin name (must be unique)
    pub name: String,
    /// Plugin version
    pub version: semver::Version,
    /// Plugin description
    pub description: String,
    /// Plugin author(s)
    pub author: String,
    /// Required dependencies
    pub dependencies: Vec<PluginDependency>,
    /// Optional dependencies
    pub optional_dependencies: Vec<PluginDependency>,
}

impl PluginInfo {
    /// Create new plugin info
    pub fn new(name: impl Into<String>, version: semver::Version) -> Self {
        Self {
            name: name.into(),
            version,
            description: String::new(),
            author: String::new(),
            dependencies: Vec::new(),
            optional_dependencies: Vec::new(),
        }
    }
    
    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
    
    /// Set author
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }
    
    /// Add a dependency
    pub fn with_dependency(mut self, dependency: PluginDependency) -> Self {
        self.dependencies.push(dependency);
        self
    }
    
    /// Add an optional dependency
    pub fn with_optional_dependency(mut self, dependency: PluginDependency) -> Self {
        self.optional_dependencies.push(dependency);
        self
    }
    
    /// Get all dependencies (required + optional)
    pub fn all_dependencies(&self) -> impl Iterator<Item = &PluginDependency> {
        self.dependencies.iter().chain(self.optional_dependencies.iter())
    }
}

/// Plugin dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Name of the required plugin
    pub name: String,
    /// Version requirement
    pub version_req: semver::VersionReq,
    /// Whether this dependency is optional
    pub optional: bool,
}

impl PluginDependency {
    /// Create a new dependency
    pub fn new(name: impl Into<String>, version_req: semver::VersionReq) -> Self {
        Self {
            name: name.into(),
            version_req,
            optional: false,
        }
    }
    
    /// Create an optional dependency
    pub fn optional(name: impl Into<String>, version_req: semver::VersionReq) -> Self {
        Self {
            name: name.into(),
            version_req,
            optional: true,
        }
    }
    
    /// Check if a version satisfies this dependency
    pub fn is_satisfied_by(&self, version: &semver::Version) -> bool {
        self.version_req.matches(version)
    }
}

/// Plugin state in the lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginState {
    /// Plugin is not loaded
    Unloaded,
    /// Plugin is loaded but not enabled
    Loaded,
    /// Plugin is enabled and active
    Enabled,
    /// Plugin is disabled
    Disabled,
    /// Plugin failed to load or run
    Failed,
}

impl PluginState {
    /// Check if the plugin is active (enabled)
    pub fn is_active(self) -> bool {
        matches!(self, Self::Enabled)
    }
    
    /// Check if the plugin is loaded (any state except unloaded)
    pub fn is_loaded(self) -> bool {
        !matches!(self, Self::Unloaded)
    }
    
    /// Check if the plugin has failed
    pub fn is_failed(self) -> bool {
        matches!(self, Self::Failed)
    }
    
    /// Get the next valid states from the current state
    pub fn valid_transitions(self) -> &'static [PluginState] {
        match self {
            Self::Unloaded => &[Self::Loaded, Self::Failed],
            Self::Loaded => &[Self::Enabled, Self::Unloaded, Self::Failed],
            Self::Enabled => &[Self::Disabled, Self::Unloaded, Self::Failed],
            Self::Disabled => &[Self::Enabled, Self::Unloaded, Self::Failed],
            Self::Failed => &[Self::Unloaded],
        }
    }
    
    /// Check if transition to another state is valid
    pub fn can_transition_to(self, target: PluginState) -> bool {
        self.valid_transitions().contains(&target)
    }
}

impl std::fmt::Display for PluginState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unloaded => write!(f, "Unloaded"),
            Self::Loaded => write!(f, "Loaded"),
            Self::Enabled => write!(f, "Enabled"),
            Self::Disabled => write!(f, "Disabled"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// A simple plugin implementation for testing
pub struct DefaultPlugin {
    info: PluginInfo,
}

impl DefaultPlugin {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            info: PluginInfo::new(name, semver::Version::new(1, 0, 0))
                .with_description("Default plugin for testing")
                .with_author("Mirai Core"),
        }
    }
}

impl Plugin for DefaultPlugin {
    fn info(&self) -> PluginInfo {
        self.info.clone()
    }
    
    fn build(&self, _app: &mut super::App) -> Result<()> {
        // Default plugin doesn't add anything
        Ok(())
    }
}