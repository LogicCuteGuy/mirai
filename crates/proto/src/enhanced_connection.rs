//! Enhanced connection handling that integrates RakNet with unified protocol system
//! 
//! This module provides enhanced connection management that seamlessly handles both
//! Java Edition TCP connections and Bedrock Edition RakNet UDP connections.

use crate::unified::{BedrockProtocolError, RawBedrockPacket};
use crate::connection::{
    BedrockConnection, BedrockConnectionManager, BedrockConnectionState, 
    ConnectionStats
};
use crate::raknet_bridge::{RakNetBridge, EnhancedRakNetClient, RakNetConfig};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Enhanced connection manager that handles Bedrock connections with integrated RakNet support
pub struct EnhancedConnectionManager {
    /// Underlying Bedrock connection manager
    connection_manager: BedrockConnectionManager,
    /// RakNet bridge for Bedrock connections
    raknet_bridge: Arc<RwLock<RakNetBridge>>,
    /// Enhanced RakNet clients
    raknet_clients: Arc<RwLock<HashMap<Uuid, EnhancedRakNetClient>>>,
    /// UDP socket for Bedrock connections
    bedrock_socket: Option<Arc<UdpSocket>>,
    /// Configuration
    config: EnhancedConnectionConfig,
}

impl EnhancedConnectionManager {
    /// Create a new enhanced connection manager
    pub fn new(config: EnhancedConnectionConfig) -> Self {
        Self {
            connection_manager: BedrockConnectionManager::new(config.max_connections),
            raknet_bridge: Arc::new(RwLock::new(RakNetBridge::new())),
            raknet_clients: Arc::new(RwLock::new(HashMap::new())),
            bedrock_socket: None,
            config,
        }
    }
    
    /// Set the UDP socket for Bedrock connections
    pub fn set_bedrock_socket(&mut self, socket: Arc<UdpSocket>) {
        self.bedrock_socket = Some(socket);
    }
    

    
    /// Add a new Bedrock Edition UDP connection
    pub async fn add_bedrock_connection(
        &self,
        address: SocketAddr,
    ) -> Result<Uuid, BedrockProtocolError> {
        let socket = self.bedrock_socket.as_ref()
            .ok_or_else(|| BedrockProtocolError::Connection(
                "No Bedrock socket configured".to_string()
            ))?;
        
        let connection = BedrockConnection::new(socket.clone(), address);
        let connection_id = connection.id;
        
        // Register with RakNet bridge
        {
            let mut bridge = self.raknet_bridge.write().await;
            bridge.register_connection(address, connection_id);
        }
        
        // Create enhanced RakNet client
        let raknet_client = EnhancedRakNetClient::new(
            socket.clone(),
            address,
            self.raknet_bridge.clone(),
            connection_id,
        );
        
        // Store the enhanced client
        {
            let mut clients = self.raknet_clients.write().await;
            clients.insert(connection_id, raknet_client);
        }
        
        self.connection_manager.add_connection(connection).await?;
        
        tracing::info!("Added Bedrock connection {} from {}", connection_id, address);
        Ok(connection_id)
    }
    
    /// Remove a connection
    pub async fn remove_connection(&self, connection_id: Uuid) -> Result<(), BedrockProtocolError> {
        // Remove from connection manager
        if let Some(connection) = self.connection_manager.remove_connection(connection_id).await {
            // Clean up RakNet resources
            // Unregister from RakNet bridge
            {
                let mut bridge = self.raknet_bridge.write().await;
                bridge.unregister_connection(&connection.address);
            }
            
            // Remove enhanced RakNet client
            {
                let mut clients = self.raknet_clients.write().await;
                clients.remove(&connection_id);
            }
            
            tracing::info!("Removed connection {} ({})", connection_id, connection.address);
        }
        
        Ok(())
    }
    
