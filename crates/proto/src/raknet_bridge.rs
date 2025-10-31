//! RakNet integration bridge for unified protocol system
//! 
//! This module provides integration between the unified protocol system and mirai's
//! existing RakNet implementation, allowing seamless handling of Bedrock Edition packets.

use crate::unified::{
    BedrockProtocolError, RawBedrockPacket, PacketDirection, BedrockPacketRegistry
};
use crate::connection::{BedrockConnection, BedrockConnectionState};
use crate::raknet::{RAKNET_VERSION, OFFLINE_MESSAGE_DATA};
use bytes::{Bytes, BytesMut, BufMut, Buf};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use uuid::Uuid;

/// RakNet integration manager that handles the bridge between Bedrock protocol and RakNet
pub struct RakNetBridge {
    /// Active RakNet connections mapped to Bedrock connections
    raknet_connections: HashMap<SocketAddr, Uuid>,
    /// Packet handlers for different RakNet packet types
    packet_handlers: HashMap<u32, Box<dyn RakNetPacketHandler>>,
    /// RakNet configuration
    config: RakNetConfig,
    /// Connection state tracking
    connection_states: HashMap<Uuid, RakNetConnectionState>,
}

impl RakNetBridge {
    /// Create a new RakNet bridge
    pub fn new() -> Self {
        Self {
            raknet_connections: HashMap::new(),
            packet_handlers: HashMap::new(),
            config: RakNetConfig::default(),
            connection_states: HashMap::new(),
        }
    }
    
    /// Register a RakNet connection with a Bedrock connection
    pub fn register_connection(&mut self, address: SocketAddr, connection_id: Uuid) {
        self.raknet_connections.insert(address, connection_id);
        self.connection_states.insert(connection_id, RakNetConnectionState::new());
        tracing::debug!("Registered RakNet connection {} for address {}", connection_id, address);
    }
    
    /// Unregister a RakNet connection
    pub fn unregister_connection(&mut self, address: &SocketAddr) -> Option<Uuid> {
        let connection_id = self.raknet_connections.remove(address);
        if let Some(id) = connection_id {
            self.connection_states.remove(&id);
            tracing::debug!("Unregistered RakNet connection {} for address {}", id, address);
        }
        connection_id
    }
    
    /// Get the Bedrock connection ID for a RakNet address
    pub fn get_connection_id(&self, address: &SocketAddr) -> Option<Uuid> {
        self.raknet_connections.get(address).copied()
    }
    
    /// Get RakNet connection state
    pub fn get_connection_state(&self, connection_id: &Uuid) -> Option<&RakNetConnectionState> {
        self.connection_states.get(connection_id)
    }
    
    /// Get mutable RakNet connection state
    pub fn get_connection_state_mut(&mut self, connection_id: &Uuid) -> Option<&mut RakNetConnectionState> {
        self.connection_states.get_mut(connection_id)
    }
    
    /// Register a packet handler for a specific RakNet packet type
    pub fn register_packet_handler<H>(&mut self, packet_id: u32, handler: H)
    where
        H: RakNetPacketHandler + 'static,
    {
        self.packet_handlers.insert(packet_id, Box::new(handler));
        tracing::debug!("Registered RakNet packet handler for packet ID 0x{:02x}", packet_id);
    }
    
    /// Process a raw RakNet packet and convert it to Bedrock format
    pub fn process_raknet_packet(
        &mut self,
        data: &[u8],
        source: SocketAddr,
    ) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        if data.is_empty() {
            return Ok(None);
        }
        
        // Update connection state if we have one
        if let Some(connection_id) = self.get_connection_id(&source) {
            if let Some(state) = self.connection_states.get_mut(&connection_id) {
                state.last_packet_time = std::time::Instant::now();
                state.packets_received += 1;
            }
        }
        
        // Check if this is a connected packet (Bedrock game packet)
        if data[0] == 0xfe { // Connected packet marker
            return self.process_connected_packet(&data[1..], source);
        }
        
