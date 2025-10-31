//! Bedrock protocol system for mirai
//! 
//! This module provides the core interface for handling Bedrock Edition (UDP/RakNet) 
//! protocol packets. Mirai is a Bedrock Edition server and only supports Bedrock protocol.

use crate::bedrock::ConnectedPacket;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::collections::HashMap;
use uuid::Uuid;

/// Bedrock packet trait for all Bedrock protocol packets
pub trait BedrockPacket: Send + Sync + Sized {
    /// Get the packet ID
    fn packet_id(&self) -> u32;
    
    /// Serialize the packet to bytes
    fn serialize(&self) -> Result<Bytes, BedrockProtocolError>;
    
    /// Deserialize the packet from bytes
    fn deserialize(data: Bytes) -> Result<Self, BedrockProtocolError>;
    
    /// Get the packet direction
    fn direction() -> PacketDirection;
    
    /// Get estimated serialized size
    fn estimated_size(&self) -> usize {
        match self.serialize() {
            Ok(data) => data.len(),
            Err(_) => 512, // Fallback estimate
        }
    }
}

/// Packet direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PacketDirection {
    /// Client to server
    Serverbound,
    /// Server to client
    Clientbound,
    /// Bidirectional
    Bidirectional,
}

/// Bedrock protocol error type
#[derive(Debug)]
pub enum BedrockProtocolError {
    SerializationFailed(String),
    DeserializationFailed(String),
    InvalidPacket(String),
    BufferUnderflow { expected: usize, actual: usize },
    BufferOverflow { max_size: usize },
    Compression(String),
    Encryption(String),
    Connection(String),
    UnsupportedOperation(String),
}

impl std::fmt::Display for BedrockProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BedrockProtocolError::SerializationFailed(msg) => write!(f, "Serialization failed: {}", msg),
            BedrockProtocolError::DeserializationFailed(msg) => write!(f, "Deserialization failed: {}", msg),
            BedrockProtocolError::InvalidPacket(msg) => write!(f, "Invalid packet: {}", msg),
            BedrockProtocolError::BufferUnderflow { expected, actual } => {
                write!(f, "Buffer underflow: expected {}, got {}", expected, actual)
            }
            BedrockProtocolError::BufferOverflow { max_size } => {
                write!(f, "Buffer overflow: maximum size {} exceeded", max_size)
            }
            BedrockProtocolError::Compression(msg) => write!(f, "Compression error: {}", msg),
            BedrockProtocolError::Encryption(msg) => write!(f, "Encryption error: {}", msg),
            BedrockProtocolError::Connection(msg) => write!(f, "Connection error: {}", msg),
            BedrockProtocolError::UnsupportedOperation(msg) => write!(f, "Unsupported operation: {}", msg),
        }
    }
}

impl std::error::Error for BedrockProtocolError {}

/// Raw Bedrock packet
#[derive(Debug, Clone)]
pub struct RawBedrockPacket {
    pub id: u32,
    pub data: Bytes,
    pub direction: PacketDirection,
}

impl RawBedrockPacket {
    /// Create a new raw Bedrock packet
    pub fn new(id: u32, data: Bytes, direction: PacketDirection) -> Self {
        Self { id, data, direction }
    }
    
    /// Get the total packet size (ID + data)
    pub fn size(&self) -> usize {
        // Bedrock packets include u32 ID + data
        4 + self.data.len()
    }
}

/// Bedrock packet registry
pub struct BedrockPacketRegistry {
    packets: HashMap<u32, String>,
}

impl BedrockPacketRegistry {
    /// Create a new Bedrock packet registry
    pub fn new() -> Self {
        Self {
            packets: HashMap::new(),
        }
    }
    
    /// Register a Bedrock packet
    pub fn register_packet<P>(&mut self, id: u32)
    where
        P: BedrockPacket + ConnectedPacket + 'static,
    {
        let type_name = std::any::type_name::<P>().to_string();
        self.packets.insert(id, type_name);
    }
    
    /// Check if a Bedrock packet is registered
    pub fn is_packet_registered(&self, id: u32) -> bool {
        self.packets.contains_key(&id)
    }
    
    /// Get registered packets
    pub fn packets(&self) -> &HashMap<u32, String> {
        &self.packets
    }
}

impl Default for BedrockPacketRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Bridge trait to convert between mirai's ConnectedPacket and BedrockPacket
pub trait BedrockPacketBridge: ConnectedPacket {
    /// Convert to raw packet
    fn to_raw(&self) -> Result<RawBedrockPacket, BedrockProtocolError>;
    