    /// Process incoming Bedrock data through RakNet
    pub async fn process_bedrock_data(
        &self,
        data: &[u8],
        source: SocketAddr,
    ) -> Result<Option<(Uuid, RawBedrockPacket)>, BedrockProtocolError> {
        // Get connection ID from RakNet bridge
        let connection_id = {
            let bridge = self.raknet_bridge.read().await;
            bridge.get_connection_id(&source)
        };
        
        let connection_id = match connection_id {
            Some(id) => id,
            None => {
                // New connection - create it
                let id = self.add_bedrock_connection(source).await?;
                tracing::debug!("Created new Bedrock connection {} for {}", id, source);
                id
            }
        };
        
        // Process through enhanced RakNet client
        let packet = {
            let mut clients = self.raknet_clients.write().await;
            if let Some(client) = clients.get_mut(&connection_id) {
                client.process_incoming_data(data).await?
            } else {
                return Err(BedrockProtocolError::Connection(
                    "RakNet client not found".to_string()
                ));
            }
        };
        
        if let Some(packet) = packet {
            Ok(Some((connection_id, packet)))
        } else {
            Ok(None)
        }
    }
    
    /// Send a packet to a specific connection
    pub async fn send_packet(
        &self,
        connection_id: Uuid,
        packet: RawBedrockPacket,
    ) -> Result<(), BedrockProtocolError> {
        // Send through RakNet client
        let mut clients = self.raknet_clients.write().await;
        if let Some(client) = clients.get_mut(&connection_id) {
            return client.send_bedrock_packet(packet).await;
        }
        
        Err(BedrockProtocolError::Connection(
            "RakNet client not found".to_string()
        ))
    }
    
    /// Broadcast a packet to all Bedrock connections
    pub async fn broadcast_to_all(
        &self,
        packet: RawBedrockPacket,
    ) -> Result<usize, BedrockProtocolError> {
        // Send through all RakNet clients
        let mut sent_count = 0;
        let mut clients = self.raknet_clients.write().await;
        
        for (_, client) in clients.iter_mut() {
            if client.connection().state.is_active() {
                if let Err(e) = client.send_bedrock_packet(packet.clone()).await {
                    tracing::warn!("Failed to send packet to Bedrock client: {}", e);
                } else {
                    sent_count += 1;
                }
            }
        }
        
        Ok(sent_count)
    }
    
    /// Get connection information
    pub async fn get_connection_info(&self, connection_id: Uuid) -> Option<ConnectionInfo> {
        if let Some(connection) = self.connection_manager.get_connection(connection_id).await {
            let mut info = ConnectionInfo {
                id: connection.id,
                address: connection.address,
                state: connection.state,
                protocol_version: connection.protocol_version,
                username: connection.auth_data.username.clone(),
                player_uuid: connection.auth_data.player_uuid,
                stats: connection.get_stats().clone(),
                raknet_info: None,
            };
            
            // Add RakNet-specific information
            let clients = self.raknet_clients.read().await;
            if let Some(client) = clients.get(&connection_id) {
                if let Some(raknet_state) = client.raknet_state().await {
                    info.raknet_info = Some(RakNetConnectionInfo {
                        guid: raknet_state.guid,
                        mtu: raknet_state.mtu,
                        packets_sent: raknet_state.packets_sent,
                        packets_received: raknet_state.packets_received,
                        connection_duration: raknet_state.connection_duration(),
                        is_alive: client.is_alive().await,
                    });
                }
            }
            
            Some(info)
        } else {
            None
        }
    }
    
    /// Get all active connections
    pub async fn get_all_connections(&self) -> Vec<ConnectionInfo> {
        let mut connections = Vec::new();
        
        // Get all Bedrock connections
        let bedrock_connections = self.connection_manager.connections_by_state(BedrockConnectionState::Play).await;
        
        for connection_id in bedrock_connections {
            if let Some(info) = self.get_connection_info(connection_id).await {
                connections.push(info);
            }
        }
        
        connections
    }
    