        // Handle other RakNet protocol packets (connection establishment, etc.)
        self.process_unconnected_packet(data, source)
    }
    
    /// Process a connected Bedrock game packet
    fn process_connected_packet(
        &self,
        data: &[u8],
        source: SocketAddr,
    ) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        if data.is_empty() {
            return Err(BedrockProtocolError::InvalidPacket(
                "Empty connected packet".to_string()
            ));
        }
        
        // Extract packet ID (first byte after connected packet marker)
        let packet_id = data[0] as u32;
        let packet_data = Bytes::copy_from_slice(&data[1..]);
        
        // Create Bedrock packet
        let bedrock_packet = RawBedrockPacket {
            id: packet_id,
            data: packet_data,
            direction: PacketDirection::Serverbound,
        };
        
        tracing::trace!("Processed connected RakNet packet ID 0x{:02x} from {}", packet_id, source);
        Ok(Some(bedrock_packet))
    }
    
    /// Process an unconnected RakNet protocol packet
    fn process_unconnected_packet(
        &self,
        data: &[u8],
        source: SocketAddr,
    ) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        if data.is_empty() {
            return Ok(None);
        }
        
        let packet_id = data[0] as u32;
        
        // Check if we have a handler for this packet type
        if let Some(handler) = self.packet_handlers.get(&packet_id) {
            return handler.handle_packet(data, source);
        }
        
        // Default handling for common RakNet packets
        match packet_id {
            0x01 => self.handle_unconnected_ping(data, source),
            0x1c => self.handle_unconnected_pong(data, source),
            0x05 => self.handle_open_connection_request1(data, source),
            0x06 => self.handle_open_connection_reply1(data, source),
            0x07 => self.handle_open_connection_request2(data, source),
            0x08 => self.handle_open_connection_reply2(data, source),
            0x09 => self.handle_connection_request(data, source),
            0x10 => self.handle_connection_request_accepted(data, source),
            0x13 => self.handle_new_incoming_connection(data, source),
            0x15 => self.handle_disconnect_notification(data, source),
            _ => {
                tracing::warn!("Unknown RakNet packet ID 0x{:02x} from {}", packet_id, source);
                Ok(None)
            }
        }
    }
    
    /// Convert a Bedrock packet back to RakNet format
    pub fn convert_to_raknet(
        &mut self,
        packet: &RawBedrockPacket,
        connection_id: Option<Uuid>,
    ) -> Result<Bytes, BedrockProtocolError> {
        let mut buf = BytesMut::new();
        
        // Add connected packet marker for game packets
        if packet.id != 0x01 && packet.id != 0x1c { // Not ping/pong
            buf.put_u8(0xfe); // Connected packet marker
        }
        
        // Add packet ID
        buf.put_u8(packet.id as u8);
        
        // Add packet data
        buf.extend_from_slice(&packet.data);
        
        // Update connection state if we have one
        if let Some(conn_id) = connection_id {
            if let Some(state) = self.connection_states.get_mut(&conn_id) {
                state.packets_sent += 1;
            }
        }
        
        Ok(buf.freeze())
    }
    
    /// Get RakNet configuration
    pub fn config(&self) -> &RakNetConfig {
        &self.config
    }
    
    /// Get mutable RakNet configuration
    pub fn config_mut(&mut self) -> &mut RakNetConfig {
        &mut self.config
    }
    
    /// Clear all RakNet connections
    pub fn clear_connections(&mut self) {
        self.raknet_connections.clear();
        self.connection_states.clear();
        tracing::debug!("Cleared all RakNet connections");
    }
    
    // RakNet packet handlers
    
    fn handle_unconnected_ping(&self, data: &[u8], source: SocketAddr) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        tracing::trace!("Received unconnected ping from {}", source);
        let packet_data = Bytes::copy_from_slice(&data[1..]);
        Ok(Some(RawBedrockPacket {
            id: 0x01,
            data: packet_data,
            direction: PacketDirection::Serverbound,
        }))
    }
    
    fn handle_unconnected_pong(&self, data: &[u8], source: SocketAddr) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        tracing::trace!("Received unconnected pong from {}", source);
        let packet_data = Bytes::copy_from_slice(&data[1..]);
        Ok(Some(RawBedrockPacket {
            id: 0x1c,
            data: packet_data,
            direction: PacketDirection::Serverbound,
        }))
    }
    
    fn handle_open_connection_request1(&self, data: &[u8], source: SocketAddr) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        tracing::debug!("Received open connection request 1 from {}", source);
        let packet_data = Bytes::copy_from_slice(&data[1..]);
        Ok(Some(RawBedrockPacket {
            id: 0x05,
            data: packet_data,
            direction: PacketDirection::Serverbound,
        }))
    }
    
    fn handle_open_connection_reply1(&self, data: &[u8], source: SocketAddr) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        tracing::debug!("Received open connection reply 1 from {}", source);
        let packet_data = Bytes::copy_from_slice(&data[1..]);
        Ok(Some(RawBedrockPacket {
            id: 0x06,
            data: packet_data,
            direction: PacketDirection::Clientbound,
        }))
    }
    
    fn handle_open_connection_request2(&self, data: &[u8], source: SocketAddr) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        tracing::debug!("Received open connection request 2 from {}", source);
        let packet_data = Bytes::copy_from_slice(&data[1..]);
        Ok(Some(RawBedrockPacket {
            id: 0x07,
            data: packet_data,
            direction: PacketDirection::Serverbound,
        }))
    }
    
    fn handle_open_connection_reply2(&self, data: &[u8], source: SocketAddr) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        tracing::debug!("Received open connection reply 2 from {}", source);
        let packet_data = Bytes::copy_from_slice(&data[1..]);
        Ok(Some(RawBedrockPacket {
            id: 0x08,
            data: packet_data,
            direction: PacketDirection::Clientbound,
        }))
    }
    
    fn handle_connection_request(&self, data: &[u8], source: SocketAddr) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        tracing::debug!("Received connection request from {}", source);
        let packet_data = Bytes::copy_from_slice(&data[1..]);
        Ok(Some(RawBedrockPacket {
            id: 0x09,
            data: packet_data,
            direction: PacketDirection::Serverbound,
        }))
    }
    
    fn handle_connection_request_accepted(&self, data: &[u8], source: SocketAddr) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        tracing::debug!("Received connection request accepted from {}", source);
        let packet_data = Bytes::copy_from_slice(&data[1..]);
        Ok(Some(RawBedrockPacket {
            id: 0x10,
            data: packet_data,
            direction: PacketDirection::Clientbound,
        }))
    }
    
    fn handle_new_incoming_connection(&self, data: &[u8], source: SocketAddr) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        tracing::debug!("Received new incoming connection from {}", source);
        let packet_data = Bytes::copy_from_slice(&data[1..]);
        Ok(Some(RawBedrockPacket {
            id: 0x13,
            data: packet_data,
            direction: PacketDirection::Serverbound,
        }))
    }
    
    fn handle_disconnect_notification(&self, data: &[u8], source: SocketAddr) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        tracing::debug!("Received disconnect notification from {}", source);
        let packet_data = Bytes::copy_from_slice(&data[1..]);
        Ok(Some(RawBedrockPacket {
            id: 0x15,
            data: packet_data,
            direction: PacketDirection::Serverbound,
        }))
    }
}

