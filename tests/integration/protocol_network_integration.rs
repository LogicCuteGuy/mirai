//! Integration tests for protocol consolidation and network functionality
//! 
//! Tests that validate the merged protocol handling from minecraft-server-protocol
//! works correctly with mirai's existing RakNet functionality.

use super::*;
use mirai_proto::{
    UnifiedProtocolHandler, RakNetConnectionManager, RakNetManagerConfig,
    UnifiedAuthService, UnifiedAuthConfig, ProtocolType,
    BedrockProtocolError, RawBedrockPacket, PacketDirection,
    EnhancedConnection, ConnectionState, UnifiedConnectionState
};
use bytes::Bytes;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn test_unified_protocol_handler_integration() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let raknet_config = RakNetManagerConfig::default();
    let auth_config = UnifiedAuthConfig::default();
    
    let protocol_handler = UnifiedProtocolHandler::new(addr, raknet_config, auth_config).await
        .expect("Failed to create unified protocol handler");
    
    // Start the handler
    protocol_handler.start().await
        .expect("Failed to start protocol handler");
    
    // Verify both RakNet and protocol systems are running
    assert!(protocol_handler.is_running().await);
    assert!(protocol_handler.raknet_manager().is_running().await);
    
    // Test packet registry
    let registry = protocol_handler.packet_registry();
    assert!(registry.is_packet_registered(0x01)); // Login packet
    assert!(registry.is_packet_registered(0x02)); // Play status packet
    
    protocol_handler.stop().await
        .expect("Failed to stop protocol handler");
}

#[tokio::test]
async fn test_raknet_protocol_bridge() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let config = RakNetManagerConfig::default();
    
    let manager = RakNetConnectionManager::new(addr, config).await
        .expect("Failed to create RakNet manager");
    
    manager.start().await.expect("Failed to start manager");
    
    // Test protocol packet handling through RakNet
    let test_packet = RawBedrockPacket {
        id: 0x01,
        data: Bytes::from_static(b"test protocol data"),
        direction: PacketDirection::ServerToClient,
    };
    
    // Test packet processing
    let processed = manager.process_protocol_packet(test_packet).await;
    assert!(processed.is_ok());
    
    // Test enhanced connection management
    let connection_id = Uuid::new_v4();
    let enhanced_conn = EnhancedConnection::new(
        connection_id,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 12345),
        ProtocolType::Bedrock
    );
    
    manager.register_enhanced_connection(enhanced_conn).await
        .expect("Failed to register enhanced connection");
    
    let conn_info = manager.get_enhanced_connection_info(connection_id).await;
    assert!(conn_info.is_some());
    
    manager.stop().await.expect("Failed to stop manager");
}

#[tokio::test]
async fn test_unified_authentication_integration() {
    let config = UnifiedAuthConfig::default();
    let auth_service = UnifiedAuthService::new(config);
    
    // Test Java authentication
    let mut java_auth_data = mirai_proto::UnifiedAuthData::default();
    java_auth_data.username = Some("JavaPlayer".to_string());
    
    let java_result = auth_service.authenticate_player(ProtocolType::Java, &java_auth_data);
    assert!(java_result.is_ok());
    
    let java_profile = java_result.unwrap();
    assert_eq!(java_profile.username, "JavaPlayer");
    assert_eq!(java_profile.protocol_type, ProtocolType::Java);
    
    // Test Bedrock authentication
    let mut bedrock_auth_data = mirai_proto::UnifiedAuthData::default();
    bedrock_auth_data.username = Some("BedrockPlayer".to_string());
    bedrock_auth_data.xuid = Some("1234567890".to_string());
    
    let bedrock_result = auth_service.authenticate_player(ProtocolType::Bedrock, &bedrock_auth_data);
    assert!(bedrock_result.is_ok());
    
    let bedrock_profile = bedrock_result.unwrap();
    assert_eq!(bedrock_profile.username, "BedrockPlayer");
    assert_eq!(bedrock_profile.protocol_type, ProtocolType::Bedrock);
    assert_eq!(bedrock_profile.xuid, Some(1234567890));
    
    // Verify authentication statistics
    let stats = auth_service.get_auth_stats();
    assert_eq!(stats.total_authentications, 2);
    assert_eq!(stats.java_authentications, 1);
    assert_eq!(stats.bedrock_authentications, 1);
}

#[tokio::test]
async fn test_connection_state_management() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let config = RakNetManagerConfig::default();
    
    let manager = RakNetConnectionManager::new(addr, config).await
        .expect("Failed to create manager");
    
    manager.start().await.expect("Failed to start manager");
    
    let connection_id = Uuid::new_v4();
    let client_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 12345);
    
    // Create enhanced connection
    let mut connection = EnhancedConnection::new(connection_id, client_addr, ProtocolType::Bedrock);
    
    // Test state transitions
    assert_eq!(connection.state(), &UnifiedConnectionState::Handshaking);
    
    connection.set_state(UnifiedConnectionState::Login);
    assert_eq!(connection.state(), &UnifiedConnectionState::Login);
    
    connection.set_state(UnifiedConnectionState::Play);
    assert_eq!(connection.state(), &UnifiedConnectionState::Play);
    assert!(connection.state().is_active());
    
    // Test encryption setup
    let shared_secret = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    connection.enable_encryption(shared_secret).expect("Failed to enable encryption");
    assert!(connection.is_encrypted());
    
    // Test compression
    connection.enable_compression(6).expect("Failed to enable compression");
    assert!(connection.is_compressed());
    
    manager.stop().await.expect("Failed to stop manager");
}

