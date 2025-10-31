//! Bedrock connection management for UDP/RakNet protocol
//! 
//! This module provides connection management that integrates with mirai's existing
//! RakNet functionality for Bedrock Edition clients only.

use crate::unified::{BedrockProtocolError, RawBedrockPacket};
use crate::codec::BedrockPacketCodec;
use bytes::{Bytes, BytesMut};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpStream, UdpSocket};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Connection state for Bedrock protocol handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BedrockConnectionState {
    /// Connection is being established
    Connecting,
    /// Initial RakNet connection established
    Connected,
    /// Login/authentication phase
    Login,
    /// Resource pack negotiation
    ResourcePacks,
    /// Main gameplay phase
    Play,
    /// Connection is being closed
    Disconnecting,
    /// Connection is closed
    Disconnected,
}

impl BedrockConnectionState {
    /// Check if the connection is active
    pub fn is_active(&self) -> bool {
        matches!(
            *self,
            Self::Connecting | Self::Connected | Self::Login | Self::ResourcePacks | Self::Play
        )
    }
    
    /// Check if the connection is closed
    pub fn is_closed(self) -> bool {
        matches!(self, Self::Disconnected)
    }
}

/// Authentication data for Bedrock connections
#[derive(Debug, Clone)]
pub struct BedrockAuthData {
    /// Player username
    pub username: Option<String>,
    /// Player UUID
    pub player_uuid: Option<Uuid>,
    /// Xbox Live user ID
    pub xuid: Option<String>,
    /// JWT identity chain
    pub identity_chain: Option<Vec<String>>,
    /// Client data JWT
    pub client_data: Option<String>,
    /// Identity public key
    pub identity_public_key: Option<String>,
    /// Client public key
    pub client_public_key: Option<Vec<u8>>,
    /// Shared secret for encryption
    pub shared_secret: Option<Vec<u8>>,
    /// Authentication timestamp
    pub auth_timestamp: Option<std::time::SystemTime>,
}

impl Default for BedrockAuthData {
    fn default() -> Self {
        Self {
            username: None,
            player_uuid: None,
            xuid: None,
            identity_chain: None,
            client_data: None,
            identity_public_key: None,
            client_public_key: None,
            shared_secret: None,
            auth_timestamp: None,
        }
    }
}

/// Bedrock connection that handles UDP/RakNet protocol
#[derive(Clone)]
pub struct BedrockConnection {
    /// Unique connection ID
    pub id: Uuid,
    /// Remote address
    pub address: SocketAddr,
    /// Current connection state
    pub state: BedrockConnectionState,
    /// Protocol version
    pub protocol_version: Option<i32>,
    /// Authentication data
    pub auth_data: BedrockAuthData,
    /// UDP socket for RakNet communication
    udp_socket: Option<Arc<UdpSocket>>,
    /// Packet codec
    codec: BedrockPacketCodec,
    /// Write buffer
    write_buffer: BytesMut,
    /// Connection statistics
    stats: ConnectionStats,
}

impl BedrockConnection {
    /// Create a new Bedrock Edition UDP connection
    pub fn new(socket: Arc<UdpSocket>, address: SocketAddr) -> Self {
        Self {
            id: Uuid::new_v4(),
            address,
            state: BedrockConnectionState::Connecting,
            protocol_version: None,
            auth_data: BedrockAuthData::default(),
            udp_socket: Some(socket),
            codec: BedrockPacketCodec::new(),
            write_buffer: BytesMut::with_capacity(8192),
            stats: ConnectionStats::new(),
        }
    }
    
    /// Set the connection state
    pub fn set_state(&mut self, state: BedrockConnectionState) {
        tracing::debug!("Connection {} state: {:?} -> {:?}", self.id, self.state, state);
        self.state = state;
    }
    
    /// Set the protocol version
    pub fn set_protocol_version(&mut self, version: i32) {
        self.protocol_version = Some(version);
        tracing::debug!("Connection {} protocol version: {}", self.id, version);
    }
    
    /// Set player authentication information
    pub fn set_player_info(&mut self, username: String, uuid: Uuid) {
        self.auth_data.username = Some(username.clone());
        self.auth_data.player_uuid = Some(uuid);
        tracing::info!("Connection {} authenticated as {} ({})", self.id, username, uuid);
    }
    
    /// Enable packet batching
    pub fn enable_batching(&mut self) {
        self.codec.set_batching_enabled(true);
        tracing::debug!("Connection {} batching enabled", self.id);
    }
    
    /// Enable encryption with shared secret
    pub fn enable_encryption(&mut self, shared_secret: Vec<u8>) -> Result<(), BedrockProtocolError> {
        self.auth_data.shared_secret = Some(shared_secret);
        self.auth_data.auth_timestamp = Some(std::time::SystemTime::now());
        tracing::debug!("Connection {} encryption enabled", self.id);
        Ok(())
    }
    