impl Default for RakNetBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for handling specific RakNet packet types
pub trait RakNetPacketHandler: Send + Sync {
    /// Handle a RakNet packet and optionally return a Bedrock packet
    fn handle_packet(
        &self,
        data: &[u8],
        source: SocketAddr,
    ) -> Result<Option<RawBedrockPacket>, BedrockProtocolError>;
}

/// RakNet configuration
#[derive(Debug, Clone)]
pub struct RakNetConfig {
    /// Maximum transfer unit
    pub mtu: u16,
    /// RakNet protocol version
    pub protocol_version: u8,
    /// Server GUID
    pub server_guid: u64,
    /// Maximum number of connections
    pub max_connections: u32,
    /// Connection timeout in milliseconds
    pub connection_timeout: u64,
    /// Enable packet loss simulation (for testing)
    pub simulate_packet_loss: bool,
    /// Packet loss percentage (0-100)
    pub packet_loss_percentage: u8,
}

impl Default for RakNetConfig {
    fn default() -> Self {
        Self {
            mtu: 1400,
            protocol_version: RAKNET_VERSION,
            server_guid: rand::random(),
            max_connections: 100,
            connection_timeout: 30000, // 30 seconds
            simulate_packet_loss: false,
            packet_loss_percentage: 0,
        }
    }
}

/// RakNet connection state tracking
#[derive(Debug, Clone)]
pub struct RakNetConnectionState {
    /// Client GUID
    pub guid: u64,
    /// Maximum transfer unit
    pub mtu: u16,
    /// Last packet receive time
    pub last_packet_time: std::time::Instant,
    /// Number of packets sent
    pub packets_sent: u64,
    /// Number of packets received
    pub packets_received: u64,
    /// Connection establishment time
    pub connected_at: std::time::Instant,
}

impl RakNetConnectionState {
    /// Create new RakNet connection state
    pub fn new() -> Self {
        let now = std::time::Instant::now();
        Self {
            guid: rand::random(),
            mtu: 1400,
            last_packet_time: now,
            packets_sent: 0,
            packets_received: 0,
            connected_at: now,
        }
    }
    
    /// Get connection duration
    pub fn connection_duration(&self) -> std::time::Duration {
        self.last_packet_time.duration_since(self.connected_at)
    }
    
    /// Check if connection is alive based on timeout
    pub fn is_alive(&self, timeout: std::time::Duration) -> bool {
        self.last_packet_time.elapsed() < timeout
    }
}