    /// Convert from raw packet
    fn from_raw(packet: RawBedrockPacket) -> Result<Self, BedrockProtocolError>
    where
        Self: Sized;
}

/// Implement BedrockPacket for types that implement BedrockPacketBridge
impl<T> BedrockPacket for T
where
    T: BedrockPacketBridge + Send + Sync + Sized,
{
    fn packet_id(&self) -> u32 {
        T::ID
    }
    
    fn serialize(&self) -> Result<Bytes, BedrockProtocolError> {
        let raw = self.to_raw()?;
        Ok(raw.data)
    }
    
    fn deserialize(data: Bytes) -> Result<Self, BedrockProtocolError> {
        let packet = RawBedrockPacket::new(T::ID, data, PacketDirection::Serverbound);
        T::from_raw(packet)
    }
    
    fn direction() -> PacketDirection {
        // Default to bidirectional for Bedrock packets
        // Individual packets can override this
        PacketDirection::Bidirectional
    }
}

/// Utility functions for Bedrock packet handling
pub mod utils {
    use super::*;
    
    /// Write a u32 packet ID to a buffer (little endian)
    pub fn write_packet_id(buf: &mut BytesMut, id: u32) {
        buf.put_u32_le(id);
    }
    
    /// Read a u32 packet ID from a buffer (little endian)
    pub fn read_packet_id(buf: &mut Bytes) -> Result<u32, BedrockProtocolError> {
        if buf.remaining() < 4 {
            return Err(BedrockProtocolError::BufferUnderflow {
                expected: 4,
                actual: buf.remaining(),
            });
        }
        
        Ok(buf.get_u32_le())
    }
}

/// Macro to implement BedrockPacket for Bedrock Edition packets
#[macro_export]
macro_rules! impl_bedrock_packet {
    ($packet_type:ty, $direction:expr) => {
        impl $crate::unified::BedrockPacketBridge for $packet_type {
            fn to_raw(&self) -> Result<$crate::unified::RawBedrockPacket, $crate::unified::BedrockProtocolError> {
                // This would need to be implemented based on mirai's serialization
                // For now, return a placeholder
                Ok($crate::unified::RawBedrockPacket::new(
                    Self::ID,
                    bytes::Bytes::new(),
                    $direction
                ))
            }
            
            fn from_raw(packet: $crate::unified::RawBedrockPacket) -> Result<Self, $crate::unified::BedrockProtocolError> {
                // This would need to be implemented based on mirai's deserialization
                // For now, return an error
                Err($crate::unified::BedrockProtocolError::UnsupportedOperation(
                    "Bedrock packet deserialization not yet implemented".to_string()
                ))
            }
        }
        
        impl $crate::unified::BedrockPacket for $packet_type {
            fn direction() -> $crate::unified::PacketDirection {
                $direction
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_raw_bedrock_packet() {
        let packet = RawBedrockPacket::new(0xfe, Bytes::from_static(b"test"), PacketDirection::Serverbound);
        
        assert_eq!(packet.id, 0xfe);
        assert_eq!(packet.data, Bytes::from_static(b"test"));
        
        // Bedrock packet size includes u32 ID + data
        assert_eq!(packet.size(), 8); // 4 bytes ID + 4 bytes data
    }
    
    #[test]
    fn test_packet_id_utils() {
        let mut buf = BytesMut::new();
        
        // Test packet ID read/write
        utils::write_packet_id(&mut buf, 0xfe);
        
        let mut read_buf = buf.freeze();
        let id = utils::read_packet_id(&mut read_buf).unwrap();
        
        assert_eq!(id, 0xfe);
    }
    
    #[test]
    fn test_packet_registry() {
        use crate::bedrock::ConnectedPacket;
        
        // Dummy packet for testing
        #[derive(Debug, Clone)]
        struct TestPacket;
        
        impl ConnectedPacket for TestPacket {
            const ID: u32 = 0xfe;
        }
        
        impl BedrockPacketBridge for TestPacket {
            fn to_raw(&self) -> Result<RawBedrockPacket, BedrockProtocolError> {
                Ok(RawBedrockPacket::new(0xfe, bytes::Bytes::new(), PacketDirection::Serverbound))
            }
            
            fn from_raw(_packet: RawBedrockPacket) -> Result<Self, BedrockProtocolError> {
                Ok(TestPacket)
            }
        }
        
        let mut registry = BedrockPacketRegistry::new();
        
        // Register some test packets
        registry.register_packet::<TestPacket>(0xfe);
        
        assert!(registry.is_packet_registered(0xfe));
        assert!(!registry.is_packet_registered(0xff));
    }
}