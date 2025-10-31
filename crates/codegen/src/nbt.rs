//! NBT (Named Binary Tag) parsing and code generation for Mirai

use crate::error::{CodegenError, Result};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use byteorder::{BigEndian, ReadBytesExt};

/// NBT tag types
#[derive(Debug, Clone, PartialEq)]
pub enum NbtTag {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<NbtTag>),
    Compound(HashMap<String, NbtTag>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

/// NBT parser for reading NBT data
pub struct NbtParser {
    cursor: Cursor<Vec<u8>>,
}

impl NbtParser {
    /// Create a new NBT parser from data
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            cursor: Cursor::new(data),
        }
    }
    
    /// Parse NBT data from bytes
    pub fn parse(data: &[u8]) -> Result<(String, NbtTag)> {
        if data.is_empty() {
            return Err(CodegenError::NbtError("Empty NBT data".to_string()));
        }
        
        let mut parser = Self::new(data.to_vec());
        parser.parse_named_tag()
    }
    
    /// Parse NBT from a file
    pub fn parse_file<P: AsRef<std::path::Path>>(path: P) -> Result<(String, NbtTag)> {
        let data = std::fs::read(path)?;
        
        // Check if data is gzip compressed
        let data = if utils::is_compressed(&data) {
            utils::decompress_gzip(&data)?
        } else {
            data
        };
        
        Self::parse(&data)
    }
    
    /// Parse a named NBT tag (tag type + name + payload)
    fn parse_named_tag(&mut self) -> Result<(String, NbtTag)> {
        let tag_type = self.cursor.read_u8()
            .map_err(|e| CodegenError::NbtError(format!("Failed to read tag type: {}", e)))?;
        
        if tag_type == 0 {
            return Ok(("".to_string(), NbtTag::End));
        }
        
        let name = self.read_string()?;
        let tag = self.parse_tag_payload(tag_type)?;
        
        Ok((name, tag))
    }
    
    /// Parse tag payload based on tag type
    fn parse_tag_payload(&mut self, tag_type: u8) -> Result<NbtTag> {
        match tag_type {
            0 => Ok(NbtTag::End),
            1 => Ok(NbtTag::Byte(self.cursor.read_i8()
                .map_err(|e| CodegenError::NbtError(format!("Failed to read byte: {}", e)))?)),
            2 => Ok(NbtTag::Short(self.cursor.read_i16::<BigEndian>()
                .map_err(|e| CodegenError::NbtError(format!("Failed to read short: {}", e)))?)),
            3 => Ok(NbtTag::Int(self.cursor.read_i32::<BigEndian>()
                .map_err(|e| CodegenError::NbtError(format!("Failed to read int: {}", e)))?)),
            4 => Ok(NbtTag::Long(self.cursor.read_i64::<BigEndian>()
                .map_err(|e| CodegenError::NbtError(format!("Failed to read long: {}", e)))?)),
            5 => Ok(NbtTag::Float(self.cursor.read_f32::<BigEndian>()
                .map_err(|e| CodegenError::NbtError(format!("Failed to read float: {}", e)))?)),
            6 => Ok(NbtTag::Double(self.cursor.read_f64::<BigEndian>()
                .map_err(|e| CodegenError::NbtError(format!("Failed to read double: {}", e)))?)),
            7 => self.read_byte_array(),
            8 => Ok(NbtTag::String(self.read_string()?)),
            9 => self.read_list(),
            10 => self.read_compound(),
            11 => self.read_int_array(),
            12 => self.read_long_array(),
            _ => Err(CodegenError::NbtError(format!("Unknown NBT tag type: {}", tag_type))),
        }
    }
    
    /// Read a string from the NBT data
    fn read_string(&mut self) -> Result<String> {
        let length = self.cursor.read_u16::<BigEndian>()
            .map_err(|e| CodegenError::NbtError(format!("Failed to read string length: {}", e)))?;
        
        let mut buffer = vec![0u8; length as usize];
        self.cursor.read_exact(&mut buffer)
            .map_err(|e| CodegenError::NbtError(format!("Failed to read string data: {}", e)))?;
        
        String::from_utf8(buffer)
            .map_err(|e| CodegenError::NbtError(format!("Invalid UTF-8 in string: {}", e)))
    }
    
    /// Read a byte array from the NBT data
    fn read_byte_array(&mut self) -> Result<NbtTag> {
        let length = self.cursor.read_i32::<BigEndian>()
            .map_err(|e| CodegenError::NbtError(format!("Failed to read byte array length: {}", e)))?;
        
        let mut buffer = vec![0i8; length as usize];
        for i in 0..length as usize {
            buffer[i] = self.cursor.read_i8()
                .map_err(|e| CodegenError::NbtError(format!("Failed to read byte array element: {}", e)))?;
        }
        
        Ok(NbtTag::ByteArray(buffer))
    }
    
    /// Read a list from the NBT data
    fn read_list(&mut self) -> Result<NbtTag> {
        let element_type = self.cursor.read_u8()
            .map_err(|e| CodegenError::NbtError(format!("Failed to read list element type: {}", e)))?;
        
        let length = self.cursor.read_i32::<BigEndian>()
            .map_err(|e| CodegenError::NbtError(format!("Failed to read list length: {}", e)))?;
        
        let mut elements = Vec::new();
        for _ in 0..length {
            let element = self.parse_tag_payload(element_type)?;
            elements.push(element);
        }
        
        Ok(NbtTag::List(elements))
    }
    
    /// Read a compound from the NBT data
    fn read_compound(&mut self) -> Result<NbtTag> {
        let mut compound = HashMap::new();
        
        loop {
            let tag_type = self.cursor.read_u8()
                .map_err(|e| CodegenError::NbtError(format!("Failed to read compound tag type: {}", e)))?;
            
            if tag_type == 0 {
                break; // End tag
            }
            
            let name = self.read_string()?;
            let tag = self.parse_tag_payload(tag_type)?;
            compound.insert(name, tag);
        }
        
        Ok(NbtTag::Compound(compound))
    }
    
    /// Read an int array from the NBT data
    fn read_int_array(&mut self) -> Result<NbtTag> {
        let length = self.cursor.read_i32::<BigEndian>()
            .map_err(|e| CodegenError::NbtError(format!("Failed to read int array length: {}", e)))?;
        
        let mut buffer = vec![0i32; length as usize];
        for i in 0..length as usize {
            buffer[i] = self.cursor.read_i32::<BigEndian>()
                .map_err(|e| CodegenError::NbtError(format!("Failed to read int array element: {}", e)))?;
        }
        
        Ok(NbtTag::IntArray(buffer))
    }
    
    /// Read a long array from the NBT data
    fn read_long_array(&mut self) -> Result<NbtTag> {
        let length = self.cursor.read_i32::<BigEndian>()
            .map_err(|e| CodegenError::NbtError(format!("Failed to read long array length: {}", e)))?;
        
        let mut buffer = vec![0i64; length as usize];
        for i in 0..length as usize {
            buffer[i] = self.cursor.read_i64::<BigEndian>()
                .map_err(|e| CodegenError::NbtError(format!("Failed to read long array element: {}", e)))?;
        }
        
        Ok(NbtTag::LongArray(buffer))
    }
}