impl Default for RakNetConnectionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Enhanced RakNet client that integrates with the Bedrock protocol system
pub struct EnhancedRakNetClient {
    /// Underlying Bedrock connection
    connection: BedrockConnection,
    /// RakNet bridge for packet processing
    bridge: Arc<tokio::sync::RwLock<RakNetBridge>>,
    /// Connection ID
    connection_id: Uuid,
}

impl EnhancedRakNetClient {
    /// Create a new enhanced RakNet client
    pub fn new(
        socket: Arc<UdpSocket>,
        address: SocketAddr,
        bridge: Arc<tokio::sync::RwLock<RakNetBridge>>,
        connection_id: Uuid,
    ) -> Self {
        let connection = BedrockConnection::new(socket, address);
        
        Self {
            connection,
            bridge,
            connection_id,
        }
    }
    
    /// Get the underlying Bedrock connection
    pub fn connection(&self) -> &BedrockConnection {
        &self.connection
    }
    
    /// Get mutable access to the underlying Bedrock connection
    pub fn connection_mut(&mut self) -> &mut BedrockConnection {
        &mut self.connection
    }
    
    /// Get connection ID
    pub fn connection_id(&self) -> Uuid {
        self.connection_id
    }
    
    /// Get RakNet-specific state
    pub async fn raknet_state(&self) -> Option<RakNetConnectionState> {
        let bridge = self.bridge.read().await;
        bridge.get_connection_state(&self.connection_id).cloned()
    }
    
    /// Process incoming RakNet data
    pub async fn process_incoming_data(&mut self, data: &[u8]) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        // Process through RakNet bridge
        let mut bridge = self.bridge.write().await;
        bridge.process_raknet_packet(data, self.connection.address)
    }
    
    /// Send a Bedrock packet through RakNet
    pub async fn send_bedrock_packet(&mut self, packet: RawBedrockPacket) -> Result<(), BedrockProtocolError> {
        // Convert to RakNet format
        let raknet_data = {
            let mut bridge = self.bridge.write().await;
            bridge.convert_to_raknet(&packet, Some(self.connection_id))?
        };
        
        // Create a new Bedrock packet with the RakNet-formatted data
        let raknet_packet = RawBedrockPacket {
            id: packet.id,
            data: raknet_data,
            direction: packet.direction,
        };
        
        // Send through Bedrock connection
        self.connection.write_packet(raknet_packet).await?;
        
        Ok(())
    }
    
    /// Check if the RakNet connection is still alive
    pub async fn is_alive(&self) -> bool {
        let bridge = self.bridge.read().await;
        let timeout = std::time::Duration::from_millis(bridge.config().connection_timeout);
        
        if let Some(state) = bridge.get_connection_state(&self.connection_id) {
            state.is_alive(timeout)
        } else {
            false
        }
    }
}



/// Utility functions for RakNet integration
pub mod utils {
    use super::*;
    
    /// Check if data contains the RakNet offline message data
    pub fn has_offline_message_data(data: &[u8]) -> bool {
        if data.len() < OFFLINE_MESSAGE_DATA.len() {
            return false;
        }
        
        data.windows(OFFLINE_MESSAGE_DATA.len())
            .any(|window| window == OFFLINE_MESSAGE_DATA)
    }
    
    /// Extract MTU from RakNet connection request
    pub fn extract_mtu_from_request(data: &[u8]) -> Option<u16> {
        // This is a simplified implementation
        // In a real implementation, you'd parse the actual RakNet packet structure
        if data.len() >= 3 {
            Some(u16::from_be_bytes([data[1], data[2]]))
        } else {
            None
        }
    }
    
