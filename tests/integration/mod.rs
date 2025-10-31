//! Integration tests for the merged mirai-minecraft-server system
//! 
//! This module contains comprehensive integration tests that validate
//! the merged functionality works correctly together.

pub mod ecs_plugin_integration;
pub mod protocol_network_integration;
pub mod codegen_build_integration;
pub mod performance_integration;
pub mod compatibility_validation;
pub mod system_integration_validation;

use mirai_core::{plugin::App, ecs::World, instance::Instance};
use std::time::Duration;
use std::sync::{Arc, Weak};
use tokio::time::timeout;

/// Helper function to create a test mirai instance with merged functionality
pub async fn create_test_mirai_instance() -> Result<Arc<Instance>, Box<dyn std::error::Error>> {
    let instance = Instance::builder()
        .level_path("test_level")
        .build()
        .await?;
    
    Ok(instance)
}

/// Helper function to create a test app with plugins
pub fn create_test_app() -> App {
    let mut app = App::new(Weak::new());
    
    // Add test plugins
    if let Err(e) = app.add_plugin(TestEcsPlugin) {
        eprintln!("Failed to add TestEcsPlugin: {}", e);
    }
    
    if let Err(e) = app.add_plugin(TestProtocolPlugin) {
        eprintln!("Failed to add TestProtocolPlugin: {}", e);
    }
    
    app
}

/// Test plugin for ECS functionality
pub struct TestEcsPlugin;

impl mirai_core::plugin::Plugin for TestEcsPlugin {
    fn info(&self) -> mirai_core::plugin::PluginInfo {
        mirai_core::plugin::PluginInfo::new("test_ecs_plugin", semver::Version::new(1, 0, 0))
            .with_description("Test ECS plugin for integration tests")
            .with_author("Mirai Test Suite")
    }
    
    fn build(&self, app: &mut App) -> anyhow::Result<()> {
        // Add test systems and components
        app.world_mut().insert_resource(TestResource { value: 42 });
        Ok(())
    }
}

/// Test plugin for protocol functionality
pub struct TestProtocolPlugin;

impl mirai_core::plugin::Plugin for TestProtocolPlugin {
    fn info(&self) -> mirai_core::plugin::PluginInfo {
        mirai_core::plugin::PluginInfo::new("test_protocol_plugin", semver::Version::new(1, 0, 0))
            .with_description("Test protocol plugin for integration tests")
            .with_author("Mirai Test Suite")
    }
    
    fn build(&self, app: &mut App) -> anyhow::Result<()> {
        // Add test protocol handlers
        Ok(())
    }
}

/// Test component for ECS testing
#[derive(Debug, Clone)]
pub struct TestComponent {
    pub value: i32,
}

impl mirai_core::ecs::Component for TestComponent {}

/// Test resource for ECS testing
#[derive(Debug, Clone)]
pub struct TestResource {
    pub value: i32,
}

impl mirai_core::ecs::Resource for TestResource {}

/// Helper function to wait for async operations with timeout
pub async fn wait_with_timeout<F, T>(future: F, duration: Duration) -> Result<T, Box<dyn std::error::Error>>
where
    F: std::future::Future<Output = T>,
{
    timeout(duration, future).await
        .map_err(|_| "Operation timed out".into())
}