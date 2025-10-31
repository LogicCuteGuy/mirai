//! Unified feature management system for mirai

use crate::unified_config::UnifiedFeatureFlags;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Errors related to feature validation and compatibility
#[derive(Debug, Error)]
pub enum FeatureError {
    #[error("Feature '{feature}' requires '{dependency}' but it is not enabled")]
    MissingDependency { feature: String, dependency: String },
    
    #[error("Features '{feature1}' and '{feature2}' are incompatible")]
    IncompatibleFeatures { feature1: String, feature2: String },
    
    #[error("Unknown feature: '{feature}'")]
    UnknownFeature { feature: String },
    
    #[error("Circular dependency detected in feature chain: {chain:?}")]
    CircularDependency { chain: Vec<String> },
    
    #[error("Feature '{feature}' is disabled at compile-time and cannot be enabled at runtime")]
    CompileTimeDisabled { feature: String },
}

/// Represents a game feature with its dependencies and incompatibilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureDefinition {
    pub name: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub incompatible_with: Vec<String>,
    pub performance_impact: PerformanceImpact,
    pub compile_time_enabled: bool,
    pub mirai_compatible: bool,
}

/// Performance impact levels for features
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PerformanceImpact {
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// Unified feature manager that works with both mirai and minecraft-server features
#[derive(Debug, Clone)]
pub struct UnifiedFeatureManager {
    features: HashMap<String, FeatureDefinition>,
    enabled_features: HashSet<String>,
    runtime_flags: UnifiedFeatureFlags,
}

impl UnifiedFeatureManager {
    /// Create a new unified feature manager
    pub fn new() -> Self {
        let mut manager = Self {
            features: HashMap::new(),
            enabled_features: HashSet::new(),
            runtime_flags: UnifiedFeatureFlags::default(),
        };
        
        manager.register_unified_features();
        manager
    }
    
    /// Create from unified config flags
    pub fn from_config(flags: UnifiedFeatureFlags) -> Result<Self, FeatureError> {
        let mut manager = Self::new();
        manager.runtime_flags = flags.clone();
        
        // Enable features based on config flags
        if flags.vanilla_mobs {
            manager.enable_feature("vanilla_mobs")?;
        }
        if flags.redstone {
            manager.enable_feature("redstone")?;
        }
        if flags.world_generation {
            manager.enable_feature("world_generation")?;
        }
        if flags.creative_mode {
            manager.enable_feature("creative_mode")?;
        }
        if flags.command_system {
            manager.enable_feature("command_system")?;
        }
        if flags.performance_monitoring {
            manager.enable_feature("performance_monitoring")?;
        }
        if flags.ecs_system {
            manager.enable_feature("ecs_system")?;
        }
        if flags.plugin_system {
            manager.enable_feature("plugin_system")?;
        }
        
        Ok(manager)
    }
    
    /// Register a new feature definition
    pub fn register_feature(&mut self, feature: FeatureDefinition) {
        self.features.insert(feature.name.clone(), feature);
    }
    
    /// Enable a feature with validation
    pub fn enable_feature(&mut self, feature_name: &str) -> Result<(), FeatureError> {
        if !self.features.contains_key(feature_name) {
            return Err(FeatureError::UnknownFeature {
                feature: feature_name.to_string(),
            });
        }
        
        let feature = self.features.get(feature_name).unwrap();
        
        // Check if feature is available at compile-time
        if !feature.compile_time_enabled {
            return Err(FeatureError::CompileTimeDisabled {
                feature: feature_name.to_string(),
            });
        }
        
        // Check dependencies
        self.validate_dependencies(feature_name, &mut Vec::new())?;
        
        // Check incompatibilities
        self.validate_incompatibilities(feature_name)?;
        
        self.enabled_features.insert(feature_name.to_string());
        
        // Update runtime flags
        self.update_runtime_flags(feature_name, true);
        
        Ok(())
    }
    
    /// Disable a feature
    pub fn disable_feature(&mut self, feature_name: &str) {
        self.enabled_features.remove(feature_name);
        self.update_runtime_flags(feature_name, false);
    }
    
    /// Check if a feature is enabled
    pub fn is_enabled(&self, feature_name: &str) -> bool {
        self.enabled_features.contains(feature_name)
    }
    
    /// Check if a feature is available at compile-time
    pub fn is_compile_time_enabled(&self, feature_name: &str) -> bool {
        self.features.get(feature_name)
            .map(|f| f.compile_time_enabled)
            .unwrap_or(false)
    }
    
    /// Check if a feature is compatible with mirai
    pub fn is_mirai_compatible(&self, feature_name: &str) -> bool {
        self.features.get(feature_name)
            .map(|f| f.mirai_compatible)
            .unwrap_or(false)
    }
    
    /// Get all enabled features
    pub fn enabled_features(&self) -> &HashSet<String> {
        &self.enabled_features
    }
    
    /// Get runtime feature flags
    pub fn runtime_flags(&self) -> &UnifiedFeatureFlags {
        &self.runtime_flags
    }
    
    /// Update runtime flags from current enabled features
    pub fn sync_runtime_flags(&mut self) -> UnifiedFeatureFlags {
        self.runtime_flags = UnifiedFeatureFlags {
            vanilla_mobs: self.is_enabled("vanilla_mobs"),
            redstone: self.is_enabled("redstone"),
            world_generation: self.is_enabled("world_generation"),
            creative_mode: self.is_enabled("creative_mode"),
            command_system: self.is_enabled("command_system"),
            performance_monitoring: self.is_enabled("performance_monitoring"),
            ecs_system: self.is_enabled("ecs_system"),
            plugin_system: self.is_enabled("plugin_system"),
        };
        
        self.runtime_flags.clone()
    }
    
    /// Validate all currently enabled features
    pub fn validate_all(&self) -> Result<(), FeatureError> {
        for feature_name in &self.enabled_features {
            self.validate_dependencies(feature_name, &mut Vec::new())?;
            self.validate_incompatibilities(feature_name)?;
        }
        Ok(())
    }
    
    /// Get feature definition
    pub fn get_feature(&self, name: &str) -> Option<&FeatureDefinition> {
        self.features.get(name)
    }
    
    /// Calculate total performance impact of enabled features
    pub fn calculate_performance_impact(&self) -> PerformanceImpact {
        let mut total_impact = 0u8;
        
        for feature_name in &self.enabled_features {
            if let Some(feature) = self.features.get(feature_name) {
                total_impact += match feature.performance_impact {
                    PerformanceImpact::None => 0,
                    PerformanceImpact::Low => 1,
                    PerformanceImpact::Medium => 2,
                    PerformanceImpact::High => 3,
                    PerformanceImpact::Critical => 5,
                };
            }
        }
        
        match total_impact {
            0 => PerformanceImpact::None,
            1..=2 => PerformanceImpact::Low,
            3..=5 => PerformanceImpact::Medium,
            6..=10 => PerformanceImpact::High,
            _ => PerformanceImpact::Critical,
        }
    }
    
    /// Get features that are mirai-compatible
    pub fn get_mirai_compatible_features(&self) -> Vec<&FeatureDefinition> {
        self.features.values()
            .filter(|f| f.mirai_compatible)
            .collect()
    }
    
    /// Get features that require specific dependencies
    pub fn get_dependent_features(&self, dependency: &str) -> Vec<&FeatureDefinition> {
        self.features.values()
            .filter(|f| f.dependencies.contains(&dependency.to_string()))
            .collect()
    }
    
    fn update_runtime_flags(&mut self, feature_name: &str, enabled: bool) {
        match feature_name {
            "vanilla_mobs" => self.runtime_flags.vanilla_mobs = enabled,
            "redstone" => self.runtime_flags.redstone = enabled,
            "world_generation" => self.runtime_flags.world_generation = enabled,
            "creative_mode" => self.runtime_flags.creative_mode = enabled,
            "command_system" => self.runtime_flags.command_system = enabled,
            "performance_monitoring" => self.runtime_flags.performance_monitoring = enabled,
            "ecs_system" => self.runtime_flags.ecs_system = enabled,
            "plugin_system" => self.runtime_flags.plugin_system = enabled,
            _ => {}
        }
    }
    
    fn validate_dependencies(&self, feature_name: &str, chain: &mut Vec<String>) -> Result<(), FeatureError> {
        if chain.contains(&feature_name.to_string()) {
            return Err(FeatureError::CircularDependency {
                chain: chain.clone(),
            });
        }
        
        chain.push(feature_name.to_string());
        
        if let Some(feature) = self.features.get(feature_name) {
            for dependency in &feature.dependencies {
                if !self.enabled_features.contains(dependency) {
                    return Err(FeatureError::MissingDependency {
                        feature: feature_name.to_string(),
                        dependency: dependency.clone(),
                    });
                }
                
                // Recursively validate dependency chain
                self.validate_dependencies(dependency, chain)?;
            }
        }
        
        chain.pop();
        Ok(())
    }
    
    fn validate_incompatibilities(&self, feature_name: &str) -> Result<(), FeatureError> {
        if let Some(feature) = self.features.get(feature_name) {
            for incompatible in &feature.incompatible_with {
                if self.enabled_features.contains(incompatible) {
                    return Err(FeatureError::IncompatibleFeatures {
                        feature1: feature_name.to_string(),
                        feature2: incompatible.clone(),
                    });
                }
            }
        }
        Ok(())
    }
    
    fn register_unified_features(&mut self) {
        let features = vec![
            FeatureDefinition {
                name: "vanilla_mobs".to_string(),
                description: "Standard Minecraft mob spawning and AI".to_string(),
                dependencies: vec!["world_generation".to_string()],
                incompatible_with: vec![],
                performance_impact: PerformanceImpact::Medium,
                compile_time_enabled: cfg!(feature = "vanilla_mobs"),
                mirai_compatible: true,
            },
            FeatureDefinition {
                name: "redstone".to_string(),
                description: "Redstone circuits and mechanics".to_string(),
                dependencies: vec![],
                incompatible_with: vec![],
                performance_impact: PerformanceImpact::High,
                compile_time_enabled: cfg!(feature = "redstone"),
                mirai_compatible: true,
            },
            FeatureDefinition {
                name: "world_generation".to_string(),
                description: "Terrain and structure generation".to_string(),
                dependencies: vec![],
                incompatible_with: vec![],
                performance_impact: PerformanceImpact::High,
                compile_time_enabled: cfg!(feature = "world_generation"),
                mirai_compatible: true,
            },
            FeatureDefinition {
                name: "creative_mode".to_string(),
                description: "Creative game mode support".to_string(),
                dependencies: vec![],
                incompatible_with: vec![],
                performance_impact: PerformanceImpact::Low,
                compile_time_enabled: cfg!(feature = "creative_mode"),
                mirai_compatible: true,
            },
            FeatureDefinition {
                name: "command_system".to_string(),
                description: "Server commands and admin tools".to_string(),
                dependencies: vec![],
                incompatible_with: vec![],
                performance_impact: PerformanceImpact::Low,
                compile_time_enabled: cfg!(feature = "command_system"),
                mirai_compatible: true,
            },
            FeatureDefinition {
                name: "performance_monitoring".to_string(),
                description: "Real-time performance metrics and profiling".to_string(),
                dependencies: vec![],
                incompatible_with: vec![],
                performance_impact: PerformanceImpact::Low,
                compile_time_enabled: cfg!(feature = "performance_monitoring"),
                mirai_compatible: true,
            },
            FeatureDefinition {
                name: "ecs_system".to_string(),
                description: "Entity Component System architecture".to_string(),
                dependencies: vec![],
                incompatible_with: vec![],
                performance_impact: PerformanceImpact::Medium,
                compile_time_enabled: true, // Always available in unified system
                mirai_compatible: true,
            },
            FeatureDefinition {
                name: "plugin_system".to_string(),
                description: "Plugin architecture and management".to_string(),
                dependencies: vec!["ecs_system".to_string()],
                incompatible_with: vec![],
                performance_impact: PerformanceImpact::Low,
                compile_time_enabled: true, // Always available in unified system
                mirai_compatible: true,
            },
        ];
        
        for feature in features {
            self.register_feature(feature);
        }
    }
}

impl Default for UnifiedFeatureManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe feature manager for runtime use
pub struct RuntimeFeatureManager {
    inner: Arc<RwLock<UnifiedFeatureManager>>,
}

impl RuntimeFeatureManager {
    /// Create a new runtime feature manager
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(UnifiedFeatureManager::new())),
        }
    }
    
    /// Create from unified config
    pub fn from_config(flags: UnifiedFeatureFlags) -> Result<Self, FeatureError> {
        let manager = UnifiedFeatureManager::from_config(flags)?;
        Ok(Self {
            inner: Arc::new(RwLock::new(manager)),
        })
    }
    
    /// Check if a feature is enabled (thread-safe)
    pub fn is_enabled(&self, feature_name: &str) -> bool {
        self.inner.read().unwrap().is_enabled(feature_name)
    }
    
    /// Enable a feature (thread-safe)
    pub fn enable_feature(&self, feature_name: &str) -> Result<(), FeatureError> {
        self.inner.write().unwrap().enable_feature(feature_name)
    }
    
    /// Disable a feature (thread-safe)
    pub fn disable_feature(&self, feature_name: &str) {
        self.inner.write().unwrap().disable_feature(feature_name)
    }
    
    /// Get current runtime flags (thread-safe)
    pub fn get_runtime_flags(&self) -> UnifiedFeatureFlags {
        self.inner.read().unwrap().runtime_flags().clone()
    }
    
    /// Update from new config flags (thread-safe)
    pub fn update_from_config(&self, flags: UnifiedFeatureFlags) -> Result<(), FeatureError> {
        let mut manager = self.inner.write().unwrap();
        
        // Disable all features first
        let current_features: Vec<String> = manager.enabled_features().iter().cloned().collect();
        for feature in current_features {
            manager.disable_feature(&feature);
        }
        
        // Enable features based on new flags
        if flags.vanilla_mobs {
            manager.enable_feature("vanilla_mobs")?;
        }
        if flags.redstone {
            manager.enable_feature("redstone")?;
        }
        if flags.world_generation {
            manager.enable_feature("world_generation")?;
        }
        if flags.creative_mode {
            manager.enable_feature("creative_mode")?;
        }
        if flags.command_system {
            manager.enable_feature("command_system")?;
        }
        if flags.performance_monitoring {
            manager.enable_feature("performance_monitoring")?;
        }
        if flags.ecs_system {
            manager.enable_feature("ecs_system")?;
        }
        if flags.plugin_system {
            manager.enable_feature("plugin_system")?;
        }
        
        Ok(())
    }
    
    /// Validate all features (thread-safe)
    pub fn validate_all(&self) -> Result<(), FeatureError> {
        self.inner.read().unwrap().validate_all()
    }
    
    /// Calculate performance impact (thread-safe)
    pub fn calculate_performance_impact(&self) -> PerformanceImpact {
        self.inner.read().unwrap().calculate_performance_impact()
    }
}