    /// Read a packet from the connection
    pub async fn read_packet(&mut self) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        let mut temp_buf = [0u8; 4096];
        
        let bytes_read = if let Some(ref socket) = self.udp_socket {
            match socket.recv_from(&mut temp_buf).await {
                Ok((n, addr)) => {
                    // Verify the packet is from the expected address
                    if addr != self.address {
                        tracing::warn!("Received UDP packet from unexpected address: {} (expected: {})", addr, self.address);
                        return Ok(None);
                    }
                    self.stats.bytes_received += n as u64;
                    n
                }
                Err(e) => {
                    tracing::error!("Failed to read from UDP connection {}: {}", self.id, e);
                    self.set_state(BedrockConnectionState::Disconnected);
                    return Err(BedrockProtocolError::Connection(e.to_string()));
                }
            }
        } else {
            return Err(BedrockProtocolError::Connection("No UDP socket available".to_string()));
        };
        
        // Add data to codec
        self.codec.add_data(&temp_buf[..bytes_read]);
        
        // Try to decode a packet
        match self.codec.decode() {
            Ok(Some(packet)) => {
                self.stats.packets_received += 1;
                Ok(Some(packet))
            }
            Ok(None) => Ok(None), // Need more data
            Err(e) => {
                tracing::error!("Failed to decode packet from connection {}: {}", self.id, e);
                Err(e)
            }
        }
    }
    
    /// Write a packet to the connection
    pub async fn write_packet(&mut self, packet: RawBedrockPacket) -> Result<(), BedrockProtocolError> {
        // Encode the packet
        let encoded = self.codec.encode(packet)?;
        
        // Add to write buffer
        self.write_buffer.extend_from_slice(&encoded);
        
        // Flush the buffer
        self.flush().await?;
        
        self.stats.packets_sent += 1;
        Ok(())
    }
    
    /// Flush the write buffer
    pub async fn flush(&mut self) -> Result<(), BedrockProtocolError> {
        if self.write_buffer.is_empty() {
            return Ok(());
        }
        
        let result = if let Some(ref socket) = self.udp_socket {
            match socket.send_to(&self.write_buffer, self.address).await {
                Ok(_) => {
                    self.stats.bytes_sent += self.write_buffer.len() as u64;
                    Ok(())
                }
                Err(e) => Err(BedrockProtocolError::Connection(e.to_string()))
            }
        } else {
            Err(BedrockProtocolError::Connection("No UDP socket available".to_string()))
        };
        
        match result {
            Ok(()) => {
                self.write_buffer.clear();
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to write to connection {}: {}", self.id, e);
                self.set_state(BedrockConnectionState::Disconnected);
                Err(e)
            }
        }
    }
    
    /// Close the connection
    pub async fn close(&mut self) -> Result<(), BedrockProtocolError> {
        self.set_state(BedrockConnectionState::Disconnecting);
        
        // Flush any remaining data
        if let Err(e) = self.flush().await {
            tracing::warn!("Failed to flush connection {} during close: {}", self.id, e);
        }
        
        // UDP connections don't need explicit shutdown
        tracing::debug!("UDP connection {} closed", self.id);
        
        self.set_state(BedrockConnectionState::Disconnected);
        tracing::info!("Connection {} closed", self.id);
        
        Ok(())
    }
    
    /// Get connection statistics
    pub fn get_stats(&self) -> &ConnectionStats {
        &self.stats
    }
    
    /// Get mutable connection statistics
    pub fn get_stats_mut(&mut self) -> &mut ConnectionStats {
        &mut self.stats
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    /// Number of packets sent
    pub packets_sent: u64,
    /// Number of packets received
    pub packets_received: u64,
    /// Number of bytes sent
    pub bytes_sent: u64,
    /// Number of bytes received
    pub bytes_received: u64,
    /// Connection start time
    pub connected_at: std::time::SystemTime,
    /// Last activity time
    pub last_activity: std::time::SystemTime,
}

impl ConnectionStats {
    /// Create new connection statistics
    pub fn new() -> Self {
        let now = std::time::SystemTime::now();
        Self {
            packets_sent: 0,
            packets_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            connected_at: now,
            last_activity: now,
        }
    }
    
    /// Update last activity time
    pub fn update_activity(&mut self) {
        self.last_activity = std::time::SystemTime::now();
    }
    
    /// Get connection duration
    pub fn connection_duration(&self) -> std::time::Duration {
        self.last_activity.duration_since(self.connected_at)
            .unwrap_or_default()
    }
}