    /// Create a RakNet server info string for pong responses
    pub fn create_server_info(
        server_name: &str,
        protocol_version: u32,
        player_count: u32,
        max_players: u32,
    ) -> String {
        format!(
            "MCPE;{};{};1.21.0;{};{};0;Mirai;Survival;1;19132;19133;",
            server_name,
            protocol_version,
            player_count,
            max_players
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_raknet_bridge_creation() {
        let bridge = RakNetBridge::new();
        assert!(bridge.raknet_connections.is_empty());
        assert!(bridge.packet_handlers.is_empty());
        assert!(bridge.connection_states.is_empty());
    }
    
    #[test]
    fn test_connection_registration() {
        let mut bridge = RakNetBridge::new();
        let addr = "127.0.0.1:19132".parse().unwrap();
        let conn_id = Uuid::new_v4();
        
        bridge.register_connection(addr, conn_id);
        assert_eq!(bridge.get_connection_id(&addr), Some(conn_id));
        assert!(bridge.get_connection_state(&conn_id).is_some());
        
        let removed_id = bridge.unregister_connection(&addr);
        assert_eq!(removed_id, Some(conn_id));
        assert_eq!(bridge.get_connection_id(&addr), None);
        assert!(bridge.get_connection_state(&conn_id).is_none());
    }
    
    #[test]
    fn test_connected_packet_processing() {
        let mut bridge = RakNetBridge::new();
        let addr = "127.0.0.1:19132".parse().unwrap();
        
        // Test connected packet (starts with 0xfe)
        let data = vec![0xfe, 0x01, 0x02, 0x03, 0x04];
        let result = bridge.process_raknet_packet(&data, addr).unwrap();
        
        assert!(result.is_some());
        let packet = result.unwrap();
        assert_eq!(packet.id, 0x01);
        assert_eq!(packet.data, Bytes::from_static(&[0x02, 0x03, 0x04]));
        assert_eq!(packet.direction, PacketDirection::Serverbound);
    }
    
    #[test]
    fn test_unconnected_ping_processing() {
        let mut bridge = RakNetBridge::new();
        let addr = "127.0.0.1:19132".parse().unwrap();
        
        // Test unconnected ping (packet ID 0x01)
        let data = vec![0x01, 0x12, 0x34, 0x56, 0x78];
        let result = bridge.process_raknet_packet(&data, addr).unwrap();
        
        assert!(result.is_some());
        let packet = result.unwrap();
        assert_eq!(packet.id, 0x01);
        assert_eq!(packet.data, Bytes::from_static(&[0x12, 0x34, 0x56, 0x78]));
        assert_eq!(packet.direction, PacketDirection::Serverbound);
    }
    
    #[test]
    fn test_packet_conversion_to_raknet() {
        let mut bridge = RakNetBridge::new();
        let packet = RawBedrockPacket {
            id: 0x01,
            data: Bytes::from_static(&[0x12, 0x34]),
            direction: PacketDirection::Serverbound,
        };
        
        let raknet_data = bridge.convert_to_raknet(&packet, None).unwrap();
        
        // Should contain packet ID + data (no connected packet marker for ping)
        assert_eq!(raknet_data.len(), 3);
        assert_eq!(raknet_data[0], 0x01); // packet ID
        assert_eq!(raknet_data[1], 0x12); // data
        assert_eq!(raknet_data[2], 0x34); // data
    }
    
    #[test]
    fn test_game_packet_conversion_to_raknet() {
        let mut bridge = RakNetBridge::new();
        let packet = RawBedrockPacket {
            id: 0x09,
            data: Bytes::from_static(&[0xab, 0xcd]),
            direction: PacketDirection::Serverbound,
        };
        
        let raknet_data = bridge.convert_to_raknet(&packet, None).unwrap();
        
        // Should contain connected packet marker + packet ID + data
        assert_eq!(raknet_data.len(), 4);
        assert_eq!(raknet_data[0], 0xfe); // connected packet marker
        assert_eq!(raknet_data[1], 0x09); // packet ID
        assert_eq!(raknet_data[2], 0xab); // data
        assert_eq!(raknet_data[3], 0xcd); // data
    }
    
    #[test]
    fn test_raknet_config_default() {
        let config = RakNetConfig::default();
        assert_eq!(config.mtu, 1400);
        assert_eq!(config.protocol_version, RAKNET_VERSION);
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.connection_timeout, 30000);
        assert!(!config.simulate_packet_loss);
        assert_eq!(config.packet_loss_percentage, 0);
    }
    
    #[test]
    fn test_raknet_connection_state() {
        let state = RakNetConnectionState::new();
        assert!(state.guid != 0);
        assert_eq!(state.mtu, 1400);
        assert_eq!(state.packets_sent, 0);
        assert_eq!(state.packets_received, 0);
        
        // Connection duration should be very small for a new state
        assert!(state.connection_duration().as_millis() < 100);
        
        // Test alive check
        let timeout = std::time::Duration::from_secs(1);
        assert!(state.is_alive(timeout));
    }
    
    #[test]
    fn test_offline_message_data_detection() {
        let data_with_offline = [
            0x01, 0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 
            0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78, 0x99
        ];
        let data_without_offline = [0x01, 0x02, 0x03, 0x04];
        
        assert!(utils::has_offline_message_data(&data_with_offline));
        assert!(!utils::has_offline_message_data(&data_without_offline));
    }
    
    #[test]
    fn test_server_info_creation() {
        let info = utils::create_server_info("Test Server", 686, 5, 20);
        assert!(info.contains("Test Server"));
        assert!(info.contains("686"));
        assert!(info.contains("5"));
        assert!(info.contains("20"));
        assert!(info.contains("MCPE"));
    }
}