impl Default for RuntimeFeatureManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for RuntimeFeatureManager {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Compile-time feature detection macros for unified system
#[macro_export]
macro_rules! unified_feature_enabled {
    ($feature:literal) => {
        cfg!(feature = $feature)
    };
}

#[macro_export]
macro_rules! require_unified_feature {
    ($feature:literal) => {
        #[cfg(not(feature = $feature))]
        compile_error!(concat!("This code requires the '", $feature, "' feature to be enabled"));
    };
}

#[macro_export]
macro_rules! unified_feature_gate {
    ($feature:literal, $code:block) => {
        #[cfg(feature = $feature)]
        $code
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_unified_feature_manager_creation() {
        let manager = UnifiedFeatureManager::new();
        assert!(manager.get_feature("vanilla_mobs").is_some());
        assert!(manager.get_feature("ecs_system").is_some());
        assert!(manager.get_feature("plugin_system").is_some());
    }
    
    #[test]
    fn test_feature_enabling_with_dependencies() {
        let mut manager = UnifiedFeatureManager::new();
        
        // Enable ECS system first (dependency for plugin system)
        assert!(manager.enable_feature("ecs_system").is_ok());
        assert!(manager.is_enabled("ecs_system"));
        
        // Now enable plugin system
        assert!(manager.enable_feature("plugin_system").is_ok());
        assert!(manager.is_enabled("plugin_system"));
    }
    
    #[test]
    fn test_mirai_compatibility() {
        let manager = UnifiedFeatureManager::new();
        
        // All features should be mirai-compatible
        assert!(manager.is_mirai_compatible("vanilla_mobs"));
        assert!(manager.is_mirai_compatible("ecs_system"));
        assert!(manager.is_mirai_compatible("plugin_system"));
    }
    
    #[test]
    fn test_runtime_feature_manager() {
        let runtime_manager = RuntimeFeatureManager::new();
        
        // Test thread-safe operations
        assert!(runtime_manager.enable_feature("ecs_system").is_ok());
        assert!(runtime_manager.is_enabled("ecs_system"));
        
        runtime_manager.disable_feature("ecs_system");
        assert!(!runtime_manager.is_enabled("ecs_system"));
    }
    
    #[test]
    fn test_config_integration() {
        let flags = UnifiedFeatureFlags {
            vanilla_mobs: false,
            redstone: false, // Disable redstone since it's not compile-time enabled
            world_generation: false, // Disable world_generation since it's not compile-time enabled
            creative_mode: false, // Disable creative_mode since it's not compile-time enabled
            command_system: false, // Disable command_system since it's not compile-time enabled
            performance_monitoring: false,
            ecs_system: true, // This is always compile-time enabled
            plugin_system: true, // This is always compile-time enabled
        };
        
        let manager = UnifiedFeatureManager::from_config(flags).unwrap();
        assert!(!manager.is_enabled("redstone"));
        assert!(manager.is_enabled("ecs_system"));
        assert!(!manager.is_enabled("vanilla_mobs"));
    }
}