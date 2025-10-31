//! Plugin system for Mirai server
//! 
//! Provides a Bevy-like plugin architecture integrated with Mirai's existing systems.
//! This module adapts the minecraft-server-plugins system to work seamlessly with
//! Mirai's Instance and BedrockClient architecture.

pub mod plugin;
pub mod app;
pub mod registry;
pub mod lifecycle;
pub mod bridge;

// Re-export core types
pub use plugin::{Plugin, PluginInfo, PluginDependency, PluginState};
pub use app::App;
pub use registry::PluginRegistry;
pub use lifecycle::{PluginHandle};
pub use bridge::{MiraiPlugin, MiraiPluginContext, PluginCategory};

/// Plugin system version for Mirai
pub const MIRAI_PLUGIN_SYSTEM_VERSION: &str = "1.0.0";

/// Initialize the Mirai plugin system
pub fn init_mirai_plugin_system() -> anyhow::Result<PluginRegistry> {
    tracing::info!("Initializing Mirai plugin system v{}", MIRAI_PLUGIN_SYSTEM_VERSION);
    
    let registry = PluginRegistry::new();
    
    tracing::info!("Mirai plugin system initialized");
    Ok(registry)
}