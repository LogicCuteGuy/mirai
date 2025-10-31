//! Mirai Plugins
//! 
//! This crate contains plugin implementations for the Mirai Minecraft server,
//! including vanilla mobs, redstone mechanics, and other gameplay features.

pub mod vanilla_mobs;
pub mod redstone;

// Re-export core plugin types for convenience
pub use mirai_core::plugin::{Plugin, PluginInfo, PluginDependency, PluginState};
pub use mirai_core::ecs::{World, Component, Resource, System, EntityId};

// Re-export plugin implementations
pub use vanilla_mobs::VanillaMobsPlugin;
pub use redstone::RedstonePlugin;