/// NBT code generator for Mirai
pub struct NbtCodeGenerator;

impl NbtCodeGenerator {
    /// Generate Rust code from NBT structure
    pub fn generate_code(nbt: &NbtTag, struct_name: &str) -> Result<String> {
        match nbt {
            NbtTag::Compound(compound) => {
                Self::generate_compound_struct(compound, struct_name)
            }
            _ => Err(CodegenError::GenerationError(
                "Can only generate code from compound NBT tags".to_string()
            )),
        }
    }
    
    /// Generate a struct from a compound NBT tag
    fn generate_compound_struct(
        compound: &HashMap<String, NbtTag>,
        struct_name: &str,
    ) -> Result<String> {
        let mut code = String::new();
        
        code.push_str(&format!(
            "#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\n"
        ));
        code.push_str(&format!("pub struct {} {{\n", struct_name));
        
        for (name, tag) in compound {
            let field_name = Self::sanitize_field_name(name);
            let field_type = Self::nbt_tag_to_rust_type(tag);
            code.push_str(&format!("    pub {}: {},\n", field_name, field_type));
        }
        
        code.push_str("}\n");
        
        Ok(code)
    }
    
    /// Convert NBT tag to Rust type string
    fn nbt_tag_to_rust_type(tag: &NbtTag) -> String {
        match tag {
            NbtTag::End => "()".to_string(),
            NbtTag::Byte(_) => "i8".to_string(),
            NbtTag::Short(_) => "i16".to_string(),
            NbtTag::Int(_) => "i32".to_string(),
            NbtTag::Long(_) => "i64".to_string(),
            NbtTag::Float(_) => "f32".to_string(),
            NbtTag::Double(_) => "f64".to_string(),
            NbtTag::ByteArray(_) => "Vec<i8>".to_string(),
            NbtTag::String(_) => "String".to_string(),
            NbtTag::List(list) => {
                if let Some(first) = list.first() {
                    format!("Vec<{}>", Self::nbt_tag_to_rust_type(first))
                } else {
                    "Vec<()>".to_string()
                }
            }
            NbtTag::Compound(_) => "HashMap<String, NbtTag>".to_string(),
            NbtTag::IntArray(_) => "Vec<i32>".to_string(),
            NbtTag::LongArray(_) => "Vec<i64>".to_string(),
        }
    }
    