    /// Get connection statistics
    pub async fn get_connection_stats(&self) -> EnhancedConnectionStats {
        let global_stats = self.connection_manager.get_global_stats().await;
        let raknet_clients = self.raknet_clients.read().await;
        
        let mut total_raknet_packets_sent = 0;
        let mut total_raknet_packets_received = 0;
        let bedrock_connections = raknet_clients.len() as u64;
        
        for client in raknet_clients.values() {
            if let Some(state) = client.raknet_state().await {
                total_raknet_packets_sent += state.packets_sent;
                total_raknet_packets_received += state.packets_received;
            }
        }
        
        EnhancedConnectionStats {
            total_connections: global_stats.total_connections,
            bedrock_connections,
            active_connections: bedrock_connections,
            total_raknet_packets_sent,
            total_raknet_packets_received,
            uptime: global_stats.uptime(),
        }
    }
    
    /// Update RakNet configuration
    pub async fn update_raknet_config(&self, config: RakNetConfig) {
        let mut bridge = self.raknet_bridge.write().await;
        *bridge.config_mut() = config;
        tracing::info!("Updated RakNet configuration");
    }
    
    /// Get current RakNet configuration
    pub async fn get_raknet_config(&self) -> RakNetConfig {
        let bridge = self.raknet_bridge.read().await;
        bridge.config().clone()
    }
    
    /// Cleanup inactive connections
    pub async fn cleanup_inactive_connections(&self) -> usize {
        let mut cleaned_up = 0;
        let mut to_remove = Vec::new();
        
        // Check RakNet clients for inactive connections
        {
            let clients = self.raknet_clients.read().await;
            for (connection_id, client) in clients.iter() {
                if !client.is_alive().await {
                    to_remove.push(*connection_id);
                }
            }
        }
        
        // Remove inactive connections
        for connection_id in to_remove {
            if let Err(e) = self.remove_connection(connection_id).await {
                tracing::warn!("Failed to remove inactive connection {}: {}", connection_id, e);
            } else {
                cleaned_up += 1;
            }
        }
        
        if cleaned_up > 0 {
            tracing::info!("Cleaned up {} inactive connections", cleaned_up);
        }
        
        cleaned_up
    }
    
    /// Shutdown all connections
    pub async fn shutdown(&self) {
        tracing::info!("Shutting down enhanced connection manager");
        
        // Close all connections
        self.connection_manager.close_all().await;
        
        // Clear RakNet clients
        {
            let mut clients = self.raknet_clients.write().await;
            clients.clear();
        }
        
        // Clear RakNet bridge connections
        {
            let mut bridge = self.raknet_bridge.write().await;
            bridge.clear_connections();
        }
        
        tracing::info!("Enhanced connection manager shutdown complete");
    }
}

/// Configuration for enhanced connection manager
#[derive(Debug, Clone)]
pub struct EnhancedConnectionConfig {
    /// Maximum number of connections
    pub max_connections: usize,
    /// Enable RakNet integration
    pub enable_raknet: bool,
    /// RakNet configuration
    pub raknet_config: RakNetConfig,
    /// Connection cleanup interval in seconds
    pub cleanup_interval: u64,
    /// Enable connection statistics
    pub enable_stats: bool,
}

impl Default for EnhancedConnectionConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            enable_raknet: true,
            raknet_config: RakNetConfig::default(),
            cleanup_interval: 60, // 1 minute
            enable_stats: true,
        }
    }
}

/// Comprehensive connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Connection ID
    pub id: Uuid,
    /// Remote address
    pub address: SocketAddr,
    /// Connection state
    pub state: BedrockConnectionState,
    /// Protocol version
    pub protocol_version: Option<i32>,
    /// Player username
    pub username: Option<String>,
    /// Player UUID
    pub player_uuid: Option<Uuid>,
    /// Connection statistics
    pub stats: ConnectionStats,
    /// RakNet-specific information
    pub raknet_info: Option<RakNetConnectionInfo>,
}

