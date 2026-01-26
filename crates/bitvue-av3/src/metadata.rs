//! Metadata OBU parsing for AV3.

use crate::bitreader::BitReader;
use crate::error::{Av3Error, Result};
use serde::{Deserialize, Serialize};

/// Metadata types for AV3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetadataType {
    /// Metadata type not specified
    Unspecified = 0,
    /// HDR cluminance information
    Hdrccl = 1,
    /// Tone mapping
    Tmcd = 2,
    /// Frame size
    Scalability = 3,
    /// Scene description
    Sdes = 4,
    /// Alpha channel
    Alpha = 5,
    /// High dynamic range
    Hdrll = 6,
}

/// Metadata OBU for AV3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataObu {
    /// Metadata type
    pub metadata_type: MetadataType,
    /// Metadata payload
    pub payload: Vec<u8>,
}

/// Parse metadata from OBU payload.
pub fn parse_metadata(data: &[u8]) -> Result<MetadataObu> {
    let mut reader = BitReader::new(data);

    let metadata_type_raw = reader.read_bits(8)?;
    let metadata_type = match metadata_type_raw {
        0 => MetadataType::Unspecified,
        1 => MetadataType::Hdrccl,
        2 => MetadataType::Tmcd,
        3 => MetadataType::Scalability,
        4 => MetadataType::Sdes,
        5 => MetadataType::Alpha,
        6 => MetadataType::Hdrll,
        _ => return Err(Av3Error::InvalidData("Unknown metadata type".to_string())),
    };

    Ok(MetadataObu {
        metadata_type,
        payload: data[reader.byte_pos()..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metadata_empty() {
        let data: &[u8] = &[0x00];
        let result = parse_metadata(data);
        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.metadata_type, MetadataType::Unspecified);
    }
}