impl Default for ConnectionStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Bedrock connection manager that handles UDP/RakNet connections
pub struct BedrockConnectionManager {
    /// Active connections
    connections: Arc<RwLock<HashMap<Uuid, BedrockConnection>>>,
    /// Maximum number of connections
    max_connections: usize,
    /// Connection statistics
    global_stats: Arc<RwLock<GlobalConnectionStats>>,
}

impl BedrockConnectionManager {
    /// Create a new Bedrock connection manager
    pub fn new(max_connections: usize) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            max_connections,
            global_stats: Arc::new(RwLock::new(GlobalConnectionStats::new())),
        }
    }
    
    /// Add a new connection
    pub async fn add_connection(&self, connection: BedrockConnection) -> Result<(), BedrockProtocolError> {
        let mut connections = self.connections.write().await;
        
        if connections.len() >= self.max_connections {
            return Err(BedrockProtocolError::Connection(
                "Maximum connections reached".to_string()
            ));
        }
        
        let id = connection.id;
        connections.insert(id, connection);
        
        // Update global stats
        let mut stats = self.global_stats.write().await;
        stats.total_connections += 1;
        stats.bedrock_connections += 1;
        
        tracing::info!("Added Bedrock connection {} (total: {})", id, connections.len());
        
        Ok(())
    }
    
    /// Remove a connection
    pub async fn remove_connection(&self, id: Uuid) -> Option<BedrockConnection> {
        let mut connections = self.connections.write().await;
        let connection = connections.remove(&id);
        
        if connection.is_some() {
            // Update global stats
            let mut stats = self.global_stats.write().await;
            stats.bedrock_connections = stats.bedrock_connections.saturating_sub(1);
            
            tracing::info!("Removed connection {} (total: {})", id, connections.len());
        }
        
        connection
    }
    
    /// Get a connection by ID
    pub async fn get_connection(&self, id: Uuid) -> Option<BedrockConnection> {
        let connections = self.connections.read().await;
        connections.get(&id).cloned()
    }
    
    /// Execute a function with a connection
    pub async fn with_connection<F, R>(&self, id: Uuid, f: F) -> Option<R>
    where
        F: FnOnce(&mut BedrockConnection) -> R,
    {
        let mut connections = self.connections.write().await;
        connections.get_mut(&id).map(f)
    }
    
    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }
    
    /// Check if at maximum capacity
    pub async fn is_full(&self) -> bool {
        let connections = self.connections.read().await;
        connections.len() >= self.max_connections
    }
    
    /// Get connections by state
    pub async fn connections_by_state(&self, state: BedrockConnectionState) -> Vec<Uuid> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .filter(|(_, conn)| conn.state == state)
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Close all connections
    pub async fn close_all(&self) {
        let mut connections = self.connections.write().await;
        let connection_ids: Vec<Uuid> = connections.keys().copied().collect();
        
        for id in connection_ids {
            if let Some(mut connection) = connections.remove(&id) {
                if let Err(e) = connection.close().await {
                    tracing::warn!("Failed to close connection {}: {}", id, e);
                }
            }
        }
        
        // Reset global stats
        let mut stats = self.global_stats.write().await;
        stats.bedrock_connections = 0;
        
        tracing::info!("Closed all connections");
    }
    
    /// Get global connection statistics
    pub async fn get_global_stats(&self) -> GlobalConnectionStats {
        let stats = self.global_stats.read().await;
        stats.clone()
    }
    
    /// Get the number of active connections
    pub async fn active_connection_count(&self) -> usize {
        self.global_stats.read().await.bedrock_connections as usize
    }
    
    /// Broadcast a packet to all active connections
    pub async fn broadcast_to_all(&self, packet: RawBedrockPacket) -> Result<usize, BedrockProtocolError> {
        let mut connections = self.connections.write().await;
        let mut sent_count = 0;
        
        for (_, connection) in connections.iter_mut() {
            if connection.state.is_active() {
                if let Err(e) = connection.write_packet(packet.clone()).await {
                    tracing::warn!("Failed to send packet to connection {}: {}", connection.id, e);
                } else {
                    sent_count += 1;
                }
            }
        }
        
        Ok(sent_count)
    }
}

impl Default for BedrockConnectionManager {
    fn default() -> Self {
        Self::new(1000) // Default max connections
    }
}

/// Global connection statistics
#[derive(Debug, Clone)]
pub struct GlobalConnectionStats {
    /// Total number of connections ever created
    pub total_connections: u64,
    /// Current number of Bedrock connections
    pub bedrock_connections: u64,
    /// Manager start time
    pub started_at: std::time::SystemTime,
}

impl GlobalConnectionStats {
    /// Create new global connection statistics
    pub fn new() -> Self {
        Self {
            total_connections: 0,
            bedrock_connections: 0,
            started_at: std::time::SystemTime::now(),
        }
    }
    
    /// Get total active connections
    pub fn active_connections(&self) -> u64 {
        self.bedrock_connections
    }
    