    /// Sanitize field names to be valid Rust identifiers
    fn sanitize_field_name(name: &str) -> String {
        let mut result = name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
            .collect::<String>();
        
        // Ensure it doesn't start with a number
        if result.chars().next().map_or(false, |c| c.is_numeric()) {
            result = format!("field_{}", result);
        }
        
        // Handle Rust keywords
        match result.as_str() {
            "type" => "r#type".to_string(),
            "match" => "r#match".to_string(),
            "use" => "r#use".to_string(),
            "mod" => "r#mod".to_string(),
            _ => result,
        }
    }
}

/// Utility functions for NBT handling
pub mod utils {
    use super::*;
    
    /// Check if NBT data is compressed (gzip)
    pub fn is_compressed(data: &[u8]) -> bool {
        data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b
    }
    
    /// Decompress gzip-compressed NBT data
    pub fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>> {
        use flate2::read::GzDecoder;
        use std::io::Read;
        
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| CodegenError::NbtError(format!("Failed to decompress gzip data: {}", e)))?;
        
        Ok(decompressed)
    }
    
    /// Get the NBT tag type ID
    pub fn get_tag_type_id(tag: &NbtTag) -> u8 {
        match tag {
            NbtTag::End => 0,
            NbtTag::Byte(_) => 1,
            NbtTag::Short(_) => 2,
            NbtTag::Int(_) => 3,
            NbtTag::Long(_) => 4,
            NbtTag::Float(_) => 5,
            NbtTag::Double(_) => 6,
            NbtTag::ByteArray(_) => 7,
            NbtTag::String(_) => 8,
            NbtTag::List(_) => 9,
            NbtTag::Compound(_) => 10,
            NbtTag::IntArray(_) => 11,
            NbtTag::LongArray(_) => 12,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sanitize_field_name() {
        assert_eq!(NbtCodeGenerator::sanitize_field_name("normal_name"), "normal_name");
        assert_eq!(NbtCodeGenerator::sanitize_field_name("with-dashes"), "with_dashes");
        assert_eq!(NbtCodeGenerator::sanitize_field_name("with spaces"), "with_spaces");
        assert_eq!(NbtCodeGenerator::sanitize_field_name("123numeric"), "field_123numeric");
        assert_eq!(NbtCodeGenerator::sanitize_field_name("type"), "r#type");
    }
    
    #[test]
    fn test_nbt_tag_to_rust_type() {
        assert_eq!(NbtCodeGenerator::nbt_tag_to_rust_type(&NbtTag::Byte(0)), "i8");
        assert_eq!(NbtCodeGenerator::nbt_tag_to_rust_type(&NbtTag::String("".to_string())), "String");
        assert_eq!(NbtCodeGenerator::nbt_tag_to_rust_type(&NbtTag::IntArray(vec![])), "Vec<i32>");
        
        let list = NbtTag::List(vec![NbtTag::Int(0)]);
        assert_eq!(NbtCodeGenerator::nbt_tag_to_rust_type(&list), "Vec<i32>");
    }
    
    #[test]
    fn test_get_tag_type_id() {
        assert_eq!(utils::get_tag_type_id(&NbtTag::End), 0);
        assert_eq!(utils::get_tag_type_id(&NbtTag::Byte(0)), 1);
        assert_eq!(utils::get_tag_type_id(&NbtTag::String("".to_string())), 8);
        assert_eq!(utils::get_tag_type_id(&NbtTag::Compound(HashMap::new())), 10);
    }
    
    #[test]
    fn test_is_compressed() {
        let gzip_header = vec![0x1f, 0x8b, 0x08, 0x00];
        assert!(utils::is_compressed(&gzip_header));
        
        let normal_data = vec![0x0a, 0x00, 0x00];
        assert!(!utils::is_compressed(&normal_data));
    }
}