#[tokio::test]
async fn test_packet_serialization_integration() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let raknet_config = RakNetManagerConfig::default();
    let auth_config = UnifiedAuthConfig::default();
    
    let handler = UnifiedProtocolHandler::new(addr, raknet_config, auth_config).await
        .expect("Failed to create handler");
    
    handler.start().await.expect("Failed to start handler");
    
    // Test packet serialization
    let test_packet = TestBedrockPacket {
        message: "Hello, World!".to_string(),
        value: 42,
    };
    
    let serialized = handler.serialize_packet(&test_packet)
        .expect("Failed to serialize packet");
    
    assert!(!serialized.is_empty());
    
    // Test packet deserialization
    let deserialized: TestBedrockPacket = handler.deserialize_packet(&serialized)
        .expect("Failed to deserialize packet");
    
    assert_eq!(deserialized.message, "Hello, World!");
    assert_eq!(deserialized.value, 42);
    
    handler.stop().await.expect("Failed to stop handler");
}

#[tokio::test]
async fn test_network_performance_integration() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let config = RakNetManagerConfig::default();
    
    let manager = RakNetConnectionManager::new(addr, config).await
        .expect("Failed to create manager");
    
    manager.start().await.expect("Failed to start manager");
    
    // Test packet broadcasting performance
    let start_time = std::time::Instant::now();
    
    for i in 0..1000 {
        let packet = RawBedrockPacket {
            id: 0x01,
            data: Bytes::from(format!("packet_{}", i)),
            direction: PacketDirection::ServerToClient,
        };
        
        let _ = manager.broadcast_packet(packet).await;
    }
    
    let elapsed = start_time.elapsed();
    
    // Should be able to process 1000 packets quickly (under 1 second)
    assert!(elapsed < Duration::from_secs(1));
    
    // Verify performance stats
    let stats = manager.get_enhanced_stats().await;
    assert!(stats.total_connections >= 0);
    
    manager.stop().await.expect("Failed to stop manager");
}

#[tokio::test]
async fn test_error_handling_integration() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let config = RakNetManagerConfig::default();
    
    let manager = RakNetConnectionManager::new(addr, config).await
        .expect("Failed to create manager");
    
    manager.start().await.expect("Failed to start manager");
    
    // Test invalid packet handling
    let invalid_packet = RawBedrockPacket {
        id: 0xFF, // Invalid packet ID
        data: Bytes::from_static(b"invalid data"),
        direction: PacketDirection::ClientToServer,
    };
    
    let result = manager.process_protocol_packet(invalid_packet).await;
    assert!(result.is_err());
    
    // Test connection to non-existent client
    let fake_id = Uuid::new_v4();
    let test_packet = RawBedrockPacket {
        id: 0x01,
        data: Bytes::from_static(b"test"),
        direction: PacketDirection::ServerToClient,
    };
    
    let result = manager.send_packet(fake_id, test_packet).await;
    assert!(result.is_err());
    
    manager.stop().await.expect("Failed to stop manager");
}

// Test packet implementation for serialization testing
#[derive(Debug, Clone, PartialEq)]
struct TestBedrockPacket {
    message: String,
    value: i32,
}

impl mirai_proto::Packet for TestBedrockPacket {
    fn packet_id() -> u32 {
        0x99 // Test packet ID
    }
    
    fn serialize(&self) -> Result<Bytes, BedrockProtocolError> {
        let mut data = Vec::new();
        data.extend_from_slice(&(self.message.len() as u32).to_be_bytes());
        data.extend_from_slice(self.message.as_bytes());
        data.extend_from_slice(&self.value.to_be_bytes());
        Ok(Bytes::from(data))
    }
    
    fn deserialize(data: Bytes) -> Result<Self, BedrockProtocolError> {
        if data.len() < 8 {
            return Err(BedrockProtocolError::InvalidPacketData("Insufficient data".to_string()));
        }
        
        let message_len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + message_len + 4 {
            return Err(BedrockProtocolError::InvalidPacketData("Insufficient data for message".to_string()));
        }
        
        let message = String::from_utf8(data[4..4 + message_len].to_vec())
            .map_err(|_| BedrockProtocolError::InvalidPacketData("Invalid UTF-8".to_string()))?;
        
        let value_start = 4 + message_len;
        let value = i32::from_be_bytes([
            data[value_start],
            data[value_start + 1],
            data[value_start + 2],
            data[value_start + 3],
        ]);
        
        Ok(TestBedrockPacket { message, value })
    }
    
    fn handle_mirai(&self, _client: &mut mirai_proto::BedrockClient) -> Result<(), mirai_proto::MiraiError> {
        // Test packet handling
        Ok(())
    }
}