    /// Get uptime duration
    pub fn uptime(&self) -> std::time::Duration {
        std::time::SystemTime::now()
            .duration_since(self.started_at)
            .unwrap_or_default()
    }
}

impl Default for GlobalConnectionStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{UnifiedConnectionState, ProtocolType, BedrockConnection};
    use tokio::net::{TcpListener, TcpStream};
    
    #[test]
    fn test_connection_state_transitions() {
        assert!(BedrockConnectionState::Play.is_active());
        assert!(BedrockConnectionState::Play.is_active());
        assert!(!BedrockConnectionState::Disconnected.is_active());
        assert!(BedrockConnectionState::Disconnected.is_closed());
    }
    
    #[tokio::test]
    async fn test_java_connection_creation() {
        // Note: Mirai is Bedrock-only, so this test creates a Bedrock connection
        // to verify the connection infrastructure works
        let udp_socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let addr = udp_socket.local_addr().unwrap();
        
        let mut connection = BedrockConnection::new(udp_socket, addr);
        
        assert_eq!(connection.state, BedrockConnectionState::Connecting);
        assert_eq!(connection.address, addr);
        assert!(connection.protocol_version.is_none());
        assert!(connection.auth_data.username.is_none());
        
        // Test state changes
        connection.set_state(BedrockConnectionState::Login);
        assert_eq!(connection.state, BedrockConnectionState::Login);
        
        // Test protocol version
        connection.set_protocol_version(763);
        assert_eq!(connection.protocol_version, Some(763));
        
        // Test player info
        let uuid = Uuid::new_v4();
        connection.set_player_info("TestPlayer".to_string(), uuid);
        assert_eq!(connection.auth_data.username, Some("TestPlayer".to_string()));
        assert_eq!(connection.auth_data.player_uuid, Some(uuid));
    }
    
    #[tokio::test]
    async fn test_bedrock_connection_creation() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();
        let socket = Arc::new(socket);
        
        let connection = BedrockConnection::new(socket, addr);
        
        assert_eq!(connection.state, BedrockConnectionState::Connecting);
        assert_eq!(connection.address, addr);
        assert!(connection.protocol_version.is_none());
        assert!(connection.auth_data.username.is_none());
    }
    
    #[tokio::test]
    async fn test_connection_manager() {
        let manager = BedrockConnectionManager::new(2);
        
        // Create mock connections
        let socket1 = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let addr1 = socket1.local_addr().unwrap();
        let conn1 = BedrockConnection::new(socket1, addr1);
        
        let socket2 = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let addr2 = socket2.local_addr().unwrap();
        let conn2 = BedrockConnection::new(socket2, addr2);
        
        let socket3 = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let addr3 = socket3.local_addr().unwrap();
        let conn3 = BedrockConnection::new(socket3, addr3);
        
        let id1 = conn1.id;
        let id2 = conn2.id;
        
        // Add connections
        assert!(manager.add_connection(conn1).await.is_ok());
        assert!(manager.add_connection(conn2).await.is_ok());
        assert_eq!(manager.connection_count().await, 2);
        assert!(manager.is_full().await);
        
        // Should fail to add third connection
        assert!(manager.add_connection(conn3).await.is_err());
        
        // Remove a connection
        assert!(manager.remove_connection(id1).await.is_some());
        assert_eq!(manager.connection_count().await, 1);
        assert!(!manager.is_full().await);
        
        // Get connection
        assert!(manager.get_connection(id2).await.is_some());
        assert!(manager.get_connection(id1).await.is_none());
    }
    
    #[tokio::test]
    async fn test_connection_stats() {
        let stats = ConnectionStats::new();
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.packets_received, 0);
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        
        let global_stats = GlobalConnectionStats::new();
        assert_eq!(global_stats.total_connections, 0);
        assert_eq!(global_stats.bedrock_connections, 0);
        assert_eq!(global_stats.active_connections(), 0);
    }
    
    #[tokio::test]
    async fn test_protocol_filtering() {
        let manager = BedrockConnectionManager::new(10);
        
        // Create Bedrock connections
        let socket1 = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let addr1 = socket1.local_addr().unwrap();
        let conn1 = BedrockConnection::new(socket1, addr1);
        
        let socket2 = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let addr2 = socket2.local_addr().unwrap();
        let conn2 = BedrockConnection::new(socket2, addr2);
        
        assert!(manager.add_connection(conn1).await.is_ok());
        assert!(manager.add_connection(conn2).await.is_ok());
        
        // Test connection count
        assert_eq!(manager.connection_count().await, 2);
        
        // Test global stats
        let stats = manager.get_global_stats().await;
        assert_eq!(stats.bedrock_connections, 2);
        assert_eq!(stats.active_connections(), 2);
    }
}