/// RakNet-specific connection information
#[derive(Debug, Clone)]
pub struct RakNetConnectionInfo {
    /// Client GUID
    pub guid: u64,
    /// Maximum transfer unit
    pub mtu: u16,
    /// Number of packets sent
    pub packets_sent: u64,
    /// Number of packets received
    pub packets_received: u64,
    /// Connection duration
    pub connection_duration: std::time::Duration,
    /// Whether the connection is still alive
    pub is_alive: bool,
}

/// Enhanced connection statistics
#[derive(Debug, Clone)]
pub struct EnhancedConnectionStats {
    /// Total number of connections ever created
    pub total_connections: u64,
    /// Current number of Bedrock connections
    pub bedrock_connections: u64,
    /// Total active connections
    pub active_connections: u64,
    /// Total RakNet packets sent
    pub total_raknet_packets_sent: u64,
    /// Total RakNet packets received
    pub total_raknet_packets_received: u64,
    /// Manager uptime
    pub uptime: std::time::Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::{TcpListener, TcpStream};
    
    #[tokio::test]
    async fn test_enhanced_connection_manager_creation() {
        let config = EnhancedConnectionConfig::default();
        let manager = EnhancedConnectionManager::new(config);
        
        let stats = manager.get_connection_stats().await;
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.bedrock_connections, 0);
    }
    

    
    #[tokio::test]
    async fn test_bedrock_connection_management() {
        let config = EnhancedConnectionConfig::default();
        let mut manager = EnhancedConnectionManager::new(config);
        
        // Set up UDP socket for Bedrock
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();
        manager.set_bedrock_socket(Arc::new(socket));
        
        // Add Bedrock connection
        let connection_id = manager.add_bedrock_connection(addr).await.unwrap();
        
        // Verify connection was added
        let info = manager.get_connection_info(connection_id).await.unwrap();
        assert_eq!(info.address, addr);
        assert!(info.raknet_info.is_some());
        
        // Remove connection
        manager.remove_connection(connection_id).await.unwrap();
        
        // Verify connection was removed
        assert!(manager.get_connection_info(connection_id).await.is_none());
    }
    
    #[tokio::test]
    async fn test_connection_statistics() {
        let config = EnhancedConnectionConfig::default();
        let mut manager = EnhancedConnectionManager::new(config);
        
        // Set up UDP socket
        let udp_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let udp_addr = udp_socket.local_addr().unwrap();
        manager.set_bedrock_socket(Arc::new(udp_socket));
        
        // Add Bedrock connection
        let bedrock_id = manager.add_bedrock_connection(udp_addr).await.unwrap();
        
        // Check statistics
        let stats = manager.get_connection_stats().await;
        assert_eq!(stats.bedrock_connections, 1);
        assert_eq!(stats.active_connections, 1);
        
        // Get all connections (should be 0 since connection is in Connecting state)
        let connections = manager.get_all_connections().await;
        assert_eq!(connections.len(), 0); // Connection is in Connecting state, not Play
        
        // Verify connection info directly
        let conn_info = manager.get_connection_info(bedrock_id).await.unwrap();
        assert!(conn_info.raknet_info.is_some());
    }
    
    #[tokio::test]
    async fn test_raknet_config_update() {
        let config = EnhancedConnectionConfig::default();
        let manager = EnhancedConnectionManager::new(config);
        
        let original_config = manager.get_raknet_config().await;
        assert_eq!(original_config.mtu, 1400);
        
        let mut new_config = original_config.clone();
        new_config.mtu = 1200;
        new_config.max_connections = 50;
        
        manager.update_raknet_config(new_config.clone()).await;
        
        let updated_config = manager.get_raknet_config().await;
        assert_eq!(updated_config.mtu, 1200);
        assert_eq!(updated_config.max_connections, 50);
    }
    
    #[test]
    fn test_enhanced_connection_config_default() {
        let config = EnhancedConnectionConfig::default();
        assert_eq!(config.max_connections, 1000);
        assert!(config.enable_raknet);
        assert_eq!(config.cleanup_interval, 60);
        assert!(config.enable_stats);
    }
}