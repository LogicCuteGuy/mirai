//! Integration tests for RakNet connection management
//! 
//! These tests verify that the RakNet integration works correctly with
//! the enhanced protocol handling system.

use mirai_proto::{
    RakNetConnectionManager, RakNetManagerConfig, RakNetConfig,
    BedrockProtocolError, RawBedrockPacket, PacketDirection
};
use bytes::Bytes;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::time::{timeout, Duration};
use uuid::Uuid;

#[tokio::test]
async fn test_raknet_manager_lifecycle() {
    // Create manager with default config
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let config = RakNetManagerConfig::default();
    
    let manager = RakNetConnectionManager::new(addr, config).await
        .expect("Failed to create RakNet manager");
    
    // Verify initial state
    assert!(!manager.is_running().await);
    assert!(manager.local_addr().is_ok());
    
    // Start the manager
    manager.start().await.expect("Failed to start manager");
    assert!(manager.is_running().await);
    
    // Verify stats are initialized
    let stats = manager.get_raknet_stats().await;
    assert_eq!(stats.active_connections, 0);
    assert_eq!(stats.total_connections, 0);
    assert!(stats.start_time.is_some());
    
    // Stop the manager
    manager.stop().await.expect("Failed to stop manager");
    assert!(!manager.is_running().await);
}

#[tokio::test]
async fn test_raknet_config_management() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let config = RakNetManagerConfig::default();
    
    let manager = RakNetConnectionManager::new(addr, config).await
        .expect("Failed to create RakNet manager");
    
    // Get initial config
    let initial_config = manager.get_raknet_config().await;
    assert_eq!(initial_config.mtu, 1400);
    assert_eq!(initial_config.max_connections, 100);
    
    // Update config
    let mut new_config = initial_config.clone();
    new_config.mtu = 1200;
    new_config.max_connections = 50;
    new_config.connection_timeout = 60000;
    
    manager.update_raknet_config(new_config.clone()).await;
    
    // Verify config was updated
    let updated_config = manager.get_raknet_config().await;
    assert_eq!(updated_config.mtu, 1200);
    assert_eq!(updated_config.max_connections, 50);
    assert_eq!(updated_config.connection_timeout, 60000);
}

#[tokio::test]
async fn test_packet_broadcasting() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let config = RakNetManagerConfig::default();
    
    let manager = RakNetConnectionManager::new(addr, config).await
        .expect("Failed to create RakNet manager");
    
    manager.start().await.expect("Failed to start manager");
    
    // Create a test packet
    let test_packet = RawBedrockPacket {
        id: 0x01,
        data: Bytes::from_static(b"test packet data"),
        direction: PacketDirection::Clientbound,
    };
    
    // Broadcast packet (should succeed even with no connections)
    let sent_count = manager.broadcast_packet(test_packet).await
        .expect("Failed to broadcast packet");
    
    // Should be 0 since no connections are active
    assert_eq!(sent_count, 0);
    
    // Verify stats were updated
    let stats = manager.get_raknet_stats().await;
    assert_eq!(stats.total_packets_sent, 0); // No actual sends since no connections
    
    manager.stop().await.expect("Failed to stop manager");
}

#[tokio::test]
async fn test_connection_info_retrieval() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let config = RakNetManagerConfig::default();
    
    let manager = RakNetConnectionManager::new(addr, config).await
        .expect("Failed to create RakNet manager");
    
    manager.start().await.expect("Failed to start manager");
    
    // Try to get info for non-existent connection
    let fake_id = Uuid::new_v4();
    let info = manager.get_connection_info(fake_id).await;
    assert!(info.is_none());
    
    // Get all connections (should be empty)
    let all_connections = manager.get_all_connections().await;
    assert!(all_connections.is_empty());
    
    manager.stop().await.expect("Failed to stop manager");
}

#[tokio::test]
async fn test_enhanced_stats_integration() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let config = RakNetManagerConfig::default();
    
    let manager = RakNetConnectionManager::new(addr, config).await
        .expect("Failed to create RakNet manager");
    
    // Get enhanced stats
    let enhanced_stats = manager.get_enhanced_stats().await;
    assert_eq!(enhanced_stats.bedrock_connections, 0);
    assert_eq!(enhanced_stats.total_connections, 0);
    assert_eq!(enhanced_stats.active_connections, 0);
    
    // Get RakNet-specific stats
    let raknet_stats = manager.get_raknet_stats().await;
    assert_eq!(raknet_stats.active_connections, 0);
    assert_eq!(raknet_stats.total_connections, 0);
    assert_eq!(raknet_stats.total_packets_sent, 0);
    assert_eq!(raknet_stats.total_packets_received, 0);
}

#[tokio::test]
async fn test_manager_error_handling() {
    // Test binding to invalid address
    let invalid_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 80);
    let config = RakNetManagerConfig::default();
    
    let result = RakNetConnectionManager::new(invalid_addr, config).await;
    assert!(result.is_err());
    
    // Test double start
    let valid_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let config = RakNetManagerConfig::default();
    
    let manager = RakNetConnectionManager::new(valid_addr, config).await
        .expect("Failed to create manager");
    
    manager.start().await.expect("Failed to start manager");
    
    // Try to start again - should fail
    let result = manager.start().await;
    assert!(result.is_err());
    
    manager.stop().await.expect("Failed to stop manager");
}

#[tokio::test]
async fn test_packet_send_to_nonexistent_connection() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let config = RakNetManagerConfig::default();
    
    let manager = RakNetConnectionManager::new(addr, config).await
        .expect("Failed to create RakNet manager");
    
    manager.start().await.expect("Failed to start manager");
    
    // Try to send packet to non-existent connection
    let fake_id = Uuid::new_v4();
    let test_packet = RawBedrockPacket {
        id: 0x01,
        data: Bytes::from_static(b"test"),
        direction: PacketDirection::Clientbound,
    };
    
    let result = manager.send_packet(fake_id, test_packet).await;
    assert!(result.is_err());
    
    manager.stop().await.expect("Failed to stop manager");
}

#[test]
fn test_raknet_manager_config_validation() {
    let config = RakNetManagerConfig::default();
    
    // Verify default values are reasonable
    assert!(config.max_connections > 0);
    assert!(config.cleanup_interval > 0);
    assert!(config.max_packet_size > 0);
    assert!(config.max_packet_size <= 65536); // UDP limit
    
    // Verify RakNet config is valid
    assert!(config.raknet_config.mtu > 0);
    assert!(config.raknet_config.connection_timeout > 0);
    assert!(config.raknet_config.max_connections > 